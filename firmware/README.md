# word-clock-firmware

字钟固件（路线 A：Rust std on ESP-IDF）。

## 结构

```
firmware/
├── app/              # ESP32-C5 固件 bin（esp-idf-svc）
│   └── src/
│       ├── main.rs       # 启动流程：自检 → WiFi → SNTP → 显示循环
│       ├── leddrv.rs     # WS2812B RMT 驱动封装（亮度、自检、整帧刷新）
│       ├── wifi.rs       # STA 连接（凭据用编译期环境变量传入）
│       └── time_svc.rs   # SNTP + 时区（UTC+8）
└── wordclock-core/   # 纯逻辑：时间 → LED 点阵。无依赖、no_std，
                        # 可在宿主机 cargo test，未来路线B（esp-hal）直接复用
```

## 环境准备（一次性）

```bash
cargo install espup ldproxy espflash   # ldproxy=链接器包装, espflash=刷写工具
espup install                          # 装 esp Rust 工具链（预编译 std）
source env.sh                          # 注入 IDF + esp 工具链环境
```

## 版本锁定（已验证的组合，勿随意升級）

- `esp-idf-svc 0.51` / `esp-idf-hal 0.45` + `ws2812-esp32-rmt-driver 0.13`
- **IDF v5.4.2**（esp-idf-sys managed 安装在 `.embuild/`）：v5.5 与 hal 0.45 API 不兼容；v5.2 不支持 C5
- esp Rust 工具链 1.95.0（nightly fork，`build-std` 编译 std）

## 网络加速（国内必做）

构建命令需要两个环境变量（git clone 与 IDF 工具下载加速）：

```bash
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0='url.https://gh-proxy.com/https://github.com/.insteadOf' \
GIT_CONFIG_VALUE_0='https://github.com/' \
IDF_GITHUB_ASSETS='dl.espressif.com/github_assets' \
cargo build -p word-clock
```

> eim 安装的 IDF v6.0.2（`~/.espressif`）与本项目无关；其 python env 缺失需跑 `install.sh` 补齐后才可用于其他 IDF 项目。

## 常用命令

```bash
source env.sh

# 逻辑层测试（宿主机，强烈推荐先跑）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 编译
cargo build -p word-clock

# 刷写 + 串口监视（板子在 /dev/ttyACM0；WiFi 凭据以编译期环境变量传入）
WIFI_SSID=你的SSID WIFI_PASSWORD=你的密码 cargo run -p word-clock
```

## 待办（MVP 之后）

- [ ] 首次编译时按实际 crate API 微调 `leddrv.rs` / `wifi.rs` / `time_svc.rs`
- [ ] Web 配置页（esp-idf-svc `EspHttpServer`）：颜色/亮度/WiFi
- [ ] 光敏电阻自动亮度、夜间模式
- [ ] OTA
- [ ] 路线 B：esp-hal + embassy 重写（复用 wordclock-core）
