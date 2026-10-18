# AGENTS.md — AI Agent 快速上手指南

面向在本仓库工作的 AI agent / 新成员。读完本文件即可安全开工，细节查链接文档。

## 项目一句话

QLOCKTWO 风格英文字钟：ESP32-C5 + WS2812B（114 灯），Rust 固件（esp-idf-svc / std on IDF v5.5.1）。**MVP 已实机点亮**。

## 仓库结构

- `firmware/app/` — 固件主程序（启动流程见 `main.rs` 模块注释）
- `firmware/wordclock-core/` — 纯逻辑（时间→点阵），no_std，宿主机可测
- `firmware/vendor/` — vendored ws2812 驱动（hal 0.46 补丁版，勿升级）
- `docs/` — roadmap / development / wiring / bom
- `firmware/app/src/` — `main.rs`（状态机）`wifi.rs`（连接原语）`creds.rs`（NVS 凭据）
  `settings.rs`（亮度/色温/时区/管理口令）`web.rs` + `web_setup.html` + `web_admin.html`（内建 Web）
  `leddrv.rs`（legacy RMT + 预编码）`time_svc.rs`（SNTP + TZ）
- `tools/` — 宿主机工具：`uart_sniff.py`（日志）、`frame_preview.py`（点阵预览/映射反查）、`debug_web.py`+`debug_web.html`（浏览器点选实测亮灯）
- `scripts/windows/` — `usb-up/down.ps1`（usbipd bind+attach）、`open-debug.ps1`（用正确地址打开调试页）、`wsl-port-forward.ps1`（portproxy 备用，本机无效）
- `enclosure/` — OpenSCAD 结构件

## 关键命令

```bash
cd firmware && source env.sh                 # 每次新终端必须先执行

# 逻辑层测试（改 wordclock-core 后必跑）
cargo test -p wordclock-core --target x86_64-unknown-linux-gnu

# 编译（**不需要任何 WiFi 环境变量**：凭据只存在 NVS，由配网页面写入）
cargo build -p word-clock --release

# 配网（NVS 无凭据或连不上时自动进入）
#   手机连热点 WordClock-XXXXXX（开放）→ 浏览器打开日志里打印的 AP IP（本机实测 192.168.71.1）
#   → 选网络 / 输密码（可选设管理口令）→ 设备重启后生效
# 恢复出厂：连续 3 次上电都没跑到健康状态（WiFi+NTP 成功）即清空凭据与设置
# 运行期管理页：http://<设备 IP>/（改亮度/色温/时区/口令、更换网络、恢复出厂）

# 调试固件：不连 WiFi，只显示静态标尺（每行左红/右绿/中暗蓝 + 四角白）
# 用于验证灯链映射与信号完整性，不依赖网络、不依赖时间
cargo build -p word-clock --release --features diag

# 烧录（USB-JTAG 口，按 VID:PID 找口，ttyACM 编号会漂移）
#   303a:1001 = USB-JTAG（烧录/调试）  1a86:55d3 = UART（日志，CH343/CH9102F）
ls -l /dev/ttyACM*        # 用 udevadm info -q property -n <port> | grep ID_VENDOR_ID 核对
ESPFLASH_PORT=/dev/ttyACM0   # 通常 ACM0=JTAG、ACM1=UART，但务必核对后再烧
espflash flash target/riscv32imac-esp-espidf/debug/word-clock --port $ESPFLASH_PORT

# 日志（UART 口，常驻；按 VID:PID 自动找口）
python3 tools/uart_sniff.py              # --log uart.log / --grep word_clock

# 点阵调试（宿主机，零依赖）
python3 tools/frame_preview.py --at 18:26            # 打印该时刻应亮的字母阵
python3 tools/frame_preview.py --decode               # 粘贴实物观测 → 反查走线假设
python3 tools/debug_web.py                            # 浏览器点选实测亮灯（.debug/obs-*.json）
```

## 硬件操作铁律

1. **双 USB 口**：UART 口（CH9102F）= 日志；USB-JTAG 口 = 烧录/调试。两线常驻。
2. **烧录**：espflash 走 USB-JTAG 口即可，2026-10 实测**自动复位可用，不需要手动 BOOT+RESET**（早前经 usbipd 不可靠的说法已不成立；真进不去再手动：按住 BOOT → 短按 RESET → 松开）。端口按 VID:PID 认，别硬编码编号。
3. **烧录前停掉 uart_sniff**：串口被占用会导致 espflash 同步失败。
4. **uart_sniff 的 dtr/rts 必须为 False**：拉住会把芯片摁进下载模式（`tools/uart_sniff.py` 已正确处理，勿改）。
5. **WSL2 + usbipd 环境**：设备复位会掉线；Windows 侧用 `scripts/windows/usb-up.ps1` 一键 bind + auto-attach（结束用 `usb-down.ps1`）。
   - **ttyACM 编号会漂移**：别硬编码 `/dev/ttyACM1`，用 VID:PID 区分（见上）。2026-10 实测组合为 `ttyACM0`=USB-JTAG、`ttyACM1`=UART，但这只是当次枚举结果。
   - 本机存在 `USBPcap` / `hrdevmon` 过滤驱动，**裸 `usbipd bind` 必失败**，必须 `bind --force`（`usb-up.ps1` 已自动回退）。
   - WSL2 的 localhost 转发在本机不通，调试 Web 页要用 `http://<WSL IP>:端口`；直接跑 `scripts\windows\open-debug.ps1` 自动取 IP 并打开。
   - **`.ps1` 必须带 UTF-8 BOM + CRLF**：Windows PowerShell 5.1 会把无 BOM 的 UTF-8 当 GBK 读，中文会把引号吃掉导致整脚本解析失败（已踩过）。
6. **Web handler 里绝不直接操作 WiFi / 写 NVS**：esp_http_server 是单事件任务，
   阻塞它会卡死所有请求。handler 只改 `web::Shared`，动作交给主循环（见 `web.rs` 注释）。
7. **数据链 3.3V 直驱只在小样本验证过**：114 颗全链必须上 74AHCT125（见 `docs/wiring.md`）。电平不足的典型现象是偶发丢位 → 整行右移一位、单词缺失字母。

## 代码级红线（改动前先读对应注释）

- `wifi.rs`：`auth_method` 必须显式设 WPA2，默认 None 连不上任何 AP
- `leddrv.rs`：**必须走 `rmt_write_items` 路径**（帧先预编码成 `VariableLengthSignal` 再 `start_blocking`）。
  切勿回退到 `start_iter_blocking`（vendor 驱动用的就是它）——那个 API 在 RMT 中断里跑 Rust 迭代器喂数据，
  WiFi 一抢占就欠载 → WS2812 提前锁存 → 后段灯保持旧帧（黑）/整段变成全 1（白）。同类的官方警告见
  esp-idf-hal `rmt_legacy.rs` 的 `start_iter_blocking` 文档；`mem_block_num` 已从默认 1 提到 4。
- `main.rs`：任何阶段失败必须重试，**绝不让 main 返回 Err**（返回 = 整机重启循环）
- `app/sdkconfig.defaults`：主任务栈 16384（默认值会栈溢出）；主控制台是 UART0
- 版本锁定组合见 `firmware/README.md`，勿随意升级 esp-idf-svc/hal/IDF

## 验证顺序（改完代码后）

1. `cargo test -p wordclock-core --target x86_64-unknown-linux-gnu`（宿主机）
2. `cargo build -p word-clock --release`（目标机，确认零警告）
3. 逻辑正确性可在宿主机验证的，优先宿主机验证再上板

## 当前状态与下一步

见 [docs/roadmap.md](docs/roadmap.md)。MVP 已点亮（2026-08），下一步是 114 灯全量平铺实测 → Phase A-Full（Web 配置页等）。
