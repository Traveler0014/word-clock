//! 动效服务：把 `wordclock_core::anim` 算出的帧按 ~25fps 推到灯板。
//!
//! 为什么是"服务"而不是独立线程：`LedDriver` 是 main 独占的（web 层按调用
//! 借用），开线程要引入 Arc<Mutex<>>，而 25fps 的算帧成本只有几十微秒，
//! 直接在各循环里调 `tick()` 更简单、也不会有锁竞争。
//!
//! 调用约定：每个循环体里无脑调 `tick()`；内部自己做 40ms 节流。

use std::time::{Duration, Instant};

use anyhow::Result;
use wordclock_core::anim::{Anim, Lights, State};
use wordclock_core::TOTAL_LEDS;
use wordclock_core::Frame;

use crate::leddrv::LedDriver;

const FRAME_MS: u64 = 40; // 25fps
const DEFAULT_FADE_MS: u32 = 700;

pub struct AnimSvc {
    anim: Anim,
    state: State,
    entered: Instant,
    /// 上一帧：状态切换时用它做交叉淡入（"加载中 → 时间"就会自然读成"星星聚拢成文字"）
    prev: Lights,
    /// 最近一帧（下次切状态时成为 prev）
    last_lights: Lights,
    last_tick: Instant,
    fade_ms: u32,
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
            // 让第一次 tick 立刻绘帧
            last_tick: now - Duration::from_millis(FRAME_MS),
            fade_ms: DEFAULT_FADE_MS,
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
        leds.show_rgb(&rgb)?;
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
fn smoothstep(u: u16) -> u16 {
    let x = u as u64;
    let v = (3 * x * x * 1000 - 2 * x * x * x) / 1_000_000;
    v.min(1000) as u16
}
