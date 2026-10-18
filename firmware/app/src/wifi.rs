//! WiFi 连接原语。
//!
//! 拆成几个可复用步骤，供两条路径共用：
//! * 正常启动：读 NVS 凭据 → `connect_or_retry()` → 进时钟循环
//! * 配网模式：`start_ap()` 起热点 + HTTP 服务 → 收到凭据后 `try_connect()` 验证
//!
//! 设计约束（实测踩坑详见 docs/development.md）：
//! - `auth_method` 必须显式设置：`ClientConfiguration::default()` 是 `None`，
//!   密码再对也连不上 WPA2 AP（症状为 `ESP_ERR_TIMEOUT`）。
//! - 连接失败只记日志并重试，绝不让 main 退出
//!   （app 返回 → IDF 重启整机 → 自检循环闪）。
//! - 每轮连接前先扫描确认 SSID 可见，避免凭据/AP 错误导致的盲目重试。

use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    wifi::{
        AccessPointConfiguration, AccessPointInfo, AuthMethod, BlockingWifi, ClientConfiguration,
        Configuration, EspWifi, WifiDeviceId,
    },
};
use log::{error, info, warn};

use crate::creds::Creds;

/// 项目里统一用的 WiFi 句柄类型（BlockingWifi + ESP-IDF WiFi）
pub type Wifi = BlockingWifi<EspWifi<'static>>;

/// 配网热点的固定地址（.20 是给用户留的"记住这个地址"的锚点）。
///
/// 库默认是 192.168.71.1（`embedded_svc::ipv4::RouterConfiguration::default()`），
/// 纯属库作者的随意取值；192.168.9.0/24 与常见家用网段（1.x / 0.x / 31.x）冲突更少。
const AP_IP: [u8; 4] = [192, 168, 9, 20];
/// 必须与 `AP_IP` 一致；DHCP option 114 的值，也必须活得够久（'static）
const CAPTIVE_URI: &str = "http://192.168.9.20/";

/// 把 AP 的网关地址改掉：必须先停 DHCP 服务器，改完再开，否则租约/网关不一致。
fn set_ap_ip(wifi: &Wifi) -> Result<String> {
    unsafe {
        use esp_idf_svc::sys as sys;
        // 注意：esp_ip4_addr_t.addr 是**网络字节序**（按内存字节摆放）。
        // 直接用 (a<<24)|(b<<16)|... 在 LE 芯片上会得到反序地址 —— 之前就因此把
        // 掩码写成非法值，dhcps 起不来并在 esp-idf-svc 里 unwrap panic。
        let oct = |a: u8, b: u8, c: u8, d: u8| -> u32 { u32::from_ne_bytes([a, b, c, d]) };
        // EspNetif 实现了 RawHandle，用 handle() 取裸指针
        use esp_idf_svc::handle::RawHandle;
        let handle = wifi.wifi().ap_netif().handle();
        let addr = sys::esp_ip4_addr_t { addr: oct(AP_IP[0], AP_IP[1], AP_IP[2], AP_IP[3]) };
        let mask = sys::esp_ip4_addr_t { addr: oct(255, 255, 255, 0) };
        let info = sys::esp_netif_ip_info_t { ip: addr, netmask: mask, gw: addr };
        sys::esp_netif_dhcps_stop(handle);
        let rc = sys::esp_netif_set_ip_info(handle, &info);

        // RFC 8910 Captive Portal API（DHCP option 114）：iOS 14+ / Android 11+ 会主动
        // 读这个 URI 判断"该弹登录页了"，比猜各家探测 URL 更可靠。
        // 注意：IDF 只保存指针，字符串必须活满 DHCP 服务器生命周期 → 用 'static str。
        let rc_opt = sys::esp_netif_dhcps_option(
            handle,
            sys::esp_netif_dhcp_option_mode_t_ESP_NETIF_OP_SET,
            sys::esp_netif_dhcp_option_id_t_ESP_NETIF_CAPTIVEPORTAL_URI,
            CAPTIVE_URI.as_ptr() as *mut core::ffi::c_void,
            CAPTIVE_URI.len() as u32,
        );
        if rc_opt != 0 {
            warn!("wifi: 设置 DHCP option 114 失败 rc={rc_opt}");
        }

        // 把 AP netif 的 DNS 指到自己 —— 这是让 DHCP option 6 广播我们地址的正确入口。
        // （注意：`esp_netif_dhcps_option(DOMAIN_NAME_SERVER)` 要的是**单字节开关**，
        //   不是 4 字节 IP；早前按 IP 传会返回 ESP_ERR_INVALID_PARAMS。）
        let mut dns_info: sys::esp_netif_dns_info_t = core::mem::zeroed();
        dns_info.ip.u_addr.ip4 = addr;
        let rc_dns = sys::esp_netif_set_dns_info(
            handle,
            sys::esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN,
            &mut dns_info,
        );
        if rc_dns != 0 {
            warn!("wifi: 设置 AP DNS 失败 rc={rc_dns}");
        } else {
            info!("wifi: DHCP 将广播 DNS = {}.{}.{}.{}", AP_IP[0], AP_IP[1], AP_IP[2], AP_IP[3]);
        }

        sys::esp_netif_dhcps_start(handle);
        if rc != 0 {
            anyhow::bail!("esp_netif_set_ip_info 失败 rc={rc}");
        }
    }
    let ip = wifi.wifi().ap_netif().get_ip_info()?;
    Ok(format!("{}", ip.ip))
}

/// 配网热点的 SSID 后缀取 STA MAC 末 3 字节，方便用户区分多台设备
pub fn ap_ssid(wifi: &Wifi) -> String {
    match wifi.wifi().get_mac(WifiDeviceId::Sta) {
        Ok(mac) => format!(
            "WordClock-{:02X}{:02X}{:02X}",
            mac[3], mac[4], mac[5]
        ),
        Err(_) => "WordClock-SETUP".to_string(),
    }
}

pub fn create(
    modem: Modem<'static>,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<Wifi> {
    info!("wifi: creating EspWifi...");
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
    info!("wifi: EspWifi created");
    Ok(BlockingWifi::wrap(esp_wifi, sysloop)?)
}

fn client_config(creds: &Creds) -> Result<ClientConfiguration> {
    Ok(ClientConfiguration {
        ssid: creds
            .ssid
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("SSID 超过 32 字节"))?,
        password: creds
            .pass
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("密码超过 64 字节"))?,
        // Default 的 auth_method 是 None，密码再对也连不上 WPA2 AP（实测 ESP_ERR_TIMEOUT）
        auth_method: if creds.pass.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPAWPA2Personal
        },
        ..Default::default()
    })
}

/// 起配网热点（混合模式：AP 提供网页，STA 用于扫描附近网络）
pub fn start_ap(wifi: &mut Wifi) -> Result<(String, String)> {
    let ssid = ap_ssid(wifi);
    let ap_conf = AccessPointConfiguration {
        ssid: ssid
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("AP SSID 构造失败"))?,
        ssid_hidden: false,
        channel: 6,
        // 开放的配网热点：省掉"用户还要先猜热点密码"这一步
        auth_method: AuthMethod::None,
        password: "".try_into().unwrap(),
        max_connections: 4,
        ..Default::default()
    };

    if wifi.is_started().unwrap_or(false) {
        let _ = wifi.stop();
    }
    // Mixed：STA 侧留空（仅用于扫描），AP 侧提供服务
    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration::default(),
        ap_conf,
    ))?;
    wifi.start()?;
    // AP 模式下 AP netif 立即可用（STA 侧没有上游，别等 wait_netif_up）
    let ip = match set_ap_ip(wifi) {
        Ok(ip) => ip,
        Err(e) => {
            warn!("wifi: 设置 AP 地址失败（用库默认）: {e:#}");
            match wifi.wifi().ap_netif().get_ip_info() {
                Ok(info) => format!("{}", info.ip),
                Err(_) => "192.168.71.1".to_string(),
            }
        }
    };
    info!("provision: 热点已开启 ssid=\"{ssid}\" → http://{ip}");
    Ok((ssid, ip))
}

/// 扫描附近 AP（配网页面用）。
///
/// 注意：`scan_n::<N>()` 的 `heapless::Vec<AccessPointInfo, N>` 是**放栈上**的，
/// N 大了会直接把主任务栈打爆（实测 N=20 → Stack protection fault）。
/// 8 个足够配网页面用，更多没意义（用户只会选自己的网络）。
pub fn scan(wifi: &mut Wifi) -> Result<Vec<AccessPointInfo>> {
    // APSTA 模式下扫描期间 AP 必须离开自己的信道 → beacon 中断 → 手机"看不到热点"。
    // 所以：① 只在页面要求时扫（不做周期扫描）；② 驻留时间调到 60~90ms（默认 ~120ms/信道），
    // 把整个扫描压到 1s 左右，尽量缩短 AP 消失的时间窗。
    let cfg = esp_idf_svc::wifi::config::ScanConfig {
        scan_type: esp_idf_svc::wifi::config::ScanType::Active {
            min: Duration::from_millis(60),
            max: Duration::from_millis(90),
        },
        show_hidden: false,
        ..Default::default()
    };
    let mut list = Vec::new();
    {
        let driver = wifi.wifi_mut();
        driver.start_scan(&cfg, true)?;
        let (found, _) = driver.get_scan_result_n::<8>()?;
        list.extend(found);
    }
    Ok(list)
}

/// 单次连接尝试（用于验证用户刚填的凭据）。成功返回 IP。
pub fn try_connect(wifi: &mut Wifi, creds: &Creds) -> Result<String> {
    creds.validate()?;
    if wifi.is_started().unwrap_or(false) {
        let _ = wifi.stop();
    }
    wifi.set_configuration(&Configuration::Client(client_config(creds)?))?;
    wifi.start()?;
    wifi.connect()
        .map_err(|e| anyhow::anyhow!("assoc 失败: {e:?}"))?;
    // 内部超时 15s（esp-idf-svc CONNECT_TIMEOUT）
    wifi.wait_netif_up()
        .map_err(|e| anyhow::anyhow!("拿不到 IP: {e:?}"))?;
    let ip = wifi.wifi().sta_netif().get_ip_info()?;
    Ok(format!("{:?}", ip.ip))
}

/// 正常启动路径：有限次尝试；全部失败则返回 Err，由调用方决定是否进配网模式。
pub fn connect_with_retries(
    wifi: &mut Wifi,
    creds: &Creds,
    attempts: usize,
) -> Result<String> {
    let mut last_err = None;
    for n in 1..=attempts {
        info!(
            "wifi: 尝试连接 \"{}\" ({n}/{attempts})",
            creds.ssid
        );
        match try_connect(wifi, creds) {
            Ok(ip) => {
                info!("wifi up, ip = {ip}");
                return Ok(ip);
            }
            Err(e) => {
                warn!("wifi: 第 {n} 次连接失败: {e:#}");
                last_err = Some(e);
                let _ = wifi.disconnect();
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("连接失败")))
}

/// 时钟运行期间的保活检查（非阻塞，由显示循环周期调用）：
/// 已连接就什么都不做；掉线则尝试重连一次。绝不返回 Err 让 main 退出。
pub fn ensure_connected(wifi: &mut Wifi, creds: &Creds) -> bool {
    if wifi.is_connected().unwrap_or(false) {
        return true;
    }
    error!("wifi: 连接已断开，重连中...");
    match try_connect(wifi, creds) {
        Ok(ip) => {
            info!("wifi: 重连成功 ip={ip}");
            true
        }
        Err(e) => {
            warn!("wifi: 重连失败: {e:#}");
            false
        }
    }
}
