# AGENTS.md — AI Agent 快速上手指南

面向在本仓库工作的 AI agent / 新成员。读完本文件即可安全开工，细节查链接文档。

## 项目一句话

QLOCKTWO 风格英文字钟：ESP32-C5 + WS2812B（114 灯），Rust 固件（esp-idf-svc / std on IDF v5.5.1）。**MVP 已实机点亮**。

## 仓库结构

- `firmware/app/` — 固件主程序（启动流程见 `main.rs` 模块注释）
- `firmware/wordclock-core/` — 纯逻辑（时间→点阵），no_std，宿主机可测
- `firmware/vendor/` — vendored ws2812 驱动（hal 0.46 补丁版，勿升级）
- `docs/` — roadmap / development / wiring / bom
- `enclosure/` — OpenSCAD 结构件

## 关键命令

```bash
cd firmware && source env.sh                 # 每次新终端必须先执行

# 逻辑层测试（改 wordclock-core 后必跑）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 编译（WiFi 凭据是编译期环境变量！改凭据必须重新 build）
WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo build -p word-clock

# 烧录（走 USB-JTAG 口 /dev/ttyACM1，需先手动进下载模式）
espflash flash target/riscv32imac-esp-espidf/debug/word-clock --port /dev/ttyACM1

# 日志（UART 口，常驻）
python3 tools/uart_sniff.py
```

## 硬件操作铁律

1. **双 USB 口**：UART 口（CH9102F）= 日志；USB-JTAG 口 = 烧录/调试。两线常驻。
2. **烧录需手动进下载模式**：按住 BOOT → 短按 RESET → 松开 BOOT（espflash 自动复位经 usbipd 不可靠）。
3. **烧录前停掉 uart_sniff**：串口被占用会导致 espflash 同步失败。
4. **uart_sniff 的 dtr/rts 必须为 False**：拉住会把芯片摁进下载模式（`tools/uart_sniff.py` 已正确处理，勿改）。
5. **WSL2 + usbipd 环境**：设备复位会掉线；Windows 侧用 `scripts/windows/usb-up.ps1` 一键 bind + auto-attach（结束用 `usb-down.ps1`）；ttyACM 编号会漂移。

## 代码级红线（改动前先读对应注释）

- `wifi.rs`：`auth_method` 必须显式设 WPA2，默认 None 连不上任何 AP
- `main.rs`：任何阶段失败必须重试，**绝不让 main 返回 Err**（返回 = 整机重启循环）
- `app/sdkconfig.defaults`：主任务栈 16384（默认值会栈溢出）；主控制台是 UART0
- 版本锁定组合见 `firmware/README.md`，勿随意升级 esp-idf-svc/hal/IDF

## 验证顺序（改完代码后）

1. `cargo test -p wordclock-core --target x86_64-unknown-linux-gnu`（宿主机）
2. `cargo build -p word-clock`（目标机，确认零警告）
3. 逻辑正确性可在宿主机验证的，优先宿主机验证再上板

## 当前状态与下一步

见 [docs/roadmap.md](docs/roadmap.md)。MVP 已点亮（2026-08），下一步是 114 灯全量平铺实测 → Phase A-Full（Web 配置页等）。
