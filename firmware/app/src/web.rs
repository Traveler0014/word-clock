//! 内建 Web 服务：两种模式共用一套接口。
//!
//! * `Mode::Setup` —— 没有 WiFi 凭据（或连不上）时，起 SoftAP `WordClock-XXXXXX`，
//!   页面是两步向导：选网络 → 输密码（+ 可选管理口令）→ 设备重启。
//! * `Mode::Run` —— 已联网时在局域网 `http://<ip>/` 提供管理页：
//!   查看状态、改设置（亮度/色温/时区/管理口令）、更换网络、恢复出厂。
//!   运行模式**不提供扫描**（扫描会让 STA 跳信道，正在看的页面会断），只允许手填 SSID。
//!
//! 设计约束：
//! * HTTP handler 里**不能**直接操作 WiFi/NVS 写操作（esp_http_server 是单事件任务），
//!   handler 只改共享状态，真正的动作由主循环执行（见 `main.rs`）。
//! * 所有页面文案走 i18n（页面内 JS 字典，按 `navigator.language` 选中/英）。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use embedded_svc::http::{Headers, Method};
use embedded_svc::io::{Read, Write};
use esp_idf_svc::{
    http::server::{Configuration as HttpConfiguration, EspHttpServer},
    nvs::EspDefaultNvsPartition,
};
use log::{error, info, warn};

use crate::creds::{self, Creds};
use crate::leddrv::LedDriver;
use crate::settings::{self, Settings};
use crate::wifi::{self, Wifi};

static SETUP_PAGE: &str = include_str!("web_setup.html");
static ADMIN_PAGE: &str = include_str!("web_admin.html");
/// 两页共用的时区表（IANA↔POSIX），只存一份
static TZ_JS: &str = include_str!("web_tz.js");

/// setup 模式下扫描的最小间隔（避免连续点刷新）
const SCAN_COOLDOWN: Duration = Duration::from_secs(10);
/// 管理页会话有效期
const SESSION_TTL: Duration = Duration::from_secs(30 * 60);
/// 口令错误锁定
const LOCK_AFTER_FAILS: u32 = 5;
const LOCK_TIME: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Setup,
    Run,
}

#[derive(Clone)]
struct Net {
    ssid: String,
    rssi: i32,
    secured: bool,
}

#[derive(Clone)]
pub struct Shared {
    pub mode: Mode,
    /// setup：扫描结果
    nets: Vec<Net>,
    /// setup：idle | scanning | connecting | ok:<ip> | fail:<原因>
    pub state: String,
    /// setup：页面提交上来的凭据，等主循环取走
    pub pending: Option<Creds>,
    /// setup：页面请求刷新扫描
    pub scan_req: bool,
    /// 当前生效的设置；主循环发现 `settings_dirty` 后会落盘并重新应用
    pub settings: Settings,
    pub settings_dirty: bool,
    /// 运行模式：页面请求恢复出厂
    pub reset_req: bool,
    /// 运行模式：页面提交了新的 WiFi 凭据，等主循环写 NVS 后重启
    pub new_wifi: Option<Creds>,
    // 会话与限流
    sessions: Vec<(String, Instant)>,
    fails: u32,
    lock_until: Option<Instant>,
    // 运行模式展示用
    boot: Instant,
    pub ip: String,
    pub ssid: String,
    pub rssi: i32,
    /// setup：DNS 劫持线程是否已起
    dns_hijack: bool,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Setup
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            mode: Mode::Setup,
            nets: Vec::new(),
            state: String::new(),
            pending: None,
            scan_req: false,
            settings: Settings::default(),
            settings_dirty: false,
            reset_req: false,
            new_wifi: None,
            sessions: Vec::new(),
            fails: 0,
            lock_until: None,
            boot: Instant::now(),
            ip: String::new(),
            ssid: String::new(),
            rssi: 0,
            dns_hijack: false,
        }
    }
}

impl Shared {
    fn touch_session(&mut self, req_token: Option<&str>) -> bool {
        let now = Instant::now();
        self.sessions.retain(|(_, exp)| *exp > now);
        match req_token {
            Some(t) => self.sessions.iter().any(|(s, _)| s == t),
            None => false,
        }
    }

    fn new_session(&mut self) -> String {
        let tok = format!("{:08x}{:08x}", unsafe { esp_idf_svc::sys::esp_random() },
                          unsafe { esp_idf_svc::sys::esp_random() });
        self.sessions.push((tok.clone(), Instant::now() + SESSION_TTL));
        tok
    }

    fn json_status(&self) -> String {
        let nets: Vec<String> = self
            .nets
            .iter()
            .map(|n| {
                format!(
                    "{{\"ssid\":\"{}\",\"rssi\":{},\"secured\":{}}}",
                    json_escape(&n.ssid),
                    n.rssi,
                    n.secured
                )
            })
            .collect();
        format!(
            "{{\"mode\":\"{}\",\"state\":\"{}\",\"nets\":[{}]}}",
            if self.mode == Mode::Setup { "setup" } else { "run" },
            json_escape(&self.state),
            nets.join(",")
        )
    }

    fn json_info(&self) -> String {
        format!(
            "{{\"mode\":\"run\",\"ip\":\"{}\",\"ssid\":\"{}\",\"rssi\":{},\"uptime\":{},\"fw\":\"{}\",\"admin\":{}}}",
            json_escape(&self.ip),
            json_escape(&self.ssid),
            self.rssi,
            self.boot.elapsed().as_secs(),
            env!("CARGO_PKG_VERSION"),
            !self.settings.admin.is_empty()
        )
    }

    fn json_settings(&self) -> String {
        format!(
            "{{\"brightness\":{},\"color\":{},\"tz\":\"{}\",\"admin\":{}}}",
            self.settings.brightness,
            self.settings.color,
            json_escape(&self.settings.tz),
            !self.settings.admin.is_empty()
        )
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn parse_form(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            Some((percent_decode(k), percent_decode(it.next().unwrap_or(""))))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let hex = |b: u8| -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    };
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 3;
                }
                _ => {
                    out.push(bytes[i]);
                    i += 1;
                }
            },
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 强制门户（captive portal）：把所有 DNS A 查询都答成 AP 自己的地址。
///
/// 手机/平板连上热点后会探测 `captive.apple.com` / `connectivitycheck.gstatic.com` 等；
/// 只要这些域名被解析到本机、且 HTTP 侧一律 302 回配置页，系统就会自动弹出
/// "登录网络" 面板 —— 用户不需要手输 IP。
fn spawn_dns_hijack(ap_ip: [u8; 4]) {
    // 注意：ESP-IDF 的 pthread 默认栈只有 ~3KB（CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT），
    // std::net 的初始化（地址解析、socket 层）远超这个量 —— 必须显式给大栈。
    let spawned = std::thread::Builder::new()
        .name("dns-hijack".into())
        .stack_size(8192)
        .spawn(move || {
        let sock = match std::net::UdpSocket::bind("0.0.0.0:53") {
            Ok(s) => s,
            Err(e) => {
                warn!("setup: DNS 劫持起不来（{e}），强制门户不可用，可手动访问 http://{}.{}.{}.{}",
                      ap_ip[0], ap_ip[1], ap_ip[2], ap_ip[3]);
                return;
            }
        };
        info!("setup: DNS 劫持已就绪 (0.0.0.0:53 → {}.{}.{}.{})",
              ap_ip[0], ap_ip[1], ap_ip[2], ap_ip[3]);
        let mut buf = [0u8; 512];
        let mut seen = 0u32;
        loop {
            match sock.recv_from(&mut buf) {
                Ok((n, src)) => {
                    if seen < 40 {
                        seen += 1;
                        info!("setup: DNS 查询 #{} {} <- {}", seen, dns_qname(&buf[..n]), src);
                    }
                    if let Some(resp) = dns_reply(&buf[..n], ap_ip) {
                        let _ = sock.send_to(&resp, src);
                    }
                }
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    });
    if let Err(e) = spawned {
        warn!("setup: DNS 劫持线程启动失败: {e}");
    }
}

/// 自检：向自己的 :53 发一条真实查询，确认应答器能被 lwIP 路由到并正确回包。
/// 没有这一步，"DNS 是否真的工作"只能靠手机试探，排查成本很高。
fn self_test_dns(ip: &str) {
    let target = format!("{ip}:53");
    let Some(ip_octets) = parse_ipv4(ip).ok() else { return };
    // 构造 captive.apple.com 的 A 查询
    let mut q: Vec<u8> = vec![0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    for label in ["captive", "apple", "com"] {
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01]);

    std::thread::Builder::new()
        .name("dns-selftest".into())
        .stack_size(8192)
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(800));
            let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") else {
                warn!("setup: DNS 自检：无法创建 socket");
                return;
            };
            let _ = sock.set_read_timeout(Some(Duration::from_secs(2)));
            if let Err(e) = sock.send_to(&q, &target) {
                warn!("setup: DNS 自检：发送到 {target} 失败: {e}");
                return;
            }
            let mut buf = [0u8; 512];
            match sock.recv_from(&mut buf) {
                Ok((n, _)) => {
                    let ip_hit = n >= 16 && buf[n - 4..n] == ip_octets;
                    info!(
                        "setup: DNS 自检通过：{} 字节应答，A 记录={}.{}.{}.{}",
                        n, buf[n - 4], buf[n - 3], buf[n - 2], buf[n - 1]
                    );
                    if !ip_hit {
                        warn!("setup: DNS 自检：应答里的地址不是 AP IP");
                    }
                }
                Err(e) => warn!("setup: DNS 自检失败（2s 内无应答）: {e}"),
            }
        })
        .ok();
}

/// 取出 DNS 查询里的域名（仅用于日志）
fn dns_qname(query: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 12;
    while i < query.len() {
        let len = query[i] as usize;
        if len == 0 || len & 0xC0 != 0 {
            break;
        }
        if i + 1 + len > query.len() {
            break;
        }
        if !out.is_empty() {
            out.push('.');
        }
        out.push_str(&String::from_utf8_lossy(&query[i + 1..i + 1 + len]));
        i += 1 + len;
    }
    if out.is_empty() { "<unparsed>".to_string() } else { out }
}

/// 把一个 DNS 查询改写成"A 记录 = ip"的应答；解析失败返回 None
fn dns_reply(query: &[u8], ip: [u8; 4]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    // 跳过问题段（若干 label + 结束 0）拿到 QTYPE/QCLASS 末尾
    let mut i = 12;
    loop {
        if i >= query.len() {
            return None;
        }
        let len = query[i] as usize;
        if len == 0 {
            i += 1;
            break;
        }
        if len & 0xC0 != 0 {
            return None; // 压缩指针：不处理（查询里不该出现）
        }
        i += 1 + len;
    }
    let qend = i + 4;
    if qend > query.len() {
        return None;
    }
    let mut r = Vec::with_capacity(qend + 16);
    r.extend_from_slice(&query[..qend]);
    r[2] = 0x81; // QR=1, RD=1
    r[3] = 0x80; // RA=1, RCODE=0
    r[6] = 0x00;
    r[7] = 0x01; // ANCOUNT=1
    r[8] = 0;
    r[9] = 0;
    r[10] = 0;
    r[11] = 0;
    r.extend_from_slice(&[0xC0, 0x0C]); // NAME → 问题名
    r.extend_from_slice(&[0x00, 0x01]); // TYPE A
    r.extend_from_slice(&[0x00, 0x01]); // CLASS IN
    r.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL 60s
    r.extend_from_slice(&[0x00, 0x04]); // RDLENGTH
    r.extend_from_slice(&ip);
    Some(r)
}

fn cookie_token(cookie: Option<&str>) -> Option<String> {
    for part in cookie?.split(';') {
        let p = part.trim();
        if let Some(v) = p.strip_prefix("wcs=") {
            return Some(v.to_string());
        }
    }
    None
}

fn parse_ipv4(s: &str) -> Result<[u8; 4]> {
    let v: Vec<u8> = s
        .split('.')
        .filter_map(|p| p.parse::<u8>().ok())
        .collect();
    anyhow::ensure!(v.len() == 4, "IP 解析失败: {s}");
    Ok([v[0], v[1], v[2], v[3]])
}

fn set_state(shared: &Arc<Mutex<Shared>>, s: &str) {
    shared.lock().unwrap().state = s.to_string();
}

// ────────────────────────── 配网模式（AP） ──────────────────────────

/// 配网模式主流程（永不返回：成功写入并 `esp_restart()`，失败则继续等待重试）
pub fn run_setup(wifi: &mut Wifi, nvs: &EspDefaultNvsPartition, leds: &mut LedDriver) -> ! {
    let shared = Arc::new(Mutex::new(Shared {
        mode: Mode::Setup,
        state: "starting".into(),
        settings: settings::load(nvs),
        boot: Instant::now(),
        ..Default::default()
    }));

    let _server = loop {
        match start_server(&shared, nvs) {
            Ok(s) => break s,
            Err(e) => {
                error!("setup: HTTP 服务启动失败: {e:#}，5s 后重试");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    };

    loop {
        match wifi::start_ap(wifi) {
            Ok((ssid, ip)) => {
                info!("setup: 热点 \"{ssid}\" → http://{ip}");
                shared.lock().unwrap().ip = ip.clone();
                if let Ok(octets) = parse_ipv4(&ip) {
                    if !shared.lock().unwrap().dns_hijack {
                        spawn_dns_hijack(octets);
                        shared.lock().unwrap().dns_hijack = true;
                        self_test_dns(&ip);
                    }
                }
                set_state(&shared, "idle");
            }
            Err(e) => {
                error!("setup: 起 hotspot 失败: {e:#}，5s 后重试");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        }
        serve_setup(wifi, nvs, leds, &shared);
    }
}

fn serve_setup(
    wifi: &mut Wifi,
    nvs: &EspDefaultNvsPartition,
    leds: &mut LedDriver,
    shared: &Arc<Mutex<Shared>>,
) {
    let mut want_scan = true;
    let mut last_scan = Instant::now() - SCAN_COOLDOWN;
    let mut dots = 0usize;

    loop {
        // 1) 页面提交的凭据
        let pending = shared.lock().unwrap().pending.take();
        if let Some(creds) = pending {
            info!("setup: 收到凭据 ssid=\"{}\"，开始验证", creds.ssid);
            set_state(shared, "connecting");
            let _ = leds.solid((0, 0, 40));

            match creds.validate().and_then(|()| wifi::try_connect(wifi, &creds)) {
                Ok(ip) => {
                    info!("setup: 连接成功 ip={ip}");
                    // 设置项（时区/管理口令）在配网模式下也由 handler 写进内存，
                    // 这里必须显式落盘，否则重启就丢（早前遗漏过这一点）
                    let cfg = shared.lock().unwrap().settings.clone();
                    if let Err(e) = settings::save(nvs, &cfg) {
                        warn!("setup: 设置落盘失败: {e:#}");
                    }

                    match creds::save(nvs, &creds) {
                        Ok(()) => {
                            set_state(shared, &format!("ok:{ip}"));
                            let _ = leds.solid((0, 40, 0));
                            std::thread::sleep(Duration::from_millis(2500));
                            info!("setup: 重启进入运行模式");
                            unsafe { esp_idf_svc::sys::esp_restart() };
                        }
                        Err(e) => {
                            error!("setup: 写 NVS 失败: {e:#}");
                            set_state(shared, "fail:NVS");
                        }
                    }
                }
                Err(e) => {
                    error!("setup: 验证失败: {e:#}");
                    set_state(shared, &format!("fail:{e}"));
                }
            }
            for _ in 0..3 {
                let _ = leds.solid((40, 0, 0));
                std::thread::sleep(Duration::from_millis(180));
                let _ = leds.solid((0, 0, 0));
                std::thread::sleep(Duration::from_millis(180));
            }
            return;
        }

        // 2) 按需扫描（扫描会让 AP 短暂消失，所以不做周期扫描）
        {
            let mut g = shared.lock().unwrap();
            if g.scan_req {
                g.scan_req = false;
                want_scan = true;
            }
        }
        if want_scan && last_scan.elapsed() >= SCAN_COOLDOWN {
            want_scan = false;
            last_scan = Instant::now();
            if shared.lock().unwrap().state == "idle" {
                set_state(shared, "scanning");
            }
            match wifi::scan(wifi) {
                Ok(list) => {
                    let mut nets: Vec<Net> = list
                        .into_iter()
                        .filter(|ap| !ap.ssid.is_empty())
                        .map(|ap| Net {
                            ssid: ap.ssid.to_string(),
                            rssi: ap.signal_strength as i32,
                            secured: ap.auth_method.is_some(),
                        })
                        .collect();
                    nets.sort_by(|a, b| b.rssi.cmp(&a.rssi));
                    nets.dedup_by(|a, b| a.ssid == b.ssid);
                    info!("setup: 扫描到 {} 个网络", nets.len());
                    let mut g = shared.lock().unwrap();
                    g.nets = nets;
                    if g.state == "scanning" {
                        g.state = "idle".into();
                    }
                }
                Err(e) => {
                    warn!("setup: 扫描失败: {e:#}");
                    set_state(shared, "idle");
                }
            }
        }

        // 3) 角点转圈
        let _ = leds.spinner(dots);
        dots = (dots + 1) % 4;
        std::thread::sleep(Duration::from_millis(400));
    }
}

// ────────────────────────── 运行模式（LAN） ──────────────────────────

/// 运行模式：起局域网管理页。返回的 server 必须保持存活（主循环持有）。
pub fn run_lan(
    nvs: &EspDefaultNvsPartition,
    ip: String,
    ssid: String,
) -> (Option<EspHttpServer<'static>>, Arc<Mutex<Shared>>) {
    let shared = Arc::new(Mutex::new(Shared {
        mode: Mode::Run,
        state: "run".into(),
        settings: settings::load(nvs),
        boot: Instant::now(),
        ip,
        ssid,
        ..Default::default()
    }));
    // 管理页起不来不应让时钟停摆：记日志后继续走时（shared 仍然可用）
    let server = match start_server(&shared, nvs) {
        Ok(s) => {
            info!("run: 管理页 http://{}/", shared.lock().unwrap().ip);
            Some(s)
        }
        Err(e) => {
            error!("run: 管理页启动失败: {e:#}（时钟继续运行）");
            None
        }
    };
    (server, shared)
}

// ────────────────────────── HTTP 路由 ──────────────────────────

fn start_server(
    shared: &Arc<Mutex<Shared>>,
    nvs: &EspDefaultNvsPartition,
) -> Result<EspHttpServer<'static>> {
    let mode = shared.lock().unwrap().mode;
    let mut server = EspHttpServer::new(&HttpConfiguration {
        stack_size: 12288,
        // 强制门户：任何未注册的 GET 都要能兜住做 302
        uri_match_wildcard: true,
        ..Default::default()
    })?;

    // 首页
    let page = if mode == Mode::Setup { SETUP_PAGE } else { ADMIN_PAGE };
    server.fn_handler::<anyhow::Error, _>("/", Method::Get, move |req| {
        let mut r = req.into_ok_response()?;
        r.write_all(page.as_bytes())?;
        Ok(())
    })?;

    // 时区表（两页共用）
    server.fn_handler::<anyhow::Error, _>("/tz.js", Method::Get, |req| {
        let mut r = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/javascript; charset=utf-8"),
              ("Cache-Control", "max-age=3600")],
        )?;
        r.write_all(TZ_JS.as_bytes())?;
        Ok(())
    })?;

    // 状态（两种模式都有）
    let s = shared.clone();
    server.fn_handler::<anyhow::Error, _>("/api/status", Method::Get, move |req| {
        let body = s.lock().unwrap().json_status();
        let mut r = req.into_response(200, Some("OK"),
            &[("Content-Type", "application/json; charset=utf-8")])?;
        r.write_all(body.as_bytes())?;
        Ok(())
    })?;

    if mode == Mode::Setup {
        // 扫描（仅配网模式）
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/scan", Method::Post, move |req| {
            {
                let mut g = s.lock().unwrap();
                g.scan_req = true;
                g.state = "scanning".into();
            }
            let mut r = req.into_ok_response()?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;

        // 提交凭据（仅配网模式，无鉴权：此时还没有管理口令）
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/save", Method::Post, move |mut req| {
            let len = req.content_len().unwrap_or(0) as usize;
            if len == 0 || len > 512 {
                let mut r = req.into_status_response(400)?;
                r.write_all(b"bad length")?;
                return Ok(());
            }
            let mut buf = vec![0u8; len];
            req.read_exact(&mut buf)?;
            let form = parse_form(&String::from_utf8_lossy(&buf));
            let get = |k: &str| {
                form.iter()
                    .find(|(kk, _)| kk == k)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            };
            let creds = Creds::new(get("ssid"), get("pass"));
            if let Err(e) = creds.validate() {
                let msg = format!("{e}");
                let mut r = req.into_status_response(400)?;
                r.write_all(msg.as_bytes())?;
                return Ok(());
            }
            info!("setup: 页面提交 ssid=\"{}\"", creds.ssid);
            {
                let mut g = s.lock().unwrap();
                g.pending = Some(creds.clone());
                g.state = "connecting".into();
                // 配网时可选设置管理口令与时区（时区由页面按手机自动识别）
                let admin = get("admin");
                if !admin.is_empty() {
                    g.settings.admin = admin;
                    g.settings_dirty = true;
                }
                let tz = get("tz");
                if !tz.is_empty() {
                    info!("setup: 页面自动识别的时区 = {tz}");
                    g.settings.tz = tz;
                    g.settings_dirty = true;
                }
            }
            let mut r = req.into_ok_response()?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;

        // 配网期间也允许把口令落盘（主循环会处理 settings_dirty）
        let s = shared.clone();
        let nvs_part = nvs.clone();
        server.fn_handler::<anyhow::Error, _>("/api/persist", Method::Post, move |req| {
            let cfg = s.lock().unwrap().settings.clone();
            let _ = settings::save(&nvs_part, &cfg);
            let mut r = req.into_ok_response()?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;

        // 强制门户兜底：系统探测请求（captive.apple.com/hotspot-detect.html、
        // connectivitycheck.gstatic.com/generate_204、msftconnecttest.com…）都会落到这里。
        // 一律 302 回配置页 —— 这是 iOS/Android 判定"该开登录页了"的信号。
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/*", Method::Get, move |req| {
            info!(
                "setup: 门户兜底 {} host={}",
                req.uri(),
                req.header("Host").unwrap_or("-")
            );
            let ip = {
                let g = s.lock().unwrap();
                if g.ip.is_empty() { "192.168.9.20".to_string() } else { g.ip.clone() }
            };
            let target = format!("http://{ip}/");
            let mut r = req.into_response(302, Some("Found"), &[("Location", target.as_str())])?;
            r.write_all(b"")?;
            Ok(())
        })?;
    } else {
        // 登录
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/login", Method::Post, move |mut req| {
            let len = req.content_len().unwrap_or(0) as usize;
            let mut buf = vec![0u8; len.min(256)];
            if len > 0 {
                req.read_exact(&mut buf)?;
            }
            let form = parse_form(&String::from_utf8_lossy(&buf));
            let pass = form
                .iter()
                .find(|(k, _)| k == "pass")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();

            let (ok, locked) = {
                let mut g = s.lock().unwrap();
                if let Some(t) = g.lock_until {
                    if t > Instant::now() {
                        (false, true)
                    } else {
                        g.lock_until = None;
                        g.fails = 0;
                        (pass == g.settings.admin, false)
                    }
                } else if g.settings.admin.is_empty() {
                    (true, false) // 未设置口令 = 不鉴权
                } else {
                    let ok = pass == g.settings.admin;
                    if !ok {
                        g.fails += 1;
                        if g.fails >= LOCK_AFTER_FAILS {
                            g.lock_until = Some(Instant::now() + LOCK_TIME);
                            g.fails = 0;
                        }
                    }
                    (ok, false)
                }
            };
            if locked {
                let mut r = req.into_response(429, Some("Too Many Requests"), &[])?;
                r.write_all(b"locked")?;
                return Ok(());
            }
            if !ok {
                let mut r = req.into_status_response(401)?;
                r.write_all(b"bad password")?;
                return Ok(());
            }
            let token = s.lock().unwrap().new_session();
            let cookie = format!("wcs={token}; Path=/; HttpOnly; Max-Age=1800");
            let mut r = req.into_response(
                200,
                Some("OK"),
                &[("Set-Cookie", cookie.as_str()),
                  ("Content-Type", "application/json; charset=utf-8")],
            )?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;

        // 信息
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/info", Method::Get, move |req| {
            let body = {
                let mut g = s.lock().unwrap();
                let authed = g.settings.admin.is_empty()
                    || g.touch_session(cookie_token(req.header("Cookie")).as_deref());
                let mut j = g.json_info();
                // 在末尾补 authed 字段（避免再拼一次结构）
                j.pop();
                format!("{j},\"admin_ok\":{authed}}}")
            };
            let mut r = req.into_response(200, Some("OK"),
                &[("Content-Type", "application/json; charset=utf-8")])?;
            r.write_all(body.as_bytes())?;
            Ok(())
        })?;

        // 读设置（需鉴权）
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/settings", Method::Get, move |req| {
            let auth = {
                let mut g = s.lock().unwrap();
                g.settings.admin.is_empty()
                    || g.touch_session(cookie_token(req.header("Cookie")).as_deref())
            };
            if !auth {
                let mut r = req.into_status_response(401)?;
                r.write_all(b"unauthorized")?;
                return Ok(());
            }
            let body = s.lock().unwrap().json_settings();
            let mut r = req.into_response(200, Some("OK"),
                &[("Content-Type", "application/json; charset=utf-8")])?;
            r.write_all(body.as_bytes())?;
            Ok(())
        })?;

        // 改设置（需鉴权）
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/settings", Method::Post, move |mut req| {
            let auth = {
                let mut g = s.lock().unwrap();
                g.settings.admin.is_empty()
                    || g.touch_session(cookie_token(req.header("Cookie")).as_deref())
            };
            if !auth {
                let mut r = req.into_status_response(401)?;
                r.write_all(b"unauthorized")?;
                return Ok(());
            }
            let len = req.content_len().unwrap_or(0) as usize;
            if len == 0 || len > 512 {
                let mut r = req.into_status_response(400)?;
                r.write_all(b"bad length")?;
                return Ok(());
            }
            let mut buf = vec![0u8; len];
            req.read_exact(&mut buf)?;
            let form = parse_form(&String::from_utf8_lossy(&buf));
            let get = |k: &str| {
                form.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.clone())
            };
            let mut g = s.lock().unwrap();
            if let Some(v) = get("brightness").and_then(|v| v.parse::<u8>().ok()) {
                g.settings.brightness = v;
            }
            if let Some(v) = get("color").and_then(|v| v.parse::<u8>().ok()) {
                g.settings.color = v;
            }
            if let Some(v) = get("tz") {
                if !v.is_empty() {
                    g.settings.tz = v;
                }
            }
            if let Some(v) = get("admin") {
                g.settings.admin = v; // 空串 = 取消鉴权
            }
            g.settings.sanitize();
            g.settings_dirty = true;
            info!("run: 设置已更新（待主循环落盘）");
            drop(g);
            let mut r = req.into_ok_response()?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;

        // 更换网络（需鉴权）：手填 SSID，不接受扫描
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/wifi", Method::Post, move |mut req| {
            let auth = {
                let mut g = s.lock().unwrap();
                g.settings.admin.is_empty()
                    || g.touch_session(cookie_token(req.header("Cookie")).as_deref())
            };
            if !auth {
                let mut r = req.into_status_response(401)?;
                r.write_all(b"unauthorized")?;
                return Ok(());
            }
            let len = req.content_len().unwrap_or(0) as usize;
            if len == 0 || len > 512 {
                let mut r = req.into_status_response(400)?;
                r.write_all(b"bad length")?;
                return Ok(());
            }
            let mut buf = vec![0u8; len];
            req.read_exact(&mut buf)?;
            let form = parse_form(&String::from_utf8_lossy(&buf));
            let get = |k: &str| {
                form.iter()
                    .find(|(kk, _)| kk == k)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            };
            let creds = Creds::new(get("ssid"), get("pass"));
            if let Err(e) = creds.validate() {
                let msg = format!("{e}");
                let mut r = req.into_status_response(400)?;
                r.write_all(msg.as_bytes())?;
                return Ok(());
            }
            s.lock().unwrap().new_wifi = Some(creds);
            let mut r = req.into_ok_response()?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;

        // 恢复出厂（需鉴权）
        let s = shared.clone();
        server.fn_handler::<anyhow::Error, _>("/api/reset", Method::Post, move |req| {
            let auth = {
                let mut g = s.lock().unwrap();
                g.settings.admin.is_empty()
                    || g.touch_session(cookie_token(req.header("Cookie")).as_deref())
            };
            if !auth {
                let mut r = req.into_status_response(401)?;
                r.write_all(b"unauthorized")?;
                return Ok(());
            }
            s.lock().unwrap().reset_req = true;
            let mut r = req.into_ok_response()?;
            r.write_all(b"{\"ok\":true}")?;
            Ok(())
        })?;
    }

    Ok(server)
}
