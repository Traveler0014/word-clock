//! WiFi STA 连接。
//!
//! 设计约束（实测踩坑详见 docs/development.md）：
//! - `auth_method` 必须显式设置：`ClientConfiguration::default()` 是 `None`，
//!   密码再对也连不上 WPA2 AP（症状为 `ESP_ERR_TIMEOUT`）。
//! - connect/netif 失败只记日志并无限重试，绝不让 main 退出
//!   （app 返回 → IDF 重启整机 → 自检循环闪）。
//! - 每轮连接前先扫描确认 SSID 可见，避免凭据/AP 错误导致的盲目重试。

use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{error, info, warn};

/// 凭据从编译期环境变量读取：
/// `WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo build -p word-clock`
pub fn connect(
    modem: Modem<'static>,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<BlockingWifi<EspWifi<'static>>> {
    let ssid = option_env!("WIFI_SSID").unwrap_or("YOUR_SSID");
    let pass = option_env!("WIFI_PASSWORD").unwrap_or("");

    let mut wifi = BlockingWifi::wrap({
        info!("wifi: creating EspWifi...");
        let w = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
        info!("wifi: EspWifi created");
        w
    }, sysloop)?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().expect("SSID too long"),
        password: pass.try_into().expect("password too long"),
        // Default 的 auth_method 是 None，密码再对也连不上 WPA2 AP（实测 ESP_ERR_TIMEOUT）
        auth_method: if pass.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPAWPA2Personal
        },
        ..Default::default()
    }))?;
    wifi.start()?;
    info!("wifi: driver started");

    // 无限重试：连接失败只记日志并重来，绝不让 main 退出（退出会触发整机重启循环）
    loop {
        // 先扫描确认 SSID 可见，避免凭据错误导致的盲目重试
        let (ap_list, _) = wifi.wifi_mut().scan_n::<16>()?;
        let found = ap_list.iter().find(|ap| ap.ssid == ssid);
        match found {
            Some(ap) => info!(
                "wifi: scan found \"{ssid}\" rssi={} auth={:?}",
                ap.signal_strength, ap.auth_method
            ),
            None => {
                let ssids: Vec<&str> = ap_list.iter().map(|ap| ap.ssid.as_str()).collect();
                warn!("wifi: scan did NOT find \"{ssid}\", visible: {ssids:?}");
            }
        }

        info!("wifi connecting to {ssid} ... (attempt)");
        match wifi.connect() {
            Ok(()) => info!("wifi: connect() returned Ok"),
            Err(e) => {
                error!("wifi connect failed: {e:?}, retry in 5s");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        }
        info!("wifi: waiting netif up...");
        match wifi.wait_netif_up() {
            Ok(()) => {}
            Err(e) => {
                warn!("netif up failed: {e:?}, retry in 5s");
                std::thread::sleep(Duration::from_secs(5));
                // 断开残留状态后重连
                let _ = wifi.disconnect();
                continue;
            }
        }
        break;
    }

    let ip = wifi.wifi().sta_netif().get_ip_info()?;
    info!("wifi up, ip = {:?}", ip.ip);
    Ok(wifi)
}
