//! 字钟固件入口。
//!
//! 启动流程：RMT 自检（RGB 全链扫描）→ 10s 控制台接入窗口 → WiFi STA →
//! SNTP 对时（UTC+8）→ 每分钟整帧刷新显示。
//!
//! 设计约束（勿违反，实测踩坑详见 docs/development.md）：
//! - 任何阶段失败都必须重试而非返回 Err：app 一旦返回，IDF 会重启整机，
//!   现象是周期性自检循环闪。
//! - 启动 10s 延迟：USB 重枚举后主机有 ~6s 空窗，延迟保证串口监听能接上
//!   抓到全量日志（量产前移除）。
//! - Rust `log` 输出只进主控制台（UART0）；sdkconfig 见 app/sdkconfig.defaults。

mod creds;
mod leddrv;
mod settings;
mod time_svc;
mod web;
mod wifi;

use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::hal::peripherals::Peripherals;
use log::{error, info, warn};

/// 静态标尺测试图案（--features diag 专用，永远不返回）：
/// * 每行最左一颗（col 0）红、最右一颗（col 10）绿、中间（col 5）暗蓝
/// * 四角点 110~113 白
/// * 每 2s 重发同一图案 —— 图案若自己变/闪，就是信号仍在丢位
#[cfg(feature = "diag")]
fn diag_pattern(leds: &mut leddrv::LedDriver) -> ! {
    use wordclock_core::{xy_to_index, COLS, LETTER_LEDS, ROWS, TOTAL_LEDS};

    let mut rgb = [(0u8, 0u8, 0u8); TOTAL_LEDS];
    for r in 0..ROWS {
        rgb[xy_to_index(r, 0)] = (32, 0, 0); // 行首：红
        rgb[xy_to_index(r, COLS - 1)] = (0, 32, 0); // 行尾：绿
        rgb[xy_to_index(r, COLS / 2)] = (0, 0, 12); // 行中：暗蓝（参考点）
    }
    for px in rgb.iter_mut().skip(LETTER_LEDS) {
        *px = (16, 16, 16); // 四角：白
    }

    info!("diag: 标尺图案 — 每行左端红 / 右端绿 / 中间暗蓝，四角白；每 2s 重发");
    loop {
        if let Err(e) = leds.show_rgb(&rgb) {
            error!("diag: 写灯失败: {e:#}");
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

#[cfg_attr(feature = "diag", allow(unreachable_code))] // diag 分支永远不返回
fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("word-clock boot");

    let peripherals = Peripherals::take()?;

    // LED 初始化 + 上电自检（RGB 全扫一遍，验证灯链与供电后再进网络流程）
    #[allow(deprecated)] // legacy rmt api，同 leddrv.rs 注释
    let mut leds = leddrv::LedDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio4)?;
    leds.self_test()?;

    // 调试固件（--features diag）：不连 WiFi，只显示静态标尺，用于验证
    // 灯链映射 / 信号完整性是否修好。
    #[cfg(feature = "diag")]
    diag_pattern(&mut leds);

    // USB 重枚举后主机有 ~6s 空窗：停 10s 保证串口监听接上后再打 WiFi 日志
    info!("boot: waiting 10s for console attach...");
    std::thread::sleep(Duration::from_secs(10));

    let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take()?;
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take()?;
    let mut wifi = wifi::create(peripherals.modem, sysloop, nvs.clone())?;

    // ── 恢复出厂手势 ────────────────────────────────────────────────
    // 无按键设备的通用做法：连续 3 次"没跑到健康状态"就认为用户想强制重新配网。
    // 健康状态在 WiFi + NTP 成功后由 clear_boot_streak() 确认（见下方）。
    let streak = creds::bump_boot_streak(&nvs);
    if streak >= 3 {
        warn!("检测到连续 {streak} 次未正常启动 → 恢复出厂：清除 WiFi 凭据");
        if let Err(e) = creds::clear(&nvs) {
            error!("creds: 清除失败: {e:#}");
        }
        creds::clear_boot_streak(&nvs);
        // 视觉反馈：整屏快闪 3 下（黑白，不用颜色也能看懂）
        for _ in 0..3 {
            let _ = leds.solid((48, 48, 48));
            std::thread::sleep(Duration::from_millis(120));
            let _ = leds.solid((0, 0, 0));
            std::thread::sleep(Duration::from_millis(120));
        }
    } else {
        info!("boot: 未正常启动计数 = {streak}/3（连续 3 次将恢复出厂）");
    }

    // 凭据只来自 NVS（配网页面写入）。固件本身不含任何 WiFi 密码。
    let stored = creds::load(&nvs);

    // 有凭据：有限次重试（~2 分钟）；失败则进配网模式，而不是无限重试
    let active_creds = match stored {
        Some(c) => match wifi::connect_with_retries(&mut wifi, &c, 8) {
            Ok(ip) => {
                info!("wifi ready, ip = {ip}");
                Some((c, ip))
            }
            Err(e) => {
                warn!("wifi: 连不上已保存的网络（{e:#}），进入配网模式");
                None
            }
        },
        None => {
            info!("creds: 无凭据，进入配网模式");
            None
        }
    };

    let (active_creds, ip) = match active_creds {
        Some(v) => v,
        // 进入配网模式说明用户此刻就想配网，计数清零
        // 配网模式永不返回：成功后 esp_restart()，失败则继续等待
        None => {
            creds::clear_boot_streak(&nvs);
            web::run_setup(&mut wifi, &nvs, &mut leds)
        }
    };

    // SNTP 对时（UTC+8），内部轮询等待同步完成
    let sntp = loop {
        match time_svc::start_sntp() {
            Ok(s) => break s,
            Err(e) => {
                error!("sntp start failed: {e:#}, retry in 10s");
                std::thread::sleep(Duration::from_secs(10));
            }
        }
    };
    time_svc::wait_synced(&sntp);

    // 已到健康状态（WiFi + NTP 都通了）→ 清零未正常启动计数
    creds::clear_boot_streak(&nvs);

    // ── 运行模式：起局域网管理页 ──────────────────────────────────
    // server 必须活到进程结束，这里是它的唯一持有者。
    // 管理页起不来时内部已记录日志并返回 None，时钟照常运行
    let (_lan_server, shared) = web::run_lan(&nvs, ip.clone(), active_creds.ssid.clone());

    // 首屏就有信号强度可看（之后由 30s 周期的保活检查刷新）
    if let Ok(rssi) = wifi.wifi().get_rssi() {
        shared.lock().unwrap().rssi = rssi;
    }

    let mut cfg = settings::load(&nvs);
    time_svc::set_tz(&cfg.tz);
    info!(
        "run: 亮度 {} / 色温 {} / 时区 {}",
        cfg.brightness, cfg.color, cfg.tz
    );

    info!("entering display loop");
    let mut last_minute = u8::MAX;
    let mut last_wifi_check = std::time::Instant::now();
    loop {
        // 1) 页面改过设置 → 落盘并重新应用
        let dirty = shared.lock().unwrap().settings_dirty;
        if dirty {
            let next = shared.lock().unwrap().settings.clone();
            if let Err(e) = settings::save(&nvs, &next) {
                error!("settings: 落盘失败: {e:#}");
            }
            time_svc::set_tz(&next.tz);
            cfg = next;
            shared.lock().unwrap().settings_dirty = false;
            last_minute = u8::MAX; // 强制重绘（亮度/色温可能变了）
        }

        // 2) 页面请求恢复出厂
        if shared.lock().unwrap().reset_req {
            warn!("run: 页面请求恢复出厂");
            shared.lock().unwrap().reset_req = false;
            if let Err(e) = settings::factory_reset(&nvs) {
                error!("settings: 恢复出厂失败: {e:#}");
            }
            std::thread::sleep(Duration::from_millis(500));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }

        // 3) 页面提交了新的 WiFi 凭据
        if let Some(new) = shared.lock().unwrap().new_wifi.take() {
            info!("run: 更换网络为 \"{}\"，写入后重启", new.ssid);
            if let Err(e) = creds::save(&nvs, &new) {
                error!("creds: 保存失败: {e:#}");
            } else {
                std::thread::sleep(Duration::from_millis(500));
                unsafe { esp_idf_svc::sys::esp_restart() };
            }
        }

        // 4) WiFi 掉线自愈：每 30s 检查一次（掉线不影响走时，只影响 NTP 校准）
        if last_wifi_check.elapsed() >= Duration::from_secs(30) {
            last_wifi_check = std::time::Instant::now();
            let _ = wifi::ensure_connected(&mut wifi, &active_creds);
            // 顺手把信号强度刷给管理页
            if let Ok(rssi) = wifi.wifi().get_rssi() {
                shared.lock().unwrap().rssi = rssi;
            }
        }

        // 5) 走时
        let (hour, minute) = time_svc::local_hm();
        if minute != last_minute {
            last_minute = minute;
            let frame = wordclock_core::time_to_frame(hour, minute);
            info!("{hour:02}:{minute:02} -> {} leds", frame.count());
            leds.show_with(&frame, cfg.color_rgb(), cfg.brightness_div())?;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
