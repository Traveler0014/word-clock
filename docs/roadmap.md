# 路线图

## Phase A-MVP — 路线A：Rust std on esp-idf-svc —— ✅ 达成（2026-10-18）

> 状态：实机点亮并稳定走时；114 灯全量验证 + 配网/管理页可用。踩坑详见 `docs/development.md`。

- [x] 工程脚手架（workspace: app + wordclock-core）
- [x] 字库映射 + 时间逻辑 + 宿主机单元测试（9 通过）
- [x] espup/espflash 工具链安装
- [x] 首次编译通过（svc 0.52 / hal 0.46 + IDF v5.5.1，vendor ws2812 补丁版）
- [x] 实机刷写：自检灯链 → WiFi → SNTP → 每分钟刷新（2026-08 实测通过，
      WiFi auth_method 修复后 2.7s 连上，NTP 2s 同步，IP 192.168.1.110）
- [x] 灯条平铺实测：114 灯全量，蛇形方向与逐行映射已用静态标尺验证（2026-10-18）
      ⇒ 同时修掉一个真 bug：vendor 驱动 `start_iter_blocking` 在 WiFi 下供数欠载导致花屏，
      改为预编码 + `rmt_write_items` 后恢复正常
- [x] 配网：凭据进 NVS + SoftAP 网页配置（`WordClock-XXXXXX` → `http://192.168.9.20`，含强制门户与 DNS 劫持），
      不再需要把 WiFi 密码编进固件（2026-10-18）
- [x] 配网页版式：二级菜单（选网络 → 输密码）、黑白 + 两级灰、无动画/无图标/浏览器默认衬线，
      兼容易墨水屏（2026-10-18）

## 打磨阶段（进行中）

优先级从高到低，均为"在不改变架构前提下的功能补强"：

- [ ] 强制门户在各平台的自动弹页验证（iOS CNA / Android notification / Windows 提示）
- [ ] 夜间时段调光（需一个轻量调度器）
- [ ] 换词淡入淡出（注意：`show()` 是整帧写入，过渡要做在帧生成层）
- [ ] 24h 稳定性测试 + 长跑内存/堆水位观察
- [ ] 配网页/管理页可访问性（对比度、字号档位）
- [ ] OTA（`esp_https_ota`）+ 版本号上报

## Phase A-Full

- [x] 运行期管理页（局域网 `http://<ip>/`）：状态、亮度/色温/时区/管理口令、更换网络、恢复出厂（2026-10-18）
- [x] 恢复出厂手势：连续 3 次未正常启动清空 NVS
- [ ] 强制门户（DNS 劫持 + 302），配网时手机自动弹登录页（当前需手动输 IP）
- [ ] BLE 配网（NimBLE GATT + Web Bluetooth 页面，仅 Android/桌面 Chrome）
- [ ] mDNS `clock.local`（评估结论：IDF5 需外置组件 + 构建期联网，暂不引入）
- [ ] Web 配置页（EspHttpServer）：颜色/亮度/WiFi/夜间时段（配网页已是同一套 HTTP 服务，可复用）
- [ ] 光敏自动亮度（ADC）
- [ ] 换词淡入淡出过渡
- [ ] OTA；DS3231 断电守时（可选）
- [ ] 结构：小样验证 → 全套打印 → 总装
- [ ] 24h 稳定性测试

## Phase B（练手）— 路线B：esp-hal + embassy no_std

- [ ] 复用 `wordclock-core`（已 no_std 就绪）
- [ ] esp-hal RMT SmartLeds 驱动
- [ ] esp-radio WiFi + embassy-net + 手写 SNTP 客户端
- [ ] picoserve Web 配置
- [ ] 与路线A 对比：固件体积/内存/功耗/开发体验，写篇总结

## 结构迭代

> 分支决策：灯条飞线方案 vs 灯板 PCB 方案（评估见 [led-pcb.md](led-pcb.md)）。
> 若走 PCB，pitch/尺寸/结构需按 PCB 重新定稿，下面的灯条校验规步骤作废。

- [ ] v1: 间距校验规（1×11 单行 + 真实灯条，定稿 pitch）→ 格栅 + 背板小样验证串光/槽松紧
- [x] **灯板 PCB 设计**：192×192 / pitch 15.0 / 114 灯 + 板载电平转换（KiCad 原理图 + PCB + Gerber，ERC/DRC 0，网表逐网一致）→ `hardware/`
- [ ] 灯板打样 + 实机验证（现有 DevKitC，固件零改动）→ 在 [hardware/README.md](../hardware/README.md)
- [x] 结构按 PCB 重新定稿：**v3**（删行槽/过条缺口、格栅 14mm 让位控制区、卡扣式免螺丝、
      DevKit/USB-C 母口定位）→ [enclosure/README.md](../enclosure/README.md)、[docs/pcba-heights.md](pcba-heights.md)
- [ ] v3 实打装配验证（卡扣松紧 / 串光 / 角点对齐）
- [ ] （可选）集成版 v2：ESP32-C5-WROOM-1 上板（原生 USB-Serial-JTAG + 外置天线）
- [ ] v2: 全尺寸格栅 + 背板 + 面板框（stencil 镂空或车贴）
- [ ] v3: 电子仓、走线、底座外观（仿图中木纹/大理石质感贴膜）
