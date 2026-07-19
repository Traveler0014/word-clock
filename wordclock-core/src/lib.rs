//! 字钟核心逻辑：时间 → LED 点阵。
//! 纯 Rust、无依赖、no_std；可在宿主机 `cargo test`，路线 B（esp-hal）可直接复用。
//!
//! 字母阵（11 列 × 10 行）：
//! ```text
//!  I T L I S A S A M P M
//!  A C Q U A R T E R D C
//!  T W E N T Y F I V E X
//!  H A L F S T E N F T O
//!  P A S T E R U N I N E
//!  O N E S I X T H R E E
//!  F O U R F I V E T W O
//!  E I G H T E L E V E N
//!  S E V E N T W E L V E
//!  T E N S E O C L O C K
//! ```
//! LED 110~113 为四角分钟点（+1 ~ +4 分钟）。

#![cfg_attr(not(test), no_std)]

pub const COLS: usize = 11;
pub const ROWS: usize = 10;
pub const LETTER_LEDS: usize = COLS * ROWS; // 110
pub const CORNER_LEDS: usize = 4;
pub const TOTAL_LEDS: usize = LETTER_LEDS + CORNER_LEDS; // 114

/// 蛇形走线：第 0 行从左到右，第 1 行从右到左，依此类推。
pub const fn xy_to_index(row: usize, col: usize) -> usize {
    debug_assert!(row < ROWS && col < COLS);
    row * COLS + if row % 2 == 0 { col } else { COLS - 1 - col }
}

#[derive(Clone, Copy)]
struct Word {
    row: u8,
    col: u8,
    len: u8,
}
const fn w(row: u8, col: u8, len: u8) -> Word {
    Word { row, col, len }
}

const IT: Word = w(0, 0, 2);
const IS: Word = w(0, 3, 2);
const A: Word = w(1, 0, 1);
const QUARTER: Word = w(1, 2, 7);
const TWENTY: Word = w(2, 0, 6);
const M_FIVE: Word = w(2, 6, 4); // 分钟用 FIVE
const HALF: Word = w(3, 0, 4);
const M_TEN: Word = w(3, 5, 3); // 分钟用 TEN
const TO: Word = w(3, 9, 2);
const PAST: Word = w(4, 0, 4);
const OCLOCK: Word = w(9, 5, 6);

/// 按 1~12 索引的小时单词
const HOURS: [Word; 12] = [
    w(5, 0, 3),  // ONE
    w(6, 8, 3),  // TWO
    w(5, 6, 5),  // THREE
    w(6, 0, 4),  // FOUR
    w(6, 4, 4),  // FIVE
    w(5, 3, 3),  // SIX
    w(8, 0, 5),  // SEVEN
    w(7, 0, 5),  // EIGHT
    w(4, 7, 4),  // NINE
    w(9, 0, 3),  // TEN
    w(7, 5, 6),  // ELEVEN
    w(8, 5, 6),  // TWELVE
];

/// 一帧点亮状态
#[derive(Clone)]
pub struct Frame {
    pub lit: [bool; TOTAL_LEDS],
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            lit: [false; TOTAL_LEDS],
        }
    }
    fn word(&mut self, word: Word) {
        for i in 0..word.len as usize {
            self.lit[xy_to_index(word.row as usize, word.col as usize + i)] = true;
        }
    }
    fn dots(&mut self, n: u8) {
        for i in 0..(n as usize).min(CORNER_LEDS) {
            self.lit[LETTER_LEDS + i] = true;
        }
    }
    pub fn count(&self) -> usize {
        self.lit.iter().filter(|&&b| b).count()
    }
}

/// hour24: 0..=23, minute: 0..=59
///
/// 规则（QLOCKTWO 原版）：以 5 分钟为粒度向下取整，
/// 余数 1~4 分钟用角点表示；过半点用 TO + 下一小时。
pub fn time_to_frame(hour24: u8, minute: u8) -> Frame {
    debug_assert!(hour24 < 24 && minute < 60);
    let mut f = Frame::new();
    f.word(IT);
    f.word(IS);

    let step = minute / 5; // 0..=11
    let dots = minute % 5;

    // 显示用小时（12 小时制，1..=12）；TO 时取下一小时
    let mut h = hour24 % 12;
    if h == 0 {
        h = 12;
    }
    if step >= 7 {
        h = h % 12 + 1;
    }
    let hw = HOURS[(h - 1) as usize];

    match step {
        0 => {
            f.word(hw);
            f.word(OCLOCK);
        }
        1 => {
            f.word(M_FIVE);
            f.word(PAST);
            f.word(hw);
        }
        2 => {
            f.word(M_TEN);
            f.word(PAST);
            f.word(hw);
        }
        3 => {
            f.word(A);
            f.word(QUARTER);
            f.word(PAST);
            f.word(hw);
        }
        4 => {
            f.word(TWENTY);
            f.word(PAST);
            f.word(hw);
        }
        5 => {
            f.word(TWENTY);
            f.word(M_FIVE);
            f.word(PAST);
            f.word(hw);
        }
        6 => {
            f.word(HALF);
            f.word(PAST);
            f.word(hw);
        }
        7 => {
            f.word(TWENTY);
            f.word(M_FIVE);
            f.word(TO);
            f.word(hw);
        }
        8 => {
            f.word(TWENTY);
            f.word(TO);
            f.word(hw);
        }
        9 => {
            f.word(A);
            f.word(QUARTER);
            f.word(TO);
            f.word(hw);
        }
        10 => {
            f.word(M_TEN);
            f.word(TO);
            f.word(hw);
        }
        11 => {
            f.word(M_FIVE);
            f.word(TO);
            f.word(hw);
        }
        _ => unreachable!(),
    }

    f.dots(dots);
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word_lit(f: &Frame, word: Word) -> bool {
        (0..word.len as usize)
            .all(|i| f.lit[xy_to_index(word.row as usize, word.col as usize + i)])
    }

    #[test]
    fn serpentine_mapping() {
        assert_eq!(xy_to_index(0, 0), 0);
        assert_eq!(xy_to_index(0, 10), 10);
        assert_eq!(xy_to_index(1, 0), 21); // 第 1 行反向
        assert_eq!(xy_to_index(1, 10), 11);
        assert_eq!(xy_to_index(9, 0), 109); // 第 9 行同样反向
        assert_eq!(xy_to_index(9, 10), 99);
    }

    #[test]
    fn three_oclock() {
        let f = time_to_frame(15, 0); // IT IS THREE OCLOCK
        assert!(word_lit(&f, IT) && word_lit(&f, IS));
        assert!(word_lit(&f, HOURS[2])); // THREE
        assert!(word_lit(&f, OCLOCK));
        assert_eq!(f.count(), 2 + 2 + 5 + 6);
    }

    #[test]
    fn quarter_past_nine() {
        let f = time_to_frame(9, 15); // IT IS A QUARTER PAST NINE
        assert!(word_lit(&f, A) && word_lit(&f, QUARTER) && word_lit(&f, PAST));
        assert!(word_lit(&f, HOURS[8])); // NINE
    }

    #[test]
    fn half_past_noon() {
        let f = time_to_frame(12, 30); // IT IS HALF PAST TWELVE
        assert!(word_lit(&f, HALF) && word_lit(&f, PAST));
        assert!(word_lit(&f, HOURS[11])); // TWELVE
    }

    #[test]
    fn twentyfive_to_four() {
        let f = time_to_frame(15, 35); // IT IS TWENTY FIVE TO FOUR
        assert!(word_lit(&f, TWENTY) && word_lit(&f, M_FIVE) && word_lit(&f, TO));
        assert!(word_lit(&f, HOURS[3])); // FOUR
    }

    #[test]
    fn to_wraps_to_one() {
        let f = time_to_frame(12, 55); // IT IS FIVE TO ONE
        assert!(word_lit(&f, M_FIVE) && word_lit(&f, TO));
        assert!(word_lit(&f, HOURS[0])); // ONE
    }

    #[test]
    fn midnight_is_twelve() {
        let f = time_to_frame(0, 11); // IT IS TEN PAST TWELVE + 1 dot
        assert!(word_lit(&f, M_TEN) && word_lit(&f, PAST));
        assert!(word_lit(&f, HOURS[11])); // TWELVE
        assert!(f.lit[LETTER_LEDS] && !f.lit[LETTER_LEDS + 1]);
    }

    #[test]
    fn corner_dots_count() {
        for (m, expect) in [(0u8, 0usize), (1, 1), (2, 2), (3, 3), (4, 4)] {
            let f = time_to_frame(10, m);
            let dots = f.lit[LETTER_LEDS..].iter().filter(|&&b| b).count();
            assert_eq!(dots, expect, "minute {m}");
        }
    }

    #[test]
    fn all_day_never_panics_and_reasonable() {
        for h in 0..24 {
            for m in 0..60 {
                let f = time_to_frame(h, m);
                // 至少 IT IS + 一个内容词；最多不超过 28 颗（电源预算内）
                assert!(f.count() >= 5 && f.count() <= 28, "{h}:{m} -> {}", f.count());
            }
        }
    }
}
