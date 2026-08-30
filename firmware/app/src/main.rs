mod leddrv;
mod time_svc;
mod wifi;

use anyhow::Result;
use esp_idf_svc::hal::peripherals::Peripherals;
use log::info;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("word-clock boot");

    let peripherals = Peripherals::take()?;

    // LED 初始化 + 上电自检（RGB 全扫一遍，验证灯链与供电后再进网络流程）
    let mut leds = leddrv::LedDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio4)?;
    leds.self_test()?;

    // WiFi
    let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take()?;
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take()?;
    let _wifi = wifi::connect(peripherals.modem, sysloop, nvs)?;

    // SNTP 对时（UTC+8）
    let sntp = time_svc::start_sntp()?;
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
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
