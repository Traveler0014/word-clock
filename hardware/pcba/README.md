# pcba/ — 嘉立创（JLCPCB）PCBA 下单资料

配套 `../fab/wordclock-led-gerber-jlcpcb.zip`（PCB）+ 本目录 `bom_jlcpcb.csv` + `cpl_jlcpcb.csv`（贴片）。
下单流程：先下 PCB（上传 Gerber zip）→ 在同一订单里勾选 **PCB Assembly** → 上传 BOM + CPL。

## 下单参数（Economic PCBA）

| 项 | 选择 | 依据 |
|---|---|---|
| 服务 | **Economic PCBA**（不是 Standard） | 单面贴装、2 层、1.6mm、板 192×192、0402 最小封装，全部落在 Economic 范围内 |
| 层数 / 板厚 | 2 层 / **1.6mm** | Economic 仅 0.8–1.6mm |
| 阻焊颜色 | **黑色**（推荐，配白色丝印效果最好） | 见下表：1.6mm+黑 支持无铅 HASL，数量 2–50 |
| 表面处理 | **无铅 HASL**（1.6mm 黑色只支持 HASL；要 ENIG 需选绿色） | Economic 颜色/表面处理受限表 |
| 数量 | **2–50 片**（Economic 下限 2 片） | 送礼物建议 2 或 5 片 |
| 交付 | 单片（Single PCB）即可 | — |
| 拼板 | 不需要 | Economic：单片 10×10 – 470×500mm，192×192 直接做 |
| 基准点 (Fiducial) | **不需要加** | Economic「Fiducials: Not necessary」 |
| 贴装面 | 单面（Top） | 全部 235 个元件在正面 |
| 元件来源 | 嘉立创元件库（Turnkey，无需自备料） | 全部 8 种料均已在库有货 |

> 若你想要 **ENIG（沉金）**：只能选 **绿色** 阻焊（1.6mm 绿色支持 HASL/ENIG，2–50 片）。

## 元件清单（BOM）

| LCSC | 型号 | 封装 | 数量 | 嘉立创类型 | 单价(≈) |
|---|---|---|---|---|---|
| **C52917433** | Worldsemi WS2812B-V6 | SMD5050-4P | 114 | Extended | $0.101 |
| **C1525** | Samsung CL05B104KO5NNNC | 0402 | 114 | **Basic** | $0.0045 |
| **C19190970** | HDGC HDGCPH-PZ01-04 | Plugin 2.54mm **1×4 单排** | 2 | Extended | $0.02 |

> ✅ **已修正**：原选的 **C492419**（XFCN `PZ254V-12-4P`）是**双排 2×2**，插不进
> `PinHeader_1x04_P2.54mm_Vertical`；现改用 **HDGC HDGCPH-PZ01-04（单排 1×4，金镀）**，
> 与焊盘一致。
> 高度：绝缘座 2.54 + 插针 6.0 = **8.54 mm**（见 [`../../docs/pcba-heights.md`](../../docs/pcba-heights.md)）。
> 注：该件规格书要求 PCB 孔 Ø0.85，本板用 KiCad 标准 Ø1.0（引脚 0.64 方），波峰焊没问题。
| **C14663** | YAGEO CC0603KRX7R9BB104 | 0603 | 1 | **Basic** | $0.0138 |
| **C52306** | Samsung CL32A226KAJNNNE | 1210 | 1 | Extended | $0.305 |
| **C525818** | NCC 1000µF 16V（D10×11, P5.0） | Plugin D10 P5.0 | 1 | Extended | $0.551 |
| **C23140** | UNI-ROYAL 0603WAF330JT5E | 0603 | 1 | **Basic** | $0.0028 |
| **C52140417** | MDD 74AHCT1G125GV | SOT-23-5 | 1 | Extended | $0.075 |

- **贴装总数 235**；`JP1`（焊桥）与 `H1–H4`（安装孔）无元件，已排除。
- **Extended 料 5 种**：嘉立创按「扩展料」每种收一次性费用（约 $3/种，以订单页为准）→ 约 $15。
- 每个编号的点数：D=Led 114、C1–C114=100nF 114、J1/J2=排针 2、其余各 1。
- `1000uF` 备选：**C10750**（CX D10×17mm, 25V, $0.095，库存大）——但高 17mm，需确认结构件让位。

## LED 选型（重要）

原选的 **C2761795（WS2812B-B/T）已停产**，嘉立创会报「经济型生产模式暂不支持该器件」。
采用 **C2874885（WS2812B-V5/W，原厂 Worldsemi）**。

**可寻址灯筛选的关键：器件类型必须是 `RGB LEDs(Built-in IC)` 且属性里有 `Data Transmission Rate = 800Kbit/s`。**
按这个标准筛出来的原厂 Worldsemi 有货选项：

| LCSC | 型号 | 库存 | 单价 | 说明 |
|---|---|---|---|---|
| **C2874885** | **WS2812B-V5/W** ✅ 采用 | 77,016 | $0.1097 | 正宗 WS2812B；白磨砂透镜 |
| C2920042 | WS2812E-V5 | 291,960 | $0.0673 | 更便宜、库存更大（Worldsemi 同厂）|
| C4154882 | WS2812B-XF02/W | 26,496 | $0.0797 | Worldsemi |
| C2843785 | XL-5050RGBC-2812B | 794,978 | $0.0548 | 第三方（XINGLIGHT）克隆 |
| ~~C2761795~~ | WS2812B-B/T | — | — | ❌ 已停产 |
| ~~C114586~~ | WS2812B-B/W | — | — | ❌ 不在嘉立创 SMT 贴片库（仅 LCSC 商城有）|
| ~~C52917433~~ | 「WS2812B-V6」 | 80,466 | $0.1008 | ⚠️ **慎用**：嘉立创库里器件类型是 `LED Indication - Discrete`，无 800Kbit/s 属性 |
| C52941391 / C55629353 | -V6 变体 | 2,833 / 200 | — | 库存太少，不够 114 颗/板 |

### 关于「白光」

WS2812B 是 **RGB 三色可寻址灯（内置 IC）**，不是单色灯，也**不是专用白光灯**：
- 发白光：R=G=B（如 0xFFFFFF）；本工程固件用的是暖白 `255,190,110`
- **没有独立白色通道**；若需高显色/高效率白光，需换 **SK6812 RGBW**（4 字节/像素，固件要改，见 `docs/led-pcb.md`）

> 嘉立创提示「系统匹配的 LED 需要确认颜色」是因为 BOM 的 Comment 写得太笼统。现在 Comment 已写具体型号
> （`WS2812B-V5/W`）并给出明确 LCSC 编号，不会再要求选颜色。

## 器件方向核对结果

**设计层面已验证正确**（用几何 + 网表反推每个器件的 pin1 落在哪个网络）：

| 器件 | 旋转 | pin1 实际网络 | 结论 |
|---|---|---|---|
| D1–D114 (WS2812B) | 0° / 180° 交替 | pin1(VDD)→+5V、pin3(VSS)→GND、pin4(DIN)→数据入、pin2(DOUT)→数据出 | ✅ 偶行 DIN 在左、奇行 DIN 在右，蛇形正确 |
| U1 (SOT-23-5) | 0° | pin1(OE)→GND、pin5(VCC)→+5V、pin4(Y)→/S1 | ✅ |
| C117 (电解) | 0° | pin1(+)→+5V、pin2→GND | ✅ |
| J1 | 0° | pin1→/+5V_DEV | ✅ |
| J2 | 0° | pin1→+5V | ✅ |
| R1/C1–C115/C116 | 0° | 无极性 | ✅ 方向无关 |

CPL 的坐标与旋转与 PCB 逐项比对：**0 处不符**。

### ⚠ 下单时必须在「元件贴装确认」预览里看两件事

**① 旋转基准**：CPL 的 Rotation 是 KiCad 约定，嘉立创对每颗料有自己的 0° 定义。最易不一致的是
**WS2812B（5050）×114** 和 **SOT-23-5**。在预览里看 LED 的**缺角/pin1 标记**是否落在接 +5V 过孔
那一侧的焊盘上。若整体差 90°/180° → 改 `Rotation` 列（全列加同一偏移）重传。

**② J1/J2/C117 的坐标参考点**：KiCad 这三个封装的原点在**引脚 1**上，不是元件几何中心；
而嘉立创 CPL 规格写的是「Mid X/Mid Y = 元件中心」。差异：
- J1/J2（1×4 排针）：中心还要 +3.81mm（板坐标系 Y 向下）
- C117（D10 电解）：中心还要 +2.5mm（X）

→ 所以额外提供了 **`cpl_jlcpcb_center-ref.csv`**（把这 3 个件移到几何中心）。
先在预览里看 J1/J2/C117 是否与焊盘对齐，**对齐就用标准版，偏了就用 center-ref 版**。

## 原「两个必须人工确认的点」

1. **旋转角（最重要）**：CPL 里的 Rotation 是 KiCad 约定。嘉立创元件库对 5050 LED 的 0° 定义可能与 KiCad
   不同（常见差 90°/180°）。**下单后务必在「Confirm Parts Placement / 元件贴装确认」预览里逐颗核对**，
   尤其 114 颗 WS2812B 的缺角（pin1）方向，以及 J1/J2、U1、C117 极性。若不对，在 CPL 里加偏移量重新上传。
2. **THT 波峰焊**：J1、J2、C117 是插件，嘉立创走**波峰焊**（官方支持 SMT+THT 混装，Economic 也可）。
   板正面还有 233 个 SMD，波峰焊只接触背面，本项目背面无元件，无风险。

## 主要设计参数（复核记录）

- 板 192×192mm / 2 层 / 1.6mm；铜厚 1oz；最小线宽 0.25mm、间距 0.20mm
- 过孔 Ø0.6/0.3（环宽 0.15mm，232 个）；插件孔 Ø1.0（10 个）；NPTH Ø3.2（4 个）
- 最小封装 0402（Economic 下限 0402）；最小 IC 引脚间距 0.95mm（下限 0.4mm）
- 铜到板边 0.5mm（下限 0.2mm）；DRC 0 违规 / 0 未连接

## 重新生成

```bash
# PCB Gerber
kicad-cli pcb export gerbers -l "F.Cu,B.Cu,F.Mask,B.Mask,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,Edge.Cuts" \
  --precision 6 -o fab/ wordclock-led.kicad_pcb
kicad-cli pcb export drill -o fab/ wordclock-led.kicad_pcb
# 贴片位置
kicad-cli pcb export pos --format csv --units mm --side front -o pcba/pos.csv wordclock-led.kicad_pcb
# 再由 pos.csv 生成 cpl_jlcpcb.csv（列名 Designator,Mid X,Mid Y,Layer,Rotation，Y 沿用 KiCad 导出的负值）
```
