# word-clock-firmware

字钟固件（路线 A：Rust std on ESP-IDF）。**MVP 已实机点亮**。开发流程见 [docs/development.md](../docs/development.md)。

## 结构

```
firmware/
├── app/              # ESP32-C5 固件 bin（esp-idf-svc）
│   ├── src/
│   │   ├── main.rs       # 启动流程：自检 → 延迟 → WiFi → SNTP → 显示循环
│   │   ├── leddrv.rs     # WS2812B RMT 驱动封装（GPIO4，1/4 亮度限幅，自检，整帧刷新）
│   │   ├── wifi.rs       # STA 连接（WPA2、扫描确认、无限重试）
│   │   └── time_svc.rs   # SNTP（ntp.aliyun.com）+ 时区 UTC+8
│   └── sdkconfig.defaults    # UART0 主控制台、主任务栈 16K（各项有注释）
├── wordclock-core/   # 纯逻辑：时间 → LED 点阵。no_std，宿主机可测，路线B 直接复用
├── tools/uart_sniff.py   # UART 日志监听（自动重连）
├── vendor/ws2812-esp32-rmt-driver/  # vendored 驱动（适配 hal 0.46 的补丁版）
├── .cargo/config.toml        # 目标芯片 / IDF 版本 / sdkconfig 指定
└── env.sh            # 注入 IDF + esp 工具链环境
```

> 各源文件的模块级文档（`//!`）与关键行注释承载实现细节，**改代码时同步维护注释**，不要把细节搬进 README。

## 版本锁定（已验证组合，勿随意升级）

- `esp-idf-svc 0.52` / `esp-idf-hal 0.46` + **vendored** `ws2812-esp32-rmt-driver`（hal 0.46 补丁版，legacy rmt api）
- **IDF v5.5.1**（esp-idf-sys managed 安装在 `firmware/.embuild/`）
  - v6.0.2：esp-idf-sys 0.36 未适配，且 eim 的 python 环境残缺
  - hal 0.45：对 esp32c5 支持不全且与 v5.4+ API 漂移
- esp Rust 工具链 1.95.0（nightly fork，`build-std` 编译 std）

## 网络

构建（凭据不进固件，首次上电经配网页面写入 NVS，详见 docs/development.md）：

```bash
cargo build -p word-clock   # 不需要 WiFi 凭据：首次上电用配网页面写入
```

国内构建加速（IDF 组件/GitHub 资产代理）如需，参考：

```bash
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0='url.https://gh-proxy.com/https://github.com/.insteadOf' \
GIT_CONFIG_VALUE_0='https://github.com/' \
IDF_GITHUB_ASSETS='dl.espressif.com/github_assets' \
cargo build -p word-clock
```
