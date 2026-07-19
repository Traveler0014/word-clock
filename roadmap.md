# 路线图

## Phase A-MVP（当前）— 路线A：Rust std on esp-idf-svc

- [x] 工程脚手架（workspace: app + wordclock-core）
- [x] 字库映射 + 时间逻辑 + 宿主机单元测试
- [ ] espup/espflash 工具链安装（后台进行中）
- [ ] 首次编译通过（验证 esp-idf-sys ↔ IDF v6.0.2，不兼容则切 v5.5）
- [ ] 实机刷写：自检灯链 → WiFi → SNTP → 每分钟刷新
- [ ] 灯条平铺实测：全时间刻度正确、蛇形方向对齐

## Phase A-Full

- [ ] Web 配置页（EspHttpServer）：颜色/亮度/WiFi/夜间时段
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

- [ ] v1: 格栅 + 背板小样（2×2 格）验证间距/串光
- [ ] v2: 全尺寸格栅 + 背板 + 面板框（stencil 镂空或车贴）
- [ ] v3: 电子仓、走线、底座外观（仿图中木纹/大理石质感贴膜）
