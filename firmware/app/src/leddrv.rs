//! WS2812B 驱动（IDF legacy RMT，`rmt_write_items` 路径）。
//!
//! 为什么不用 `vendor/ws2812-esp32-rmt-driver`：
//! 它走 `TxRmtDriver::start_iter_blocking()`，而该 API 把「把位流喂给 RMT」这件事
//! 放进 RMT 中断里用 Rust 迭代器做（见 esp-idf-hal `rmt_legacy.rs` 的文档警告：
//! “Iteration must also be fast so that there are no time-gaps ... can cause weird
//!  behavior, counteracted with increasing mem_block_num”）。本机 WiFi 一开中断就会被
//! 抢占 → 供数欠载 → WS2812 提前锁存 → 后段灯保持旧数据（黑）/整段出现全 1（白）。
//!
//! 现在改成：帧先编码成 Symbol 序列放进 RAM，再用 `start_blocking()` →
//! `rmt_write_items()`，由 IDF 自己在中断里从 RAM 搬运，不再回调 Rust 代码。
//! 同时把 `mem_block_num` 从默认 1 提到 4，给欠载留更多余量。

// 整个模块都建立在 legacy RMT API 上，deprecation 告警为预期（见文件头说明）
#![allow(deprecated)]

use core::time::Duration;

use anyhow::Result;
// legacy rmt api：esp-idf-svc/hal 0.46 的 RMT 旧接口，deprecation 为预期
#[allow(deprecated)]
use esp_idf_svc::hal::{
    gpio::OutputPin,
    // 注意：hal 0.46 里 legacy RMT 以 `rmt` 名字导出（需开 hal 的 rmt-legacy feature）
    rmt::{
        config::TransmitConfig, PinState, Pulse, RmtChannel, TxRmtDriver, VariableLengthSignal,
    },
};
use wordclock_core::TOTAL_LEDS;

/// WS2812B 时序（与 datasheet 标称值一致）
const T0H: Duration = Duration::from_nanos(400);
const T0L: Duration = Duration::from_nanos(850);
const T1H: Duration = Duration::from_nanos(800);
const T1L: Duration = Duration::from_nanos(450);
/// 帧末低电平锁存时间（datasheet 要求 >50µs）
const LATCH: Duration = Duration::from_micros(80);

pub struct LedDriver<'d> {
    tx: TxRmtDriver<'d>,
    /// 复用的位流缓冲：114 灯 × 24 bit + 锁存
    sig: VariableLengthSignal,
    /// 诊断：上一帧 encode / RMT 写入各耗时多少微秒
    pub last_encode_us: u64,
    pub last_tx_us: u64,
    bit0: (Pulse, Pulse),
    bit1: (Pulse, Pulse),
    latch: (Pulse, Pulse),
}

impl<'d> LedDriver<'d> {
    /// 例：`LedDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio4)`
    #[allow(deprecated)]
    pub fn new(channel: impl RmtChannel + 'd, pin: impl OutputPin + 'd) -> Result<Self> {
        // clock_divider(1) → 80MHz 计时精度；mem_block_num(4) → 更多缓冲抗中断抢占
        let cfg = TransmitConfig::new().clock_divider(1).mem_block_num(4);
        let tx = TxRmtDriver::new(channel, pin, &cfg)?;
        let hz = tx.counter_clock()?;

        let pulse = |level: PinState, d: Duration| -> Result<Pulse> {
            Ok(Pulse::new_with_duration(hz, level, &d)?)
        };
        let bit0 = (
            pulse(PinState::High, T0H)?,
            pulse(PinState::Low, T0L)?,
        );
        let bit1 = (
            pulse(PinState::High, T1H)?,
            pulse(PinState::Low, T1L)?,
        );
        // 锁存：拆成两段低电平，避免尾部出现 0 tick 的脉冲
        let latch_head = Duration::from_micros(70);
        let latch = (
            pulse(PinState::Low, latch_head)?,
            pulse(PinState::Low, LATCH - latch_head)?,
        );

        Ok(Self {
            tx,
            // 114×24×2 个脉冲 + 锁存
            sig: VariableLengthSignal::with_capacity(TOTAL_LEDS * 24 * 2 + 2),
            last_encode_us: 0,
            last_tx_us: 0,
            bit0,
            bit1,
            latch,
        })
    }

    /// 把一整帧 (114 像素, GRB) 编码进 `sig`
    fn encode(&mut self, rgb: &[(u8, u8, u8); TOTAL_LEDS]) -> Result<()> {
        self.sig.clear();
        for &(r, g, b) in rgb.iter() {
            // WS2812B 线序是 GRB
            for byte in [g, r, b] {
                for shift in (0..8).rev() {
                    let pair = if byte & (1 << shift) != 0 {
                        self.bit1
                    } else {
                        self.bit0
                    };
                    self.sig.push([&pair.0, &pair.1])?;
                }
            }
        }
        self.sig.push([&self.latch.0, &self.latch.1])?;
        Ok(())
    }

    /// 直接写一帧原始 RGB（调试/标尺用，不做亮度缩放）
    pub fn show_rgb(&mut self, rgb: &[(u8, u8, u8); TOTAL_LEDS]) -> Result<()> {
        let t0 = std::time::Instant::now();
        self.encode(rgb)?;
        let t1 = std::time::Instant::now();
        self.tx.start_blocking(&self.sig)?;
        self.last_encode_us = t1.duration_since(t0).as_micros() as u64;
        self.last_tx_us = t1.elapsed().as_micros() as u64;
        Ok(())
    }

    /// 全屏单色（配网/状态指示用）
    pub fn solid(&mut self, rgb: (u8, u8, u8)) -> Result<()> {
        let mut px = [rgb; TOTAL_LEDS];
        self.show_rgb(&mut px)?;
        Ok(())
    }

    /// 上电自检：全链 R→G→B→灭（每步 400ms）。
    /// 常规启动已改用开屏动效（对角扫同样全板点亮），此函数只在 --features diag 用。
    /// 全屏都变 = 数据链电气连通；只有部分变 = 链断在那一颗之后。
    #[allow(dead_code)] // 仅 --features diag 使用
    pub fn self_test(&mut self) -> Result<()> {
        for color in [
            (32u8, 0u8, 0u8),
            (0, 32, 0),
            (0, 0, 32),
            (0, 0, 0),
        ] {
            self.encode(&[color; TOTAL_LEDS])?;
            self.tx.start_blocking(&self.sig)?;
            std::thread::sleep(Duration::from_millis(400));
        }
        Ok(())
    }
}

