# Word Clock 字钟项目工作区

QLOCKTWO 风格英文字钟：11×10 字母阵 + 4 角分钟点，ESP32-C5 + WS2812B（60灯/米），结构件 3D 打印。

本目录是工作区根（不单独建仓），各模块拆分为独立仓库：

| 目录 | 仓库 | 内容 | 状态 |
|---|---|---|---|
| `firmware/` | word-clock-firmware | Rust 固件。路线A MVP（esp-idf-svc / std on IDF）；含可宿主端测试的 `wordclock-core` | 🚧 搭建中 |
| `enclosure/` | word-clock-enclosure | 3D 打印结构件（OpenSCAD 参数化源码 + 打印说明） | 🚧 搭建中 |
| `docs/` | word-clock-docs | BOM、接线图、路线图 | 🚧 搭建中 |

## 硬件

- ESP32-C5-DevKit（内置 USB-Serial-JTAG，`/dev/ttyACM0`）
- WS2812B 60灯/米 × 114 颗（110 字母 + 4 角点），5V/4A 供电
- 字距 16.67mm → 字面 183×167mm，整机约 215×200mm（250³ 热床一体打印）

## 快速开始

```bash
# 1. 一次性环境（Rust esp 工具链 + IDF 环境）
cd firmware && source env.sh

# 2. 逻辑层单元测试（宿主机，无需硬件）
cd firmware && cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 3. 编译 + 刷写 + 监视
WIFI_SSID=xxx WIFI_PASSWORD=yyy cargo run -p word-clock
```

详见各子目录 README。路线图见 `docs/roadmap.md`。
