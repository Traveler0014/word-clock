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

mod anim_svc;
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

    // 动效服务：所有灯效都从这里出帧（状态见 wordclock_core::anim::State）
    // diag 固件在下面 diverge（只跑静态标尺），所以这里允许未使用
    #[cfg_attr(feature = "diag", allow(unused_mut, unused_variables))]
    let mut anim = anim_svc::AnimSvc::new(20261018);
    #[cfg_attr(feature = "diag", allow(unused_variables))]
    let boot_color = (255u8, 246u8, 232u8); // 开屏固定正白（设置还没读出来）

    #[cfg(feature = "diag")]
    leds.self_test()?;

    // 调试固件（--features diag）：不连 WiFi，只显示静态标尺，用于验证
    // 灯链映射 / 信号完整性是否修好。
    #[cfg(feature = "diag")]
    diag_pattern(&mut leds);

    // 开屏：右下 K 起点的对角自检扫 550ms + KLARA 名字星座约 3s
    anim.set_state(wordclock_core::anim::State::Boot);
    info!("boot: 开屏动效（自检扫 + KLARA）");
    anim.run_for(&mut leds, None, boot_color, Duration::from_millis(3600))?;

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
        Some(c) => {
            // 连接中：WIFI 常亮 + 快速对角扫（动效与固件状态一一对应）
            anim.set_state(wordclock_core::anim::State::WifiConnecting);
            let mut got = None;
            for n in 1..=8u32 {
                info!("wifi: 尝试连接 \"{}\" ({n}/8)", c.ssid);
                match wifi::connect_once(&mut wifi, &c) {
                    Ok(ip) => {
                        info!("wifi ready, ip = {ip}");
                        got = Some(ip);
                        break;
                    }
                    Err(e) => {
                        warn!("wifi: 第 {n} 次连接失败: {e:#}");
                        let _ = wifi.disconnect();
                        anim.run_for(&mut leds, None, boot_color, Duration::from_secs(5))?;
                    }
                }
            }
            match got {
                Some(ip) => Some((c, ip)),
                None => {
                    warn!("wifi: 连不上已保存的网络，进入配网模式");
                    None
                }
            }
        }
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
            web::run_setup(&mut wifi, &nvs, &mut leds, &mut anim)
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
    // 校时中：SYNC 逐字点亮 + 四角点 25% 刻度
    anim.set_state(wordclock_core::anim::State::NtpSyncing);
    info!("waiting for NTP sync ...");
    while !time_svc::is_synced(&sntp) {
        anim.tick(&mut leds, None, boot_color)?;
        std::thread::sleep(Duration::from_millis(10));
    }
    info!("NTP synced");

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

    let cfg0 = settings::load(&nvs);
    time_svc::set_tz(&cfg0.tz);
    info!(
        "run: 亮度 {} / 色温 {} / 时区 {}",
        cfg0.brightness, cfg0.color, cfg0.tz
    );

    info!("entering display loop");
    // 动效配色：设置里的色温，按亮度档位预先缩放一次
    let div = cfg0.brightness_div().max(1);
    let c = cfg0.color_rgb();
    let mut anim_color = (c.0 / div, c.1 / div, c.2 / div);
    let mut clock_frame = wordclock_core::time_to_frame(
        time_svc::local_hm().0,
        time_svc::local_hm().1,
    );
    let mut minute_changed_at: Option<std::time::Instant> = None;
    let mut wifi_lost = false;
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
            // 亮度/色温变了 → 重算动效配色（下一次 tick 生效）
            let d = next.brightness_div().max(1);
            let nc = next.color_rgb();
            anim_color = (nc.0 / d, nc.1 / d, nc.2 / d);
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
            let ok = wifi::ensure_connected(&mut wifi, &active_creds);
            // 断网：时间照常显示但降级（暗 50% + 偏暖），恢复后回到正常态
            if ok != wifi_lost && minute_changed_at.is_none() {
                wifi_lost = ok != true;
                anim.set_state(if wifi_lost {
                    wordclock_core::anim::State::ErrorNetwork
                } else {
                    wordclock_core::anim::State::Running
                });
            }
            // 顺手把信号强度刷给管理页
            if let Ok(rssi) = wifi.wifi().get_rssi() {
                shared.lock().unwrap().rssi = rssi;
            }
        }

        // 5) 走时 + 动效
        //    换词时切到 MinuteChange（四角点滴答），1.2s 后回到 Running
        let (hour, minute) = time_svc::local_hm();
        if minute != last_minute {
            last_minute = minute;
            clock_frame = wordclock_core::time_to_frame(hour, minute);
            info!("{hour:02}:{minute:02} -> {} leds", clock_frame.count());
            minute_changed_at = Some(std::time::Instant::now());
            anim.set_state(wordclock_core::anim::State::MinuteChange);
        }
        if let Some(at) = minute_changed_at {
            if at.elapsed() >= Duration::from_millis(1200) {
                minute_changed_at = None;
                anim.set_state(if wifi_lost {
                    wordclock_core::anim::State::ErrorNetwork
                } else {
                    wordclock_core::anim::State::Running
                });
            }
        }
        anim.tick(&mut leds, Some(&clock_frame), anim_color)?;
        std::thread::sleep(Duration::from_millis(10));
    }
}
