# word-clock-firmware

字钟固件（路线 A：Rust std on ESP-IDF）。**MVP 已实机点亮**（自检 → WiFi → SNTP → 每分钟刷新）。

## 结构

```
firmware/
├── app/              # ESP32-C5 固件 bin（esp-idf-svc）
│   ├── src/
│   │   ├── main.rs       # 启动流程：自检 → 延迟 → WiFi → SNTP → 显示循环
│   │   ├── leddrv.rs     # WS2812B RMT 驱动封装（GPIO4，1/4 亮度限幅，自检，整帧刷新）
│   │   ├── wifi.rs       # STA 连接（WPA2、扫描确认、无限重试）
│   │   └── time_svc.rs   # SNTP（ntp.aliyun.com）+ 时区 UTC+8
│   └── sdkconfig.defaults    # UART0 主控制台、主任务栈 16K
├── wordclock-core/   # 纯逻辑：时间 → LED 点阵。no_std，宿主机 cargo test（9 通过），
│                     # 未来路线B（esp-hal）直接复用
├── tools/uart_sniff.py   # UART 日志监听（自动重连；详见下文"为什么 dtr/rts 必须为 False"）
├── vendor/ws2812-esp32-rmt-driver/  # vendored 驱动（适配 hal 0.46 的补丁版）
└── env.sh            # 注入 IDF + esp 工具链环境
```

## 环境准备（一次性）

```bash
cargo install espup ldproxy espflash   # ldproxy=链接器包装, espflash=刷写工具
espup install                          # esp Rust 工具链（预编译 std）
source env.sh                          # 注入 IDF + esp 环境
```

## 版本锁定（已验证组合，勿随意升级）

- `esp-idf-svc 0.52` / `esp-idf-hal 0.46` + **vendored** `ws2812-esp32-rmt-driver`（hal 0.46 补丁版）
- **IDF v5.5.1**（esp-idf-sys managed 安装在 `firmware/.embuild/`）
  - v6.0.2：esp-idf-sys 0.36 未适配，且 eim 的 python 环境残缺
  - hal 0.45：对 esp32c5 支持不全且与 v5.4+ API 漂移
- esp Rust 工具链 1.95.0（nightly fork，`build-std` 编译 std）
- `.cargo/config.toml` 中 `ESP_IDF_SDKCONFIG_DEFAULTS` 必须显式指定（sys crate 不会自动发现 app/sdkconfig.defaults）

## 双 USB 口分工（重要）

| 口 | 芯片 | 用途 |
|---|---|---|
| UART 口 | CH9102F（`1a86:55d3`，`/dev/ttyACM0`） | **日志主控制台**（CONFIG_ESP_CONSOLE_UART_DEFAULT）。复位不断线、不重新枚举，监听常驻 |
| USB-JTAG 口 | Espressif `303a:1001`（`/dev/ttyACM1`） | **烧录 / JTAG 调试**。复位会重新枚举（usbipd 场景必掉） |

烧录流程（USB-JTAG 口自动复位进下载模式经 usbipd 常失败，需手动）：

```bash
# 1. 板上操作：按住 BOOT → 短按 RESET → 松开 BOOT
# 2. 刷写（WiFi 凭据为编译期环境变量，改凭据后需重新 build）
WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo build -p word-clock
espflash flash target/riscv32imac-esp-espidf/debug/word-clock --port /dev/ttyACM1
# 3. 烧完 espflash 软复位可能落回下载模式：按一下 RESET（不按 BOOT）即运行
```

日志监听（UART 口，开发期常驻）：

```bash
python3 tools/uart_sniff.py
```

> ⚠️ **uart_sniff 必须 `dtr=False, rts=False`**（当前实现已如此）：CH9102F 的 DTR/RTS 接有自动下载电路，拉住 RTS/DTR 会把 IO0 摁低，芯片每次复位都进下载模式——表现为"烧完不跑、日志静默"，且会干扰 espflash 同步。USB-Serial-JTAG 口则相反：不拉 DTR 不吐数据。

## 踩坑记录（实测）

1. **WiFi 连接 ESP_ERR_TIMEOUT**：`ClientConfiguration` 的 `auth_method` 默认 `None`，密码再对也连不上 WPA2 AP。显式设 `AuthMethod::WPAWPA2Personal`。
2. **主循环退出 = 重启循环**：app 返回 Err 后 IDF 重启，现象是周期性 LED 自检循环闪。所有阶段失败必须重试而非退出。
3. **主任务栈**：默认 3584 触发 Stack protection fault，sdkconfig 设 16384。
4. **烧录时串口监听必须断开**：任何进程持有 USB-JTAG 口（即使 rts=False）都会导致 espflash 同步失败。
5. **app 的 Rust log 只进主控制台**：C 层日志两路都走，Rust `info!` 只走 UART0——这也是切主控制台的原因。

## 常用命令

```bash
source env.sh

# 逻辑层测试（宿主机）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 编译 / 刷写（凭据见上文）
WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo build -p word-clock
espflash flash target/riscv32imac-esp-espidf/debug/word-clock --port /dev/ttyACM1
```

## 待办（MVP 之后）

- [ ] 全套 114 灯平铺实测（当前 4 灯样本已验证：数据链路 / 级联 / 整帧刷新）
- [ ] Web 配置页（esp-idf-svc `EspHttpServer`）：颜色/亮度/WiFi
- [ ] 光敏电阻自动亮度、夜间模式
- [ ] OTA
- [ ] 去掉 main.rs 的 10 秒启动延迟（串口监听惯用，量产前移除）
- [ ] 路线 B：esp-hal + embassy 重写（复用 wordclock-core）
