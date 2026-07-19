use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::info;

/// 凭据从编译期环境变量读取：
/// `WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo run -p word-clock`
pub fn connect(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<BlockingWifi<'static>> {
    let ssid = option_env!("WIFI_SSID").unwrap_or("YOUR_SSID");
    let pass = option_env!("WIFI_PASSWORD").unwrap_or("");

    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sysloop.clone(), Some(nvs))?, sysloop)?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().expect("SSID too long"),
        password: pass.try_into().expect("password too long"),
        ..Default::default()
    }))?;
    wifi.start()?;
    info!("wifi connecting to {ssid} ...");
    wifi.connect()?;
    wifi.wait_netif_up()?;
    let ip = wifi.wifi().sta_netif().get_ip_info()?;
    info!("wifi up, ip = {:?}", ip.ip);
    Ok(wifi)
}
