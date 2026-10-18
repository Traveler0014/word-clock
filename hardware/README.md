# hardware/ — 灯板 PCB 设计（KiCad）

把原方案的 10 行灯条 + 27 处行间飞线，替换成**单块 192×192mm 的 LED 面板 PCB**。
含**原理图 + PCB + Gerber + 预览图 + 可复现的生成脚本**。

> 方案背景与取舍见 [`../docs/led-pcb.md`](../docs/led-pcb.md)。
> 本目录是其中「方案 A：灯板 + 现有 ESP32-C5-DevKitC」的具体实现。

## 文件

| 文件 | 内容 |
|---|---|
| `wordclock-led.kicad_pro` | KiCad 工程 |
| `wordclock-led.kicad_sch` | 原理图（平坦单页 A1：114 颗灯 + 电平转换 + 电源/接口） |
| `wordclock-led.kicad_pcb` | PCB（2 层，已铺铜填充） |
| `wordclock-led-sch.pdf` | 原理图 PDF（便于评审） |
| `gen/gen_sch.py` | 原理图生成脚本（纯标准库，任意 python3） |
| `gen/gen_pcb.py` | PCB 生成脚本（需 KiCad 自带 python + pcbnew） |
| `preview/pcb_top.png` / `pcb_bottom.png` | 3D 预览 |
| `fab/*` | Gerber + 钻孔（F.Cu/B.Cu/Mask/Paste/Silk/Edge + drl + job） |

## 板子规格

| 项 | 值 |
|---|---|
| 尺寸 / 层数 | 192.0 × 192.0 mm，2 层，1.6 mm，1oz（建议 2oz 或限制亮度） |
| 灯 | 114 × WS2812B 5050（110 字母阵 11×10 + 4 角点） |
| 灯距 pitch | **15.0 mm**（字母阵 165×150，居中留边 13.5 / 21） |
| 角点 | 距板角 7 mm（腔/窗由结构件做，φ11 / φ5） |
| 数据链 | 蛇形（行 0 左→右，奇数行反向），链尾 109 在左下；角点按周长环 110→111→112→113。**原理图用显式串联导线画出**（奇数行 `mirror y` 让 DIN/DOUT 朝向链方向，VDD/VSS 保持上下） |
| 走线 | 数据走线全程**无直角**：转弯一律 45° 倒角 |
| 电源 | 顶层 **GND 铺铜区**，底层 **+5V 平面**；每颗 LED/电容就近打过孔到 +5V |
| 去耦 | 每颗灯 1 × 100nF 0402（C1–C114），入口 1000µF + 22µF + 100nF |
| 电平转换 | U1 = 74AHCT1G125（SOT-23-5，5V 供电），输出串 R1 = 33Ω |
| 安装孔 | 4 × M3（四边中点，和原外壳螺丝位一致） |
| 丝印 | 板名、`5V IN (J2)` / `ESP32-C5 (J1)`、+5V/GND |

## 与 ESP32-C5 的连接

```
ESP32-C5-DevKitC-1                灯板 (J1)                板上
  GPIO4 ── 杜邦/JST 4pin DATA ──► J1.3 ──► U1.A ── 74AHCT1G125(5V) ──► R1(33Ω) ──► D1.DIN
  GND   ────────────────────────► J1.2/4 (GND)
  5V    ◄─────────────────────── J1.1 (仅当 JP1 短接)
                                   J2 = 5V/3A 外部电源输入（5V,5V,GND,GND）
```

| 连接器 | 引脚 | 网络 | 说明 |
|---|---|---|---|
| **J1**（接 DevKitC） | 1 | `/+5V_DEV` | 经 JP1 可选给开发板供 5V |
| | 2, 4 | `GND` | |
| | 3 | `/DIN_IN` | 来自 C5 `GPIO4` 的 3.3V 数据 |
| **J2**（电源输入） | 1, 2 | `+5V` | 5V/3A，双针并联 |
| | 3, 4 | `GND` | |
| **JP1**（焊桥） | 1–2 | `+5V` ↔ `/+5V_DEV` | **默认断开**；短接则灯板 5V 反给开发板 |

- **开发期**：DevKitC 用 USB 供电时 **JP1 保持断开**（否则双电源互灌）；两者务必共地（J1.2/4）。
- **成品**：外部 5V 接 J2，JP1 短接，由灯板同时给开发板供电，或继续让开发板独立供电。
- 数据电平：板载 74AHCT125 已做 3.3V→5V，**不要再外接电平转换**。

> 若后续要出「集成版 v2」（ESP32-C5-WROOM-1 直接上板），参考 `../docs/led-pcb.md` §5 方案 B：
> 原生 USB-Serial-JTAG（IO13/14）+ IO4(RMT) + IO1(ADC 光敏) + IO23/24(I2C)，
> 天线强烈建议用 WROOM-1U 外置，USB 控制台需改 sdkconfig。

## ⚠ 元件高度与勘误

- 正面最高元件：**C117 电解 11.5mm** > 排针 8.5mm > C116 2.52mm > 灯珠 1.61mm。
  结构件按 14mm 格栅净高设计。完整实测表见 [`../docs/pcba-heights.md`](../docs/pcba-heights.md)。
- **J1/J2 已换料**：原 BOM 的 C492419（XFCN PZ254V-12-4P）是**双排 2×2**，插不进
  `PinHeader_1x04`；已改用 **LCSC C19190970 = HDGC HDGCPH-PZ01-04**（单排 1×4，金镀），
  板面高 **8.54 mm**。
- STEP 外包络是 192×**202**（上下各 ~4.85mm 工艺边），实际板子 **192×192**。

## 固件

**无需改动**：仍 GPIO4、仍 114 颗、蛇形索引与 `wordclock-core::xy_to_index` 一致。
PCB 网络名与原理图完全一致（`/D0…/D113`、`/DOUT`、`/DIN_IN`、`/S1`、`/+5V_DEV`、`+5V`、`GND`），
所以在 KiCad 里「从原理图更新 PCB」不会断开已布好的线。

角点顺序在板上是 `110 左下 → 111 右下 → 112 右上 → 113 左上`。
固件只看「点亮前 n 颗」，功能不受影响；若想 `minute=1` 固定点某角，改 `Frame::dots()` 一行即可
（原 `docs/wiring.md` 的 110~113 标注需同步修正）。

## 校验记录

- **ERC**：0 error / 0 warning
- **DRC**：0 violation / 0 未连接；走线无直角（转弯均 45°）
- **网络一致性**：原理图 netlist 与 PCB 焊盘网络逐网比对，120 网 **0 mismatch**
- **串联链**：`/D0…/D113`、`/DOUT` 逐节校验（`D{i}.DOUT → D{i+1}.DIN`）全部正确
- **Gerber/钻孔**：`kicad-cli` 导出成功

重新校验（KiCad 10 命令行）：

```bash
kicad-cli sch erc   --severity-error wordclock-led.kicad_sch
kicad-cli pcb drc   --severity-error wordclock-led.kicad_pcb
kicad-cli pcb export gerbers -o fab/ -l "F.Cu,B.Cu,F.Mask,B.Mask,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,Edge.Cuts" wordclock-led.kicad_pcb
kicad-cli pcb export drill   -o fab/ wordclock-led.kicad_pcb
```

## 重新生成

```
hardware/
├── gen/gen_sch.py     # 任意 python3（Linux 亦可），读 KiCad 符号库
└── gen/gen_pcb.py     # 必须用 KiCad 自带 python（带 pcbnew 模块）
```

```bash
cd hardware
python3 gen/gen_sch.py                                   # 生成 .kicad_sch
"C:/Program Files/KiCad/10.0/bin/python.exe" gen/gen_pcb.py   # 生成 .kicad_pcb（含铺铜）
```

脚本里的几何参数（板尺寸、pitch、元件坐标、走线）都在文件顶部，改完全局重跑即可。
两脚本自动探测 KiCad 安装路径（Windows `D:\...` 或 WSL `/mnt/d/...`）。
每次生成 UUID 随机，故与已提交版本会有 diff，属正常。

> `gen_pcb.py` 的铺铜需要「保存→重新加载→填充→再保存」的往返：
> 无头模式下直接对新建 board 调 `ZONE_FILLER` 会崩（已在脚本内处理）。

## 打样 / 装配注意

- 2 层 192×192 已能覆盖当前电流（实测 <0.5A）；全白跑满 6.8A 时请用 2oz 或限亮。
- 每颗 5050 的 GND 焊盘**实心**连到 GND 铺铜（利于散热与载流）；返修时热风要够功率。
- C117（1000µF 直立电解）在板正面顶边（146, 11.5），高度约 12mm。
- 控制区（顶边 y≈3..19）元件在**正面**：结构件需在该处留凹槽/让位，
  或把控制区整体移到板背（+5V 平面留 keepout 即可）。见「待决」。
- 建议先打 **1 片** 验证：先只上 5V 测短路/静态电流，再用现有固件自检全链。

## 待决 / 风险

1. **"200×200"口径**：若指含外壳的成品，PCB 可再缩到 ~192→实际外框 200；当前 PCB 192 + 4mm 壁 ≈ 200×200，正好。
2. **控制区位置**：现放正面顶边，结构件要配合；若想更干净可移到背面。
3. **角点顺序**：如需 `minute=1 → 左上`，改固件映射（1 行）。
4. **散热/密封**：密封腔 + 1/4 限亮预计 OK；全白需通风。
5. **是否做集成版 v2**（C5 上板）：见 `../docs/led-pcb.md` §5。
