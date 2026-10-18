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

# 编译（不需要 WiFi 凭据：凭据只存 NVS，由配网页面写入）
cargo build -p word-clock

# 烧录（走 USB-JTAG 口，端口按 VID:PID 核对：303a:1001=JTAG / 1a86:55d3=UART）
# 2026-10 实测：espflash 自动复位可用，**不需要**手动 BOOT+RESET
espflash flash target/riscv32imac-esp-espidf/debug/word-clock --port /dev/ttyACM0

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
| **字钟显示花屏/缺字母/后段全白或全黑，且图案每次不同** | vendor ws2812 驱动走 `start_iter_blocking`：在 RMT 中断里跑 Rust 迭代器喂数据，WiFi 抢占 → 供数欠载 → WS2812 提前锁存 | `app/src/leddrv.rs` 文件头；已改为预编码 + `start_blocking`（`rmt_write_items`），`mem_block_num` 1→4 |
| 自检全屏都变色，但字钟帧是错的 | 自检只验证电气连通，不验证位流 → 不能用来排除丢位 | 用 `--features diag` 静态标尺固件验证映射 |
| 进配网模式就重启（`Stack protection fault`, task "main"） | `scan_n::<20>()` 的 `heapless::Vec` 放栈上，debug 构建下直接打爆主任务栈 | `wifi::scan()` 已改为 `scan_n::<8>()`，主任务栈提到 32768 |
| 配网页打不开 `192.168.4.1` | IDF 默认 AP 网段不一定是 192.168.4.0/24（本机实测 192.168.71.1） | `start_ap()` 从 `ap_netif().get_ip_info()` 动态取并打进日志 |
| 烧完 WiFi 连不上、行为像旧密码 | 凭据只存 NVS，与固件版本无关；重新配网/恢复出厂即可 | `app/src/creds.rs` 文件头 |
| 手机看不到配网热点 / 连上又掉 | APSTA 下扫描时 AP 必须离开信道，beacon 中断。周期扫描等于让 AP 反复消失 | 改为「进配网扫一次 + 页面按需 `/api/scan`」，扫描驻留 60~90ms（`wifi::scan`） |
| 配网热点连上了、有租约，但系统不弹登录页 | DHCP 广播的 DNS 不是设备自己 → 客户端连探测请求都发不出（日志里 0 条 DNS 查询）。`esp_netif_dhcps_option(DOMAIN_NAME_SERVER)` 期望**单字节开关**，不是 4 字节 IP（传 IP 返回 `ESP_ERR_INVALID_PARAMS`） | 用 `esp_netif_set_dns_info(ESP_NETIF_DNS_MAIN)`；`web.rs` 有 DNS 自检（设备向自己 :53 发查询验证应答器） |
| 改 AP 地址后 `dhcps: Illegal subnet mask` + panic | `esp_ip4_addr_t.addr` 是**网络字节序**，用 `(a<<24)\u{7c}…` 拼在 LE 芯片上会反序 | 用 `u32::from_ne_bytes([a,b,c,d])`（`wifi.rs` 注释） |
| 起 DNS 线程即 `Stack protection fault` | IDF 的 pthread 默认栈仅 ~3KB，`std::net` 初始化就爆 | `thread::Builder::stack_size(8192)` + `CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT=8192` |
| 管理页选中的选项 1~2 秒后被刷回设备当前值 | 轮询 `/api/info` 时无条件重新拉 `/api/settings`，覆盖了未保存的表单 | 页面加 `dirty` 标记：有未保存改动时轮询不覆盖；保存成功才清标记 |
| 管理页点「保存」无反应/整站卡死 | 在 HTTP handler 里直接操作 WiFi 或写 NVS，阻塞了 esp_http_server 的事件任务 | handler 只改共享状态，由主循环落盘/重启（`app/src/web.rs` 头注释） |
| 运行模式要扫 WiFi | STA 已连时扫描会让主页断线 1~2s | 运行模式不提供扫描，只允许手填 SSID（`/api/scan` 仅在配网模式注册） |
| 配网页在墨水屏上重绘闪烁/发灰 | 灰底灰字在 1-bit 屏上会抖成网点，且频繁改 DOM 会整屏重绘 | 页面改二级菜单、只保留两级灰用于非关键信息、DOM 仅在内容变化时更新、`prefers-contrast: more` 时退化为纯黑白 |
| 日志里看不到 Rust `info!` | Rust log 只走主控制台 | `app/sdkconfig.defaults` 注释 |
| esp-idf-sys 找不到 sdkconfig.defaults | 需显式指定 | `firmware/.cargo/config.toml` 注释 |

## 启动即崩循环 → 擦 Flash

现象：串口里反复 `Guru Meditation ... Illegal instruction`，`app_main` 之前的
日志都没有，`grep -c '^ESP-ROM'` 显示几十次重启。

这不是应用代码的问题（debug/release/全量重建都一样），是 **Flash 状态损坏**
（反复烧写后 NVS/PHY 分区不一致）。处置：

```bash
espflash erase-flash --port /dev/ttyACM0     # 清整片（会丢 WiFi 凭据，之后进配网）
espflash flash target/riscv32imac-esp-espidf/release/word-clock --port /dev/ttyACM0
```

顺带一条**性能陷阱**：dev（debug）构建里 `leddrv::encode()` 慢几十倍，
实测单帧写入 175ms（release 只要 1.6~3.4ms = 线材物理时间）。
调灯效一定要用 `--release`，否则会误判成"面板支撑不了高帧率"。

## 动效调试（tools/anim_lab）

灯板动效先在宿主机浏览器里调，再落固件（规格见 [animation.md](animation.md)）：

```bash
python3 tools/anim_lab.py        # → http://<本机IP>:8901/
```

它是**单文件**（`tools/anim_lab.html`，无构建、无依赖），改完刷新即可。
三个容易踩的点：

1. **别只看网页顺滑**。网页渲染是真彩，硬件是 8bit 线性光：α 0.18 只值 **6/255**，
   暗部慢速淡入必然跳台阶。实验台的 gamma 默认 2.2 就是为了让这个台阶提前暴露。
2. **相机曝光是渲染用的，不是固件值**。它模拟暗室长曝光，改它不影响导出的 CSV。
3. 分享/回归用 URL 参数钉住画面：`?state=connect&t=2.2&pause=1&exposure=2.6`。
