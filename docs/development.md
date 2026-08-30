# 开发指南

固件开发的完整工作流：环境、编译、烧录、日志。实现细节的原理说明随代码注释维护（各文件头部及关键行），本文只讲流程与操作。

## 环境准备（一次性）

```bash
cargo install espup ldproxy espflash   # ldproxy=链接器包装, espflash=刷写工具
espup install                          # esp Rust 工具链（预编译 std）
source firmware/env.sh                 # 注入 IDF + esp 环境（每次新终端都要）
```

版本锁定清单见 [firmware/README.md](../firmware/README.md)（已验证组合，勿随意升级）。

## 硬件连接（双 USB 口分工）

| 口 | 芯片 | 用途 | Linux 设备 |
|---|---|---|---|
| UART 口 | CH9102F（`1a86:55d3`） | **日志主控制台**，复位不断线，监听常驻 | `/dev/ttyACM*`（编号会变） |
| USB-JTAG 口 | Espressif `303a:1001` | **烧录 / JTAG 调试**，复位会重新枚举 | `/dev/ttyACM*` |

WSL2 下两台设备都需 `usbipd bind` + `attach`；可直接用仓库自带的 Windows 脚本（bind/attach/detach/unbind 一键化，详见 [scripts/windows/README.md](../scripts/windows/README.md)）：

```powershell
.\usb-up.ps1        # 调试开始（含 auto-attach，断线自动重连）
.\usb-down.ps1      # 调试结束，归还设备
```

WSL2/usbipd 场景烧录必须**手动进下载模式**（板载自动复位序列经 USB/IP 转发不可靠）：

```
按住 BOOT → 短按 RESET → 松开 BOOT     # 进入下载模式，等待烧录
短按 RESET                              # 烧录完成后让固件跑起来（espflash 软复位可能落回下载模式）
```

## 日常命令

```bash
cd firmware && source env.sh

# 逻辑层单元测试（宿主机，改 wordclock-core 后必跑）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 编译（WiFi 凭据是编译期环境变量，改凭据必须重新 build）
WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo build -p word-clock

# 烧录（走 USB-JTAG 口；先完成上面的进下载模式操作）
espflash flash target/riscv32imac-esp-espidf/debug/word-clock --port /dev/ttyACM1

# 日志（UART 口，开发期常驻；dtr/rts 必须 False，原因见下）
python3 tools/uart_sniff.py
```

## 烧录时序注意事项

1. **烧录前停掉所有串口监听**：任何进程持有 USB-JTAG 口（即使 `rts=False`）都会令 espflash 同步失败。
2. **UART 口监听严禁拉 DTR/RTS**：CH9102F 的 DTR/RTS 接自动下载电路，拉住会把 IO0 摁低 → 芯片每次复位都进下载模式（`boot:0x48 DOWNLOAD`），表现为"烧完不跑、日志静默"。`tools/uart_sniff.py` 已正确处理。USB-Serial-JTAG 口恰好相反：不拉 DTR 不吐数据。
3. **日志从哪个口看**：主控制台是 UART0（sdkconfig `CONFIG_ESP_CONSOLE_UART_DEFAULT=y`）。Rust `log` 的输出**只进主控制台**；C 层日志（bootloader、驱动）ROM 阶段也走 UART。开发期只看 UART 口即可。
4. 控制台/栈大小等 sdkconfig 变更会触发 IDF 全量重生成（约 1 分钟），属正常。

## 踩坑记录速查

原理与修复位置随代码注释，这里只留索引：

| 症状 | 根因 | 详见 |
|---|---|---|
| WiFi 连接 `ESP_ERR_TIMEOUT` | `auth_method` 默认 None 连不上 WPA2 | `app/src/wifi.rs` 配置处注释 |
| 周期性 LED 自检循环闪 | app 返回 Err → IDF 重启循环 | `app/src/main.rs` 重试逻辑注释 |
| Stack protection fault | 主任务栈默认 3584 太小 | `app/sdkconfig.defaults` 注释 |
| 烧完不跑 / 日志静默 | 监听拉住 DTR/RTS → 摁进下载模式 | `tools/uart_sniff.py` 头部注释 |
| 日志里看不到 Rust `info!` | Rust log 只走主控制台 | `app/sdkconfig.defaults` 注释 |
| esp-idf-sys 找不到 sdkconfig.defaults | 需显式指定 | `firmware/.cargo/config.toml` 注释 |
