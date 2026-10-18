//! 动效服务：把 `wordclock_core::anim` 算出的帧按 ~25fps 推到灯板。
//!
//! 为什么是"服务"而不是独立线程：`LedDriver` 是 main 独占的（web 层按调用
//! 借用），开线程要引入 Arc<Mutex<>>，而 25fps 的算帧成本只有几十微秒，
//! 直接在各循环里调 `tick()` 更简单、也不会有锁竞争。
//!
//! 调用约定：每个循环体里无脑调 `tick()`；内部自己做 40ms 节流。

use std::time::{Duration, Instant};

use anyhow::Result;
use log::info;
use wordclock_core::anim::{Anim, Lights, State};
use wordclock_core::TOTAL_LEDS;
use wordclock_core::Frame;

use crate::leddrv::LedDriver;

// 实测（ESP32-C5，legacy RMT，**release 构建**）：
//   单帧 show_rgb ≈ 1.6~3.4ms —— 就是 114×24bit 在 800kHz 上的线材时间，
//   即驱动已到物理上限，不需要 DMA。
//   （曾经的 175ms 是 **debug 构建 + Flash 状态损坏** 造成的假象，见 docs/development.md）
// 取 8fps：给 WiFi/HTTP 留余量，同时保证动效每一步至少跨 1~2 帧；
// 所有动效节奏按"一帧 ≈ 125ms"定标（见 wordclock-core::anim）。
const FRAME_MS: u64 = 125; // ~8fps
const DEFAULT_FADE_MS: u32 = 700;

pub struct AnimSvc {
    anim: Anim,
    state: State,
    entered: Instant,
    /// 上一帧：状态切换时用它做交叉淡入（"加载中 → 时间"就会自然读成"星星聚拢成文字"）
    prev: Lights,
    /// 最近一帧（下次切状态时成为 prev）
    last_lights: Lights,
    /// 最近一次真正写进灯板的 RGB（用于跳过重复帧）
    last_rgb: Option<[(u8, u8, u8); TOTAL_LEDS]>,
    last_tick: Instant,
    fade_ms: u32,
    // 统计：实测帧率与单帧写入耗时（判断瓶颈在算帧还是 RMT 写入）
    stat_frames: u32,
    stat_show_us: u64,
    stat_writes: u32,
    stat_skips: u32,
    stat_since: Instant,
}

impl AnimSvc {
    pub fn new(seed: u32) -> Self {
        let now = Instant::now();
        Self {
            anim: Anim::new(seed, Some((8, 1))),
            state: State::Boot,
            entered: now,
            prev: Lights::new(),
            last_lights: Lights::new(),
            last_rgb: None,
            // 让第一次 tick 立刻绘帧
            last_tick: now - Duration::from_millis(FRAME_MS),
            fade_ms: DEFAULT_FADE_MS,
            stat_frames: 0,
            stat_show_us: 0,
            stat_writes: 0,
            stat_skips: 0,
            stat_since: now,
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> State {
        self.state
    }

    /// 切状态。相同状态重复调用是幂等的（循环里可以每次都调）。
    pub fn set_state(&mut self, next: State) {
        if next == self.state {
            return;
        }
        self.prev = self.last_lights;
        self.state = next;
        self.entered = Instant::now();
    }

    /// 推进一帧（内部节流到 ~25fps）。`clock` 为空时文字层不画。
    pub fn tick(
        &mut self,
        leds: &mut LedDriver,
        clock: Option<&Frame>,
        base: (u8, u8, u8),
    ) -> Result<()> {
        if self.last_tick.elapsed() < Duration::from_millis(FRAME_MS) {
            return Ok(());
        }
        self.last_tick = Instant::now();

        let t_ms = self.entered.elapsed().as_millis().min(u32::MAX as u128) as u32;
        let mut lights = self.anim.render(self.state, t_ms, clock);

        // 状态切换后的交叉淡入
        if t_ms < self.fade_ms {
            let k = smoothstep((t_ms * 1000 / self.fade_ms) as u16);
            for i in 0..TOTAL_LEDS {
                let a = self.prev.a[i] as u32 * (1000 - k as u32) + lights.a[i] as u32 * k as u32;
                lights.a[i] = (a / 1000) as u16;
            }
        }
        self.last_lights = lights;

        let mut rgb = [(0u8, 0u8, 0u8); TOTAL_LEDS];
        lights.to_rgb(base, &mut rgb);

        // 内容没变就不重发帧：静态画面（运行态的静止星空 + 时间文字）可以做到
        // 完全零写入，既省电也避免无谓的 RMT 占用
        if self.last_rgb.is_some() && self.last_rgb.unwrap() == rgb {
            self.stat_frames += 1;
            self.stat_skips += 1;
            self.maybe_log();
            return Ok(());
        }

        let t0 = Instant::now();
        leds.show_rgb(&rgb)?;
        let us = t0.elapsed().as_micros() as u64;
        self.last_rgb = Some(rgb);
        self.stat_frames += 1;
        self.stat_writes += 1;
        self.stat_show_us += us;
        self.maybe_log();
        Ok(())
    }

    /// 在 `total` 时长内持续 tick（用于 boot、失败重试等待等阻塞段）
    pub fn run_for(
        &mut self,
        leds: &mut LedDriver,
        clock: Option<&Frame>,
        base: (u8, u8, u8),
        total: Duration,
    ) -> Result<()> {
        let until = Instant::now() + total;
        while Instant::now() < until {
            self.tick(leds, clock, base)?;
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

/// 3u²-2u³，u ∈ 0..=1000 → 0..=1000
impl AnimSvc {
    /// 每 30s 打一行实测统计：帧率 + 单帧写入耗时（排查性能时看这个）
    fn maybe_log(&mut self) {
        if self.stat_since.elapsed() < Duration::from_secs(30) {
            return;
        }
        let el = self.stat_since.elapsed().as_micros().max(1) as u64;
        let fps = self.stat_frames as u64 * 1_000_000 / el;
        let avg = if self.stat_frames > 0 {
            self.stat_show_us / self.stat_frames.max(1) as u64
        } else {
            0
        };
        info!(
            "anim: 状态={:?} tick={}fps 写/跳={}/{} 写入均={}us",
            self.state, fps, self.stat_writes, self.stat_skips, avg
        );
        self.stat_frames = 0;
        self.stat_show_us = 0;
        self.stat_writes = 0;
        self.stat_skips = 0;
        self.stat_since = Instant::now();
    }
}

fn smoothstep(u: u16) -> u16 {
    let x = u as u64;
    let v = (3 * x * x * 1000 - 2 * x * x * x) / 1_000_000;
    v.min(1000) as u16
}
