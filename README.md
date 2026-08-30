# Word Clock 字钟项目

QLOCKTWO-style English word clock — ESP32-C5 + WS2812B (114 LEDs), built with Rust (esp-idf-svc on ESP-IDF v5.5.1), parametric 3D-printed enclosure.

QLOCKTWO 风格英文字钟：11×10 字母阵 + 4 角分钟点，ESP32-C5 + WS2812B（60灯/米），结构件 3D 打印。

**当前状态：Phase A-MVP 实机已点亮**（自检 → WiFi → SNTP → 每分钟刷新全链路跑通，2026-08）。

## 目录结构

| 路径 | 内容 |
|---|---|
| `firmware/` | Rust 固件（esp-idf-svc / std on IDF v5.5.1）+ 可宿主测试的 `wordclock-core` |
| `enclosure/` | 3D 打印结构件（OpenSCAD 参数化源码） |
| `docs/` | 项目文档（见下） |
| `AGENTS.md` | AI agent 快速上手指南 |

## 文档导航

| 文档 | 内容 |
|---|---|
| [docs/roadmap.md](docs/roadmap.md) | 路线图与当前进度 |
| [docs/development.md](docs/development.md) | 开发指南：环境搭建、编译烧录、日志调试、踩坑记录 |
| [docs/wiring.md](docs/wiring.md) | 接线图：供电/数据/灯链布局/验证阶段接线 |
| [docs/bom.md](docs/bom.md) | 物料清单与功率预算 |
| [firmware/README.md](firmware/README.md) | 固件模块结构与版本锁定 |
| [enclosure/README.md](enclosure/README.md) | 结构件参数与打印说明 |

## 快速开始

```bash
cd firmware && source env.sh                     # 环境（一次性）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu   # 逻辑层测试（宿主机）
```

编译刷写与日志调试见 [docs/development.md](docs/development.md)。

## License

[MIT](LICENSE)
