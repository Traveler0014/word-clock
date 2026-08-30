# Word Clock 字钟项目

QLOCKTWO 风格英文字钟：11×10 字母阵 + 4 角分钟点，ESP32-C5 + WS2812B（60灯/米），结构件 3D 打印。

**当前状态：Phase A-MVP 实机已点亮** —— 自检 → WiFi → SNTP → 每分钟刷新全链路跑通（2026-08）。

## 仓库结构（monorepo）

| 目录 | 内容 | 说明 |
|---|---|---|
| `firmware/` | Rust 固件 | 路线A：esp-idf-svc / std on IDF v5.5.1；含可宿主测试的 `wordclock-core` |
| `enclosure/` | 3D 打印结构件 | OpenSCAD 参数化源码（格栅 / 背板 / 面板框） |
| `docs/` | 文档 | BOM、接线图、路线图 |

> 历史沿革：曾为三仓库工作区，2026-08 合并为 monorepo（subtree 合入，提交历史完整保留；旧仓 bundle 备份在 `../word-clock-history-backup/`）。

## 硬件

- ESP32-C5-DevKitC（双 USB 口）：
  - **UART 口（CH9102F）**：日志主控制台，复位不断线，开发常驻
  - **USB-JTAG 口（Espressif 303a:1001）**：烧录 / JTAG 调试
- WS2812B 60灯/米 × 114 颗（110 字母 + 4 角点），5V/4A 独立供电
- LED 数据脚 **GPIO4**（RMT channel0），字距 16.67mm → 字面 183×167mm，整机约 215×200mm（250³ 热床一体打印）

接线图见 `docs/wiring.md`，BOM 见 `docs/bom.md`。

## 快速开始

```bash
# 1. 一次性环境（Rust esp 工具链 + IDF 环境）
cd firmware && source env.sh

# 2. 逻辑层单元测试（宿主机，无需硬件）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 3. 编译 + 刷写（WiFi 凭据为编译期环境变量）
WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo build -p word-clock
espflash flash firmware/target/riscv32imac-esp-espidf/debug/word-clock --port /dev/ttyACM1

# 4. 日志（UART 口，常驻）
python3 firmware/tools/uart_sniff.py
```

> 开发调试的完整流程与踩坑记录见 `firmware/README.md`；烧录需手动进下载模式（按住 BOOT → 短按 RESET → 松开 BOOT）。
