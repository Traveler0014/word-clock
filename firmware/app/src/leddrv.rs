use std::time::Duration;

use anyhow::Result;
// legacy rmt api：vendor 驱动锁定 hal 0.46 旧接口，新版 rmt api 不兼容，deprecation 为预期
#[allow(deprecated)]
use esp_idf_svc::hal::{gpio::OutputPin, rmt::RmtChannel};
use smart_leds_trait::{RGB8, SmartLedsWrite};
use wordclock_core::{Frame, TOTAL_LEDS};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

/// 字母颜色（暖白）与限亮度分压系数
const COLOR: RGB8 = RGB8 {
    r: 255,
    g: 190,
    b: 110,
};
const BRIGHTNESS_DIV: u8 = 4;

fn scaled(c: RGB8) -> RGB8 {
    RGB8 {
        r: c.r / BRIGHTNESS_DIV,
        g: c.g / BRIGHTNESS_DIV,
        b: c.b / BRIGHTNESS_DIV,
    }
}

pub struct LedDriver<'d> {
    ws: Ws2812Esp32Rmt<'d>,
}

impl<'d> LedDriver<'d> {
    /// 例：`LedDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio4)`
    #[allow(deprecated)]
    pub fn new(channel: impl RmtChannel + 'd, pin: impl OutputPin + 'd) -> Result<Self> {
        Ok(Self {
            ws: Ws2812Esp32Rmt::new(channel, pin)?,
        })
    }

    /// 按字钟帧整帧刷新
    pub fn show(&mut self, frame: &Frame) -> Result<()> {
        let pixels = frame
            .lit
            .iter()
            .map(|&on| if on { scaled(COLOR) } else { RGB8::default() });
        self.ws.write(pixels)?;
        Ok(())
    }

    /// 上电自检：全链 R→G→B→灭
    pub fn self_test(&mut self) -> Result<()> {
        for c in [
            RGB8 { r: 32, g: 0, b: 0 },
            RGB8 { r: 0, g: 32, b: 0 },
            RGB8 { r: 0, g: 0, b: 32 },
        ] {
            self.ws.write(core::iter::repeat_n(c, TOTAL_LEDS))?;
            std::thread::sleep(Duration::from_millis(400));
        }
        self.ws
            .write(core::iter::repeat_n(RGB8::default(), TOTAL_LEDS))?;
        Ok(())
    }
}
