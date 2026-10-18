
use anyhow::Result;
use esp_idf_svc::sntp::{EspSntp, SntpConf, SyncStatus};
use log::info;

/// 启动 SNTP（国内 NTP 服务器优先）并设置时区 UTC+8（中国无夏令时）
pub fn start_sntp() -> Result<EspSntp<'static>> {
    // 注意: IDF 默认 SNTP_MAX_SERVERS=1，只能设一个服务器
    let mut conf = SntpConf::default();
    conf.servers[0] = "ntp.aliyun.com";
    let sntp = EspSntp::new(&conf)?;

    // newlib 时区：POSIX 格式 "CST-8" 表示 UTC+8
    std::env::set_var("TZ", "CST-8");
    unsafe { esp_idf_svc::sys::tzset() };

    info!("sntp started");
    Ok(sntp)
}

/// 等待 NTP 同步完成。
/// 每 5s 汇报一次状态：WiFi 通但 NTP 不通（DNS/UDP 123 被拦）时避免静默卡死无法排查。
/// 非阻塞版本：调用方自己循环，这样等待期间可以播动画
pub fn is_synced(sntp: &EspSntp) -> bool {
    matches!(sntp.get_sync_status(), SyncStatus::Completed)
}

/// 运行期设置时区（POSIX 串，如 "CST-8"）。管理页改时区走这里。
pub fn set_tz(tz: &str) {
    if tz.is_empty() {
        return;
    }
    std::env::set_var("TZ", tz);
    unsafe { esp_idf_svc::sys::tzset() };
    info!("time_svc: TZ = {tz}");
}

/// 本地 (hour, minute)，走 newlib localtime_r（已应用 TZ）
pub fn local_hm() -> (u8, u8) {
    unsafe {
        let now = esp_idf_svc::sys::time(core::ptr::null_mut());
        let mut tm: esp_idf_svc::sys::tm = core::mem::zeroed();
        esp_idf_svc::sys::localtime_r(&now, &mut tm);
        (tm.tm_hour as u8, tm.tm_min as u8)
    }
}
