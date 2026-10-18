//! 灯板动效：状态 → 114 灯的感知亮度（定点 0..=1000）→ 8bit RGB。
//!
//! 设计规格见 `docs/animation.md`，可执行的参考实现在 `tools/anim_lab.html`。
//! 本模块是它的**固件版本**，刻意遵守三条：
//!
//! 1. **纯逻辑、no_std、无分配**：所有状态、星表、字母点位都是常量或定长数组，
//!    方便在宿主机 `cargo test` 里逐帧校验。
//! 2. **定点数 + LUT**：core 里没有 `powf`/`sinf`，用 `SIN`/`GAMMA` 两张表代替。
//!    这同时是将来做 conformance（网页 ↔ 固件逐帧一致）的数值基准。
//! 3. **状态集合由固件定义**（`State`），主题只决定"怎么表现"。新增效果要改这里，
//!    改参数不需要。

use crate::{Frame, COLS, CORNER_LEDS, LETTER_LEDS, ROWS, TOTAL_LEDS};

pub mod lut;
use lut::{GAMMA, SIN};

/// 感知亮度的满值（定点 0..=MAXA）
pub const MAXA: u16 = 1000;

// ─────────────────────────── 字母阵 ───────────────────────────
/// 物理字母阵（11 列 × 10 行），与面板镂空一一对应
pub const GRID: [[u8; COLS]; ROWS] = [
    *b"ITLISASAMPM",
    *b"ACQUARTERDC",
    *b"TWENTYFIVEX",
    *b"HALFSTENFTO",
    *b"PASTERUNINE",
    *b"ONESIXTHREE",
    *b"FOURFIVETWO",
    *b"EIGHTELEVEN",
    *b"SEVENTWELVE",
    *b"TENSEOCLOCK",
];

// ─────────────────────── 标签 / 名字的点位 ───────────────────────
// 全部按"单轴严格单调 + 紧凑"人工摆位（见 docs/animation.md 的点位表）。
// 顺序即点亮顺序；改动请同时更新文档与实验台。
pub const BOOT_NAME: &[(u8, u8)] = &[(9, 10), (7, 6), (4, 1), (1, 5), (0, 5)]; // KLARA
pub const LABEL_AP: &[(u8, u8)] = &[(0, 7), (0, 9)];
pub const LABEL_WIFI: &[(u8, u8)] = &[(2, 1), (0, 3), (2, 6), (2, 7)];
pub const LABEL_WAIT: &[(u8, u8)] = &[(2, 1), (1, 4), (2, 7), (3, 9)];
pub const LABEL_SYNC: &[(u8, u8)] = &[(3, 4), (2, 5), (3, 7), (1, 10)];
pub const LABEL_OTA: &[(u8, u8)] = &[(6, 1), (4, 3), (1, 4)];
pub const LABEL_NET: &[(u8, u8)] = &[(5, 1), (5, 2), (4, 3)];
pub const LABEL_TIME: &[(u8, u8)] = &[(1, 6), (2, 7), (0, 8), (2, 9)];

// ─────────────────────────── 状态 ───────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Boot,
    ApStart,
    WifiConnecting,
    WifiScanning,
    ProvisioningWait,
    ProvisioningClient,
    NtpSyncing,
    Running,
    MinuteChange,
    OtaDownload,
    OtaApply,
    ErrorNetwork,
    ErrorTime,
    FactoryReset,
    DeepNight,
    ApplyConfig,
}

// ─────────────────────────── 星表 ───────────────────────────
pub const STAR_MAX: usize = 40;

#[derive(Clone, Copy)]
pub struct Star {
    pub led: u8,
    pub peak: u16,
    pub period_ms: u32,
    pub phase: u16, // 0..1000
    pub tint: u8,
}

pub struct Anim {
    pub stars: [Star; STAR_MAX],
    pub n: usize,
    /// planet 暖点（唯一的"生命感"信号，运行态里唯一会动的东西）
    pub planet: Option<u8>,
}

impl Anim {
    /// 用固定种子生成星表：同一台设备每次开机都是同一片星空。
    /// 参数与 `tools/anim_lab.html` 的默认值一致（32 颗、亮星一半、峰值 0.6）。
    pub fn new(seed: u32, planet_pos: Option<(u8, u8)>) -> Self {
        let mut st = [Star { led: 0, peak: 0, period_ms: 0, phase: 0, tint: 0 }; STAR_MAX];
        let mut rnd = Mulberry32::new(seed);

        // 洗牌：决定哪些格子是星
        let mut order = [0u8; LETTER_LEDS];
        for (i, v) in order.iter_mut().enumerate() {
            *v = i as u8;
        }
        for i in (1..LETTER_LEDS).rev() {
            let j = (rnd.next() % (i as u32 + 1)) as usize;
            order.swap(i, j);
        }

        const N: usize = 32;
        let n_bright = N / 2; // 亮星比例 0.5
        for i in 0..N {
            let r1 = rnd.next() % 1000;
            let r2 = rnd.next() % 1000;
            let r3 = rnd.next() % 1000;
            let r4 = rnd.next() % 1000;
            // 星等分级：亮星稀缺，越弱越多
            let mag = if i < n_bright {
                850 + (150 * r1) / 1000 // 0.85 ~ 1.00
            } else if i < n_bright + (N * 4) / 10 {
                450 + (100 * r1) / 1000 // 0.45 ~ 0.55
            } else {
                220 + (80 * r1) / 1000 // 0.22 ~ 0.30
            };
            let peak = ((600 * mag) / 1000) as u16;
            let period_ms = 3600 + (5900 * r2) / 1000; // 3.6 ~ 9.5s，互不整除
            let tint = if r4 < 620 {
                0
            } else if r4 < 840 {
                1
            } else if r4 < 970 {
                2
            } else {
                3
            };
            st[i] = Star {
                led: order[i],
                peak,
                period_ms,
                phase: (1000 * r3) as u16,
                tint,
            };
        }
        let planet = planet_pos.map(|(r, c)| crate::xy_to_index(r as usize, c as usize) as u8);
        Self { stars: st, n: N, planet }
    }

    /// 一帧：给定状态、状态内时间、当前时间文字帧 → 每颗灯的 alpha + 色档。
    pub fn render(&self, state: State, t_ms: u32, clock: Option<&Frame>) -> Lights {
        let mut l = Lights::new();
        self.stars_base(state, t_ms, &mut l);
        self.mods(state, t_ms, &mut l);
        self.text(state, clock, &mut l);
        l.cap(SUM_BUDGET);
        l
    }

    // ── 星场：所有状态的基底，按状态做乘性调制 ──
    fn stars_base(&self, state: State, t_ms: u32, l: &mut Lights) {
        if state == State::FactoryReset || state == State::ApplyConfig {
            // 归零/重启：不画星场（各自用 mods 表达）
            if state == State::FactoryReset {
                return;
            }
        }
        // (rate‰, peak_mul‰, tide, fall)
        let (rate, peak_mul, tide, fall) = match state {
            State::Boot => (1800, 200, None, false),
            State::ApStart => (1000, 300, None, false),
            State::WifiConnecting => (1000, 300, None, false),
            State::WifiScanning => (1000, 300, None, false),
            State::ProvisioningWait => (900, 1000, Some((26_000u32, 250u16)), false),
            State::ProvisioningClient => (1000, 400, None, false),
            State::NtpSyncing => (1100, 500, None, false),
            State::Running | State::MinuteChange => (0, 300, None, false),
            State::OtaDownload => (1100, 500, None, false),
            State::OtaApply => (1000, 300, None, false),
            State::ErrorNetwork => (120, 260, None, false),
            State::ErrorTime => (100, 220, None, false),
            State::FactoryReset => (1000, 1000, None, true),
            State::DeepNight => (1000, 750, None, false),
            State::ApplyConfig => (600, 300, None, false),
        };

        let tt = (t_ms as u64 * rate as u64 / 1000) as u32;
        let mut peak_mul = peak_mul;
        if let Some((period, lo)) = tide {
            let u = (t_ms % period) * 1000 / period; // 0..1000
            let c = (500 - cos_1000(u as u16) / 2).max(0) as u32; // 0.5-0.5cos
            peak_mul = lo + (((1000 - lo as u32) * c) / 1000) as u16;
        }

        for i in 0..self.n {
            let s = &self.stars[i];
            let mut pm = peak_mul;
            if fall {
                let (r, c) = rc_of_led(s.led as usize);
                if ((r * COLS + c) as u32 * 1000 / LETTER_LEDS as u32) < (t_ms * 1000 / 3200).min(1100) {
                    pm = 0;
                }
            }
            let u = ((tt as u64 * 1000 / s.period_ms as u64) as u32 + s.phase as u32) % 1000;
            let env = twinkle_spike(u as u16);
            let peak = (s.peak as u32 * pm as u32 / 1000) as u16;
            let a = (peak as u32 * env as u32 / 1000) as u16;
            if a > l.a[s.led as usize] {
                l.a[s.led as usize] = a;
                l.tint[s.led as usize] = s.tint;
            }
        }

        // planet：14s 极慢呼吸，让运行态可以完全不闪
        if let (Some(p), true) = (self.planet, state != State::FactoryReset) {
            let u = (t_ms % 14_000) * 1000 / 14_000;
            let breath = (500 - cos_1000(u as u16) / 2).max(0) as u32;
            let a = (100 + (100 * breath) / 1000) as u16;
            put(&mut l.a, &mut l.tint, p as usize, a, 3);
        }
    }

    // ── 叠加层：身份标签 + 运动 ──
    fn mods(&self, state: State, t_ms: u32, l: &mut Lights) {
        match state {
            State::Boot => {
                // ① 从右下 K 起点的对角快扫（功能性自检）——**只跑一遍**，
                //    之后保持"KLARA 常亮 + planet 呼吸"的稳定态，避免开机等待
                //    期间面板定格在扫带的随机位置
                if t_ms < 1600 {
                    band(l, Axis::DiagR, t_ms, 1600, 1250, 420);
                }
                spell(l, BOOT_NAME, t_ms, 900, 500, 500, 950);
            }
            State::ApStart => label_blink(l, LABEL_AP, t_ms, 4000, 550, 0),
            State::WifiConnecting => {
                label_steady(l, LABEL_WIFI, 500);
                band(l, Axis::DiagR, t_ms, 1200, 1100, 450);
            }
            State::WifiScanning => {
                // 横向慢扫：方向 = 标签的阅读方向（左→右），速度编码"耗时"
                label_steady(l, LABEL_WIFI, 500);
                band(l, Axis::X, t_ms, 3200, 1400, 420);
            }
            State::ProvisioningWait => label_blink(l, LABEL_WAIT, t_ms, 6000, 500, 0),
            State::ProvisioningClient => ripple(l, t_ms, (9, 10), 1000, 320),
            State::NtpSyncing => label_progress(l, LABEL_SYNC, t_ms, 6000, 550, true),
            State::MinuteChange => tick_dots(l, 140, 450),
            State::OtaDownload => label_progress(l, LABEL_OTA, t_ms, 14_000, 600, true),
            State::OtaApply => {
                label_steady(l, LABEL_OTA, 550);
                band(l, Axis::DiagR, t_ms, 3000, 1600, 280);
            }
            State::ErrorNetwork => label_blink(l, LABEL_NET, t_ms, 6000, 420, 3),
            State::ErrorTime => label_blink(l, LABEL_TIME, t_ms, 6000, 550, 3),
            State::FactoryReset => fade_all(l, t_ms, 2400),
            State::Running | State::DeepNight | State::ApplyConfig => {}
        }
    }

    // ── 文字层：永远最亮（运行态可降级显示） ──
    fn text(&self, state: State, clock: Option<&Frame>, l: &mut Lights) {
        let (level, tint) = match state {
            State::Running | State::MinuteChange => (MAXA, 0u8),
            // 网络断了但时间还知道 → 降级显示（暗 50% + 偏暖）
            State::ErrorNetwork => (MAXA / 2, 3u8),
            _ => return,
        };
        let Some(f) = clock else { return };
        for i in 0..TOTAL_LEDS {
            if !f.lit[i] {
                continue;
            }
            let a = if i < LETTER_LEDS { level } else { (level as u32 * 85 / 100) as u16 };
            l.a[i] = a;
            l.tint[i] = tint;
        }
    }
}

// ─────────────────────────── 输出缓冲 ───────────────────────────
#[derive(Clone, Copy)]
pub struct Lights {
    pub a: [u16; TOTAL_LEDS],
    pub tint: [u8; TOTAL_LEDS],
}

/// 整帧"码值之和"上限。经验换算：电流 ≈ 0.235 mA × 码值和，
/// 所以 9000 ≈ 2.1A。超过这道闸就整帧等比压暗——WS2812 在电压跌落时会
/// 复位（表现为整板闪白 + 掉帧），必须提前掐住。
pub const SUM_BUDGET: u32 = 9_000;

impl Lights {
    pub fn new() -> Self {
        Self { a: [0; TOTAL_LEDS], tint: [0; TOTAL_LEDS] }
    }

    /// 功率闸：整帧码值和超过预算就等比压暗（保持画面比例，不改变构图）
    pub fn cap(&mut self, budget: u32) {
        let sum: u32 = self.a.iter().map(|&a| to8(a) as u32).sum();
        if sum > budget {
            let k = (budget * 1000) / sum.max(1);
            for a in self.a.iter_mut() {
                *a = ((*a as u32 * k) / 1000) as u16;
            }
        }
    }

    /// 感知 alpha → 8bit RGB（gamma + 色档）。`base` 是设备当前色温。
    pub fn to_rgb(&self, base: (u8, u8, u8), out: &mut [(u8, u8, u8); TOTAL_LEDS]) {
        for i in 0..TOTAL_LEDS {
            out[i] = rgb_for(self.a[i], self.tint[i], base);
        }
    }
}

pub fn rgb_for(a: u16, tint: u8, base: (u8, u8, u8)) -> (u8, u8, u8) {
    let v = to8(a) as u32;
    if v == 0 {
        return (0, 0, 0);
    }
    // 色档：0 设备色温 / 1 偏冷 / 2 偏暖 / 3 琥珀
    let (r, g, b) = match tint {
        1 => (base.0 as u32 * 95 / 100, base.1 as u32 * 98 / 100, base.2 as u32),
        2 => (base.0 as u32, base.1 as u32 * 94 / 100, base.2 as u32 * 78 / 100),
        3 => (255u32, 150, 60),
        _ => (base.0 as u32, base.1 as u32, base.2 as u32),
    };
    (
        ((r * v) / 255) as u8,
        ((g * v) / 255) as u8,
        ((b * v) / 255) as u8,
    )
}

/// 感知 alpha(0..=1000) → 8bit 线性光输出
pub fn to8(a: u16) -> u8 {
    let idx = (a as u32 * 255 / MAXA as u32).min(255) as usize;
    GAMMA[idx]
}

// ─────────────────────────── 生成器 ───────────────────────────
fn put(a: &mut [u16; TOTAL_LEDS], tint: &mut [u8; TOTAL_LEDS], i: usize, v: u16, t: u8) {
    if v > a[i] {
        a[i] = v;
        tint[i] = t;
    }
}

fn add(a: &mut [u16; TOTAL_LEDS], tint: &mut [u8; TOTAL_LEDS], i: usize, v: u16, t: u8) {
    let s = (a[i] as u32 + v as u32).min(MAXA as u32) as u16;
    if s > a[i] {
        a[i] = s;
        tint[i] = t;
    }
}

/// 光带：axis 决定往哪个方向扫，`period_ms` 一轮。
/// `width_milli` 是高斯宽度，单位"毫格"（1250 = 1.25 格）。
/// 单位必须和 `d` 一致——这里 d 也是毫格。
fn band(l: &mut Lights, axis: Axis, t_ms: u32, period_ms: u32, width_x100: u16, peak: u16) {
    let width = width_x100 as i32;
    // 投影坐标：x → 列；y → 行；diag → 右上到左下；diagr → 右下到左上
    let (span, proj): (i32, fn(usize, usize) -> i32) = match axis {
        Axis::X => (COLS as i32, |_, c| c as i32),
        Axis::Y => (ROWS as i32, |r, _| r as i32),
        Axis::Diag => (COLS as i32 + ROWS as i32, |r, c| c as i32 + (r * COLS / ROWS) as i32),
        Axis::DiagR => (COLS as i32 + ROWS as i32, |r, c| {
            (COLS as i32 - 1 - c as i32) + ((ROWS - 1 - r) * COLS / ROWS) as i32
        }),
    };
    let phase = (t_ms as u64 * 1000 / period_ms as u64 % 1000) as i32;
    // 注意单位：投影坐标是"格"，这里统一换算成"毫格"再和宽度比较。
    // （踩过：把 1.25 格当成 125 格 → 整块板同时点亮 → 电源跌落 → 掉帧闪白）
    let span_m = span * 1000;
    let w2_m = 2 * width; // width 本身就是毫格
    let pos_m = phase * (span_m + w2_m) / 1000 - width;
    for r in 0..ROWS {
        for c in 0..COLS {
            let d = proj(r, c) * 1000 - pos_m;
            let g = gauss_x1000(d, width);
            let v = (peak as u32 * g as u32 / 1000) as u16;
            if v > 0 {
                add(&mut l.a, &mut l.tint, crate::xy_to_index(r, c), v, 0);
            }
        }
    }
}

fn label_steady(l: &mut Lights, cells: &[(u8, u8)], peak: u16) {
    for &(r, c) in cells {
        put(&mut l.a, &mut l.tint, crate::xy_to_index(r as usize, c as usize), peak, 0);
    }
}

/// 同步闪烁：一个周期内两次脉冲（像提醒而不是呼吸）
fn label_blink(l: &mut Lights, cells: &[(u8, u8)], t_ms: u32, period_ms: u32, peak: u16, tint: u8) {
    let u = (t_ms % period_ms) * 2000 / period_ms; // 0..2000 → 两轮
    let phase = (u % 1000) as u16;
    let s = sin_1000(phase).max(0) as u32; // 0..1000
    let env = (s * s / 1000) as u16; // sin^2，锐一点
    let v = (peak as u32 * env as u32 / 1000) as u16;
    for &(r, c) in cells {
        put(&mut l.a, &mut l.tint, crate::xy_to_index(r as usize, c as usize), v, tint);
    }
}

/// 进度：按顺序累计点亮，最后一个轻微呼吸；四角点当 25% 刻度
fn label_progress(l: &mut Lights, cells: &[(u8, u8)], t_ms: u32, demo_ms: u32, peak: u16, dots: bool) {
    let span = demo_ms * 86 / 100;
    let prog = ((t_ms % demo_ms) * 1000 / span).min(1000);
    let shown = ((prog as usize * cells.len() + 999) / 1000).max(1);
    if dots {
        let quarters = prog as usize * 4 / 1000;
        for d in 0..quarters.min(CORNER_LEDS) {
            put(&mut l.a, &mut l.tint, LETTER_LEDS + d, peak / 2, 0);
        }
    }
    let breath_u = (t_ms % 1600) * 1000 / 1600;
    let breath = 550 + 450 * (500 - cos_1000(breath_u as u16) / 2) / 500;
    for (k, &(r, c)) in cells.iter().enumerate() {
        let idx = crate::xy_to_index(r as usize, c as usize);
        if k + 1 < shown {
            put(&mut l.a, &mut l.tint, idx, peak * 3 / 4, 0);
        } else if k + 1 == shown {
            put(&mut l.a, &mut l.tint, idx, (peak as u32 * breath as u32 / 1000) as u16, 0);
        }
    }
}

/// 拼写：逐字渐亮并常亮（KLARA 星座）
fn spell(l: &mut Lights, cells: &[(u8, u8)], t_ms: u32, delay_ms: u32, step_ms: u32, fade_ms: u32, peak: u16) {
    for (k, &(r, c)) in cells.iter().enumerate() {
        let start = delay_ms + k as u32 * step_ms;
        if t_ms < start {
            continue;
        }
        let local = t_ms - start;
        let env = if local < fade_ms { (local * 1000 / fade_ms).min(1000) as u16 } else { MAXA };
        // smoothstep
        let e = (env as u32 * env as u32 * (3000 - 2 * env as u32) / 1_000_000) as u16;
        put(&mut l.a, &mut l.tint, crate::xy_to_index(r as usize, c as usize), (peak as u32 * e as u32 / 1000) as u16, 0);
    }
}

fn tick_dots(l: &mut Lights, dur_ms: u32, peak: u16) {
    let _ = dur_ms;
    for k in 0..CORNER_LEDS {
        put(&mut l.a, &mut l.tint, LETTER_LEDS + k, peak, 0);
    }
}

/// 涟漪：从某格向外扩散（单次，1s）
fn ripple(l: &mut Lights, t_ms: u32, origin: (u8, u8), dur_ms: u32, peak: u16) {
    let u = (t_ms % (dur_ms * 2)) * 1000 / dur_ms; // 0..2000，前半扩散后半静默
    if u > 1350 {
        return;
    }
    let env = if u < 1000 { 1000 } else { 1350 - u } as i32;
    for r in 0..ROWS {
        for c in 0..COLS {
            let dr = (r as i32 - origin.0 as i32) * 1000 / ROWS as i32;
            let dc = (c as i32 - origin.1 as i32) * 1000 / COLS as i32;
            let d = isqrt(dr * dr + dc * dc); // 0..~1400
            let front = (u as i32 * 1600 / 1000) as i32;
            let g = gauss_x1000(d - front, 220);
            let v = (peak as u32 * g as u32 / 1000 * env.max(0) as u32 / 1000) as u16;
            if v > 0 {
                add(&mut l.a, &mut l.tint, crate::xy_to_index(r, c), v, 0);
            }
        }
    }
}

fn fade_all(l: &mut Lights, t_ms: u32, dur_ms: u32) {
    let k = if t_ms >= dur_ms { 0 } else { 1000 - t_ms * 1000 / dur_ms };
    for i in 0..TOTAL_LEDS {
        l.a[i] = (l.a[i] as u32 * k as u32 / 1000) as u16;
    }
}

// ─────────────────────────── 数学小工具 ───────────────────────────
pub fn rc_of_led(led: usize) -> (usize, usize) {
    let r = led / COLS;
    let raw = led % COLS;
    // 蛇形走线的逆映射：奇数行物理列是反的
    (r, if r % 2 == 0 { raw } else { COLS - 1 - raw })
}

/// sin(2πu)，u∈[0,1000)，返回 -1000..=1000
pub fn sin_1000(u: u16) -> i32 {
    let idx = ((u as u32 * 256) / 1000 % 256) as usize;
    SIN[idx] as i32
}

/// cos(2πu)，u∈[0,1000)，返回 -1000..=1000
pub fn cos_1000(u: u16) -> i32 {
    sin_1000((u + 250) % 1000)
}

/// 尖峰曲线 sin^4(πu)：大部分时间接近熄灭、偶尔闪一下（真实星空的手感）
fn twinkle_spike(phase: u16) -> u16 {
    let s = sin_1000(phase / 2).max(0) as u32; // sin(πu)，0..1000
    let s2 = s * s / 1000;
    ((s2 * s2 / 1000) / 1000) as u16
}

/// 高斯衰减（定点）：d, width 同量纲，返回 0..1000
fn gauss_x1000(d: i32, width: i32) -> i32 {
    if width <= 0 {
        return 0;
    }
    // exp(-(d/w)^2) ≈ 1/(1+x+x^2/2+x^3/6…)，此处用 1/(1+x^2+x^4/2) 的廉价近似
    let x = (d as i64 * 1000) / width as i64;
    let x2 = (x * x) / 1000;
    let denom = 1000 + x2 + (x2 * x2) / 2000;
    ((1000 * 1000) / denom.max(1)) as i32
}

fn isqrt(v: i32) -> i32 {
    let mut x = v.max(0);
    let mut r = 0;
    let mut b = 1 << 14;
    while b > 0 {
        if x >= r + b {
            x -= r + b;
            r = (r >> 1) + b;
        } else {
            r >>= 1;
        }
        b >>= 2;
    }
    r
}

#[derive(Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Diag,
    DiagR,
}

// ─────────────────────────── 随机数 ───────────────────────────
struct Mulberry32(u32);

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        (t ^ (t >> 14)) >> 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{time_to_frame, CORNER_LEDS, LETTER_LEDS, TOTAL_LEDS};

    fn letter_at(cell: (u8, u8)) -> u8 {
        GRID[cell.0 as usize][cell.1 as usize]
    }

    #[test]
    fn 标签点位拼出目标词() {
        let cases: &[(&str, &[(u8, u8)])] = &[
            ("KLARA", BOOT_NAME),
            ("AP", LABEL_AP),
            ("WIFI", LABEL_WIFI),
            ("WAIT", LABEL_WAIT),
            ("SYNC", LABEL_SYNC),
            ("OTA", LABEL_OTA),
            ("NET", LABEL_NET),
            ("TIME", LABEL_TIME),
        ];
        for (word, cells) in cases {
            let got: String = cells.iter().map(|&c| letter_at(c) as char).collect();
            assert_eq!(&got, word, "点位与目标词不符: {word}");
        }
    }

    #[test]
    fn 每个词都满足单轴单调() {
        // 英文词：列从左到右（或行从上到下）严格单调；名字允许任意方向
        let words: &[&[(u8, u8)]] = &[LABEL_AP, LABEL_WIFI, LABEL_WAIT, LABEL_SYNC, LABEL_OTA, LABEL_NET, LABEL_TIME];
        for cells in words {
            let cols_inc = cells.windows(2).all(|w| w[0].1 < w[1].1);
            let rows_inc = cells.windows(2).all(|w| w[0].0 < w[1].0);
            assert!(cols_inc || rows_inc, "标签点位缺少单轴单调性: {cells:?}");
        }
        let cols_inc = BOOT_NAME.windows(2).all(|w| w[0].1 < w[1].1);
        let rows_dec = BOOT_NAME.windows(2).all(|w| w[0].0 > w[1].0);
        assert!(rows_dec, "KLARA 应保持从右下到左上（行递减）");
        let _ = cols_inc;
    }

    #[test]
    fn 星表确定且参数合法() {
        let a = Anim::new(20261018, Some((8, 1)));
        let b = Anim::new(20261018, Some((8, 1)));
        assert_eq!(a.n, 32);
        for i in 0..a.n {
            assert_eq!(a.stars[i].led, b.stars[i].led);
            assert_eq!(a.stars[i].peak, b.stars[i].peak);
            assert!(a.stars[i].peak <= MAXA);
            assert!((3600..=9500).contains(&a.stars[i].period_ms));
            assert!((a.stars[i].led as usize) < LETTER_LEDS);
        }
        assert!(a.planet.is_some());
    }

    #[test]
    fn 运行态文字最亮且星空很暗() {
        let a = Anim::new(1, Some((8, 1)));
        let f = time_to_frame(10, 9); // IT IS FIVE PAST TEN
        let l = a.render(State::Running, 5000, Some(&f));
        for i in 0..LETTER_LEDS {
            if f.lit[i] {
                assert_eq!(l.a[i], MAXA, "时间文字必须是最亮层");
            }
        }
        // 星场必须远低于文字（码值 ≤ 20 的"星场层"）
        let mut max_star = 0;
        for i in 0..LETTER_LEDS {
            if !f.lit[i] {
                max_star = max_star.max(to8(l.a[i]));
            }
        }
        assert!(max_star <= 20, "运行态星空应压到星场层, got {max_star}");
    }

    #[test]
    fn 网络错误是降级显示而不是熄灭() {
        let a = Anim::new(1, Some((8, 1)));
        let f = time_to_frame(10, 9);
        let l = a.render(State::ErrorNetwork, 2000, Some(&f));
        let mut lit = 0;
        for i in 0..LETTER_LEDS {
            if f.lit[i] {
                lit += 1;
                assert!(l.a[i] > 300 && l.a[i] < MAXA, "降级显示应为 0.5 左右");
                assert_eq!(l.tint[i], 3, "降级显示应偏暖");
            }
        }
        assert!(lit > 5, "断网时时间仍须可读");
    }

    #[test]
    fn 所有状态输出都在范围内() {
        let a = Anim::new(7, Some((8, 1)));
        let f = time_to_frame(23, 47);
        let states = [
            State::Boot, State::ApStart, State::WifiConnecting, State::WifiScanning,
            State::ProvisioningWait, State::ProvisioningClient, State::NtpSyncing,
            State::Running, State::MinuteChange, State::OtaDownload, State::OtaApply,
            State::ErrorNetwork, State::ErrorTime, State::FactoryReset, State::DeepNight,
            State::ApplyConfig,
        ];
        for st in states {
            for t in [0u32, 250, 1000, 3000, 5000, 9000, 15_000, 30_000] {
                let l = a.render(st, t, Some(&f));
                let mut rgb = [(0u8, 0u8, 0u8); TOTAL_LEDS];
                l.to_rgb((255, 246, 232), &mut rgb);
                for i in 0..TOTAL_LEDS {
                    assert!(l.a[i] <= MAXA, "{st:?}@{t} alpha 越界");
                }
            }
        }
    }

    #[test]
    fn 每个状态都点得亮东西() {
        let a = Anim::new(3, Some((8, 1)));
        let f = time_to_frame(10, 9);
        for st in [State::Boot, State::WifiConnecting, State::WifiScanning, State::ProvisioningWait,
                   State::ProvisioningClient, State::NtpSyncing, State::MinuteChange,
                   State::OtaDownload, State::OtaApply, State::ErrorTime] {
            let l = a.render(st, 3000, Some(&f));
            let lit = l.a.iter().filter(|&&x| x > 20).count();
            assert!(lit > 0, "{st:?} 在 t=3s 什么都没亮");
        }
    }

    #[test]
    fn 四角点刻度随进度增加() {
        let a = Anim::new(3, None);
        let f = time_to_frame(10, 9);
        let early = a.render(State::OtaDownload, 1000, Some(&f));
        let late = a.render(State::OtaDownload, 11_000, Some(&f));
        let count = |l: &Lights| (0..CORNER_LEDS).filter(|&k| l.a[LETTER_LEDS + k] > 0).count();
        assert!(count(&late) >= count(&early), "进度刻度应递增");
    }
}
