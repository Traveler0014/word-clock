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

mod leddrv;
mod time_svc;
mod wifi;

use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::hal::peripherals::Peripherals;
use log::{error, info};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("word-clock boot");

    let peripherals = Peripherals::take()?;

    // LED 初始化 + 上电自检（RGB 全扫一遍，验证灯链与供电后再进网络流程）
    #[allow(deprecated)] // legacy rmt api，同 leddrv.rs 注释
    let mut leds = leddrv::LedDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio4)?;
    leds.self_test()?;

    // USB 重枚举后主机有 ~6s 空窗：停 10s 保证串口监听接上后再打 WiFi 日志
    info!("boot: waiting 10s for console attach...");
    std::thread::sleep(Duration::from_secs(10));

    // WiFi（connect/netif 失败在 connect 内部无限重试；创建阶段的错误视为致命）
    let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take()?;
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take()?;
    let _wifi = wifi::connect(peripherals.modem, sysloop, nvs)?;

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

    info!("entering display loop");
    let mut last_minute = u8::MAX;
    loop {
        let (hour, minute) = time_svc::local_hm();
        if minute != last_minute {
            last_minute = minute;
            let frame = wordclock_core::time_to_frame(hour, minute);
            info!("{hour:02}:{minute:02} -> {} leds", frame.count());
            leds.show(&frame)?;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
