//! 运行期可配置项（NVS 持久化）。
//!
//! 与 `creds.rs` 分开是因为生命周期不同：WiFi 凭据由配网流程写，设置在运行期
//! 由管理页面写。两者都放在同一个 NVS 命名空间 `wc` 里。
//!
//! 取值都有上下界钳制：写坏的值不能让灯变黑或让时钟显示错乱。

use anyhow::Result;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use log::info;

const NAMESPACE: &str = "wc";
const KEY_BRIGHT: &str = "bright";
const KEY_COLOR: &str = "color";
const KEY_TZ: &str = "tz";
const KEY_ADMIN: &str = "admin";

/// 默认时区（中国无夏令时）
pub const TZ_DEFAULT: &str = "CST-8";
/// 时区字符串最长 16 字节，够放 "CST-8" / "UTC" / "EST5EDT"
const TZ_MAX: usize = 16;
/// 管理口令最长 32 字节（空串 = 不鉴权）
pub const ADMIN_MAX: usize = 32;

/// 亮度档位 → 分压系数（越大越暗）。索引从 1 开始，0 视为默认。
const BRIGHTNESS_DIV: [u8; 5] = [2, 8, 4, 2, 1];

/// 色温预设
// 色温刻度（2026-10-18 按实机观感校准）：
// 旧值 (255,190,110) 在实物上被读成"正白"，说明整条刻度偏冷，故整体下压：
//   暖白 = 旧的琥珀附近      正白 = 旧的暖白       琥珀 = 更深的琥珀
const COLORS: [(u8, u8, u8); 3] = [
    (255, 165, 80), // 0 暖白
    (255, 200, 130), // 1 正白
    (255, 120, 30),  // 2 琥珀
];

#[derive(Clone, Debug)]
pub struct Settings {
    /// 1..=4，越大越亮
    pub brightness: u8,
    /// 0..=2 色温预设
    pub color: u8,
    /// POSIX 时区串，交给 newlib
    pub tz: String,
    /// 管理页口令；空 = 局域网内无需鉴权
    pub admin: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            brightness: 3,
            color: 0,
            tz: TZ_DEFAULT.to_string(),
            admin: String::new(),
        }
    }
}

impl Settings {
    /// 亮度对应的分压系数
    pub fn brightness_div(&self) -> u8 {
        BRIGHTNESS_DIV[(self.brightness.min(4)) as usize]
    }

    /// 色温对应的 RGB
    pub fn color_rgb(&self) -> (u8, u8, u8) {
        COLORS[(self.color.min(2)) as usize]
    }

    /// 钳制到合法区间（防止 NVS 里写出越界值）
    pub fn sanitize(&mut self) {
        self.brightness = self.brightness.clamp(1, 4);
        self.color = self.color.min(2);
        if self.tz.is_empty() || self.tz.len() >= TZ_MAX {
            self.tz = TZ_DEFAULT.to_string();
        }
        if self.admin.len() > ADMIN_MAX {
            self.admin.truncate(ADMIN_MAX);
        }
    }
}

fn open(nvs: &EspDefaultNvsPartition) -> Result<EspNvs<NvsDefault>> {
    Ok(EspNvs::new(nvs.clone(), NAMESPACE, true)?)
}

pub fn load(nvs: &EspDefaultNvsPartition) -> Settings {
    let mut s = Settings::default();
    let Ok(h) = open(nvs) else { return s };
    if let Ok(Some(v)) = h.get_u8(KEY_BRIGHT) {
        s.brightness = v;
    }
    if let Ok(Some(v)) = h.get_u8(KEY_COLOR) {
        s.color = v;
    }
    let mut buf = [0u8; TZ_MAX];
    if let Ok(Some(v)) = h.get_str(KEY_TZ, &mut buf) {
        s.tz = v.to_string();
    }
    let mut abuf = [0u8; ADMIN_MAX + 8];
    if let Ok(Some(v)) = h.get_str(KEY_ADMIN, &mut abuf) {
        s.admin = v.to_string();
    }
    s.sanitize();
    s
}

pub fn save(nvs: &EspDefaultNvsPartition, s: &Settings) -> Result<()> {
    let mut s = s.clone();
    s.sanitize();
    let h = open(nvs)?;
    h.set_u8(KEY_BRIGHT, s.brightness)?;
    h.set_u8(KEY_COLOR, s.color)?;
    h.set_str(KEY_TZ, &s.tz)?;
    h.set_str(KEY_ADMIN, &s.admin)?;
    info!(
        "settings: 已保存 brightness={} color={} tz={} admin={}",
        s.brightness,
        s.color,
        s.tz,
        if s.admin.is_empty() { "（无）" } else { "（已设置）" }
    );
    Ok(())
}

/// 恢复出厂：设置项 + WiFi 凭据全部清空
pub fn factory_reset(nvs: &EspDefaultNvsPartition) -> Result<()> {
    let h = open(nvs)?;
    for k in [KEY_BRIGHT, KEY_COLOR, KEY_TZ, KEY_ADMIN] {
        let _ = h.remove(k);
    }
    crate::creds::clear(nvs)?;
    info!("settings: 已恢复出厂（设置 + WiFi 凭据已清除）");
    Ok(())
}
