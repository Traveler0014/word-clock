//! WiFi 凭据持久化（NVS）。
//!
//! 为什么要有这个模块：
//! 早前凭据是**编译期环境变量**（`WIFI_SSID=xxx cargo build`），导致
//! 1) 改密码必须重新编译烧录；2) 忘记带凭据编译会烧出一个连不上网的固件。
//!
//! 现在凭据**只**存在 NVS（命名空间 `wc`），由配网页面写入，重启即生效。
//! 固件二进制里不含任何 WiFi 密码，构建也不需要任何凭据环境变量。

use anyhow::Result;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use log::info;

const NAMESPACE: &str = "wc";
const KEY_SSID: &str = "ssid";
const KEY_PASS: &str = "pass";
/// 连续"未正常启动"次数：见 `bump_boot_streak()` 的说明
const KEY_STREAK: &str = "streak";

/// 最大长度按 WiFi 规范：SSID ≤ 32 字节，密码 ≤ 64 字节
const SSID_MAX: usize = 32;
const PASS_MAX: usize = 64;

#[derive(Clone, Debug)]
pub struct Creds {
    pub ssid: String,
    pub pass: String,
}

impl Creds {
    pub fn new(ssid: impl Into<String>, pass: impl Into<String>) -> Self {
        Self {
            ssid: ssid.into(),
            pass: pass.into(),
        }
    }

    /// 基本合法性检查（空 SSID、超长直接拒绝，避免写进 NVS 后再也连不上）
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.ssid.is_empty(), "SSID 不能为空");
        anyhow::ensure!(self.ssid.len() <= SSID_MAX, "SSID 超过 {SSID_MAX} 字节");
        anyhow::ensure!(self.pass.len() <= PASS_MAX, "密码超过 {PASS_MAX} 字节");
        Ok(())
    }
}

fn open(nvs: &EspDefaultNvsPartition) -> Result<EspNvs<NvsDefault>> {
    Ok(EspNvs::new(nvs.clone(), NAMESPACE, true)?)
}

/// 从 NVS 读出凭据；没有（或读失败）返回 None
pub fn load(nvs: &EspDefaultNvsPartition) -> Option<Creds> {
    let handle = open(nvs).ok()?;
    let mut ssid_buf = [0u8; SSID_MAX];
    let ssid = handle.get_str(KEY_SSID, &mut ssid_buf).ok()??.to_string();
    if ssid.is_empty() {
        return None;
    }
    let mut pass_buf = [0u8; PASS_MAX];
    let pass = handle
        .get_str(KEY_PASS, &mut pass_buf)
        .ok()
        .flatten()
        .unwrap_or("")
        .to_string();
    Some(Creds { ssid, pass })
}

/// 写入 NVS（覆盖旧值）
pub fn save(nvs: &EspDefaultNvsPartition, creds: &Creds) -> Result<()> {
    creds.validate()?;
    let handle = open(nvs)?;
    handle.set_str(KEY_SSID, &creds.ssid)?;
    handle.set_str(KEY_PASS, &creds.pass)?;
    info!(
        "creds: 已保存到 NVS (ssid=\"{}\", pass 长度={})",
        creds.ssid,
        creds.pass.len()
    );
    Ok(())
}

/// 清除凭据（下一次启动就会进配网模式）
#[allow(dead_code)] // 目前只供手工调试用
pub fn clear(nvs: &EspDefaultNvsPartition) -> Result<()> {
    let handle = open(nvs)?;
    handle.remove(KEY_SSID)?;
    handle.remove(KEY_PASS)?;
    Ok(())
}

/// 恢复出厂手势用：连续"未正常启动"次数 +1。
///
/// 无按键设备的通用做法：短时反复上电（每次都没跑到正常显示）就认为用户想
/// 强制重新配网。判据不是真实时间间隔（断电后无法测量），而是
/// "连续 N 次重启都没到达健康状态"——健康状态由主流程在 WiFi 连接 + NTP 同步
/// 之后调用 `clear_boot_streak()` 确认。
pub fn bump_boot_streak(nvs: &EspDefaultNvsPartition) -> u8 {
    let Ok(handle) = open(nvs) else { return 0 };
    let prev = handle.get_u8(KEY_STREAK).ok().flatten().unwrap_or(0);
    let next = prev.saturating_add(1).min(10);
    let _ = handle.set_u8(KEY_STREAK, next);
    next
}

/// 到达健康状态（或已进入配网模式）时清零计数
pub fn clear_boot_streak(nvs: &EspDefaultNvsPartition) {
    if let Ok(handle) = open(nvs) {
        let _ = handle.set_u8(KEY_STREAK, 0);
    }
}
