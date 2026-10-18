#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Generate wordclock-led.kicad_pcb with KiCad's bundled python (pcbnew).

Board : 192 x 192 mm, 2 layers, 1.6 mm
  - 110x WS2812B (11x10 grid, pitch 15.0) + 4 corner minute dots
  - 114x 100nF (0402) one per LED
  - level shifter U1 (74AHCT1G125, SOT-23-5) + series R1 + bulk caps
  - J1 (to ESP32-C5 DevKitC), J2 (5V in), JP1 (solder link)
  - F.Cu = GND zone, B.Cu = +5V zone, per-LED via to +5V plane
  - serpentine data chain routed on F.Cu

Run:  "C:\\Program Files\\KiCad\\10.0\\bin\\python.exe" gen_pcb.py
Zones are filled after a save/reload round trip (headless ZONE_FILLER needs it).
"""
import os, sys, math
import pcbnew

def kicad_share():
    for c in (r"D:\Program Files\KiCad\10.0\share\kicad",
              "/mnt/d/Program Files/KiCad/10.0/share/kicad"):
        if os.path.isdir(os.path.join(c, "footprints")):
            return c
    raise SystemExit("KiCad share not found")

SHARE = kicad_share()
FP = os.path.join(SHARE, "footprints")
HERE = os.path.dirname(os.path.abspath(__file__))
OUTDIR = os.path.dirname(HERE)
OUT = os.path.join(OUTDIR, "wordclock-led.kicad_pcb")
TMP = os.path.join(OUTDIR, "_wordclock-led_unfilled.kicad_pcb")

# ----------------------------------------------------------------- geometry
BW = BH = 192.0
PITCH = 15.0
COLS, ROWS = 11, 10
GX = (BW - COLS * PITCH) / 2.0          # 13.5
GY = (BH - ROWS * PITCH) / 2.0          # 21.0

def led_xy(idx):
    if idx < 110:
        r, k = divmod(idx, COLS)
        c = k if r % 2 == 0 else COLS - 1 - k   # serpentine: odd rows reversed
        return (GX + (c + 0.5) * PITCH, GY + (r + 0.5) * PITCH,
                180.0 if r % 2 == 0 else 0.0)
    dots = [(7.0, 185.0, 0.0), (185.0, 185.0, 180.0),
            (185.0, 7.0, 180.0), (7.0, 7.0, 0.0)]
    return dots[idx - 110]

def cap_xy(idx):
    if idx < 110:
        r, k = divmod(idx, COLS)
        c = k if r % 2 == 0 else COLS - 1 - k
        return (GX + (c + 0.5) * PITCH, GY + (r + 0.5) * PITCH + 5.5)
    dots = [(13.0, 185.0), (179.0, 185.0), (179.0, 7.0), (13.0, 7.0)]
    return dots[idx - 110]

def mm(v):  return pcbnew.FromMM(v)
def V(x, y): return pcbnew.VECTOR2I(mm(x), mm(y))

# ----------------------------------------------------------------- board
board = pcbnew.CreateEmptyBoard()
board.GetDesignSettings().SetCopperLayerCount(2)
for attr in ("m_SolderMaskMinWidth", "m_SolderMaskExpansion"):
    try:
        if attr == "m_SolderMaskMinWidth":
            setattr(board.GetDesignSettings(), attr, 0)
    except Exception:
        pass
netinfo = {}

def net(name):
    if name not in netinfo:
        n = pcbnew.NETINFO_ITEM(board, name)
        board.Add(n)
        netinfo[name] = n
    return netinfo[name]

NET_5V, NET_GND = net("+5V"), net("GND")
def dnet(i):    return net("/D%d" % i)
NET_DOUT = net("/DOUT")
NET_DEV  = net("/+5V_DEV")
NET_DIN  = net("/DIN_IN")
NET_S1   = net("/S1")

def chamfer(pts, c=0.8):
    """Replace each interior vertex with a 45-degree cut (two points)."""
    if len(pts) < 3:
        return list(pts)
    out = [pts[0]]
    for i in range(1, len(pts) - 1):
        p0, p1, p2 = pts[i - 1], pts[i], pts[i + 1]
        v1 = (p1[0] - p0[0], p1[1] - p0[1]); L1 = math.hypot(*v1)
        v2 = (p2[0] - p1[0], p2[1] - p1[1]); L2 = math.hypot(*v2)
        if L1 < 1e-9 or L2 < 1e-9:
            continue
        c1 = min(c, L1 / 2.0); c2 = min(c, L2 / 2.0)
        out.append((p1[0] - v1[0] / L1 * c1, p1[1] - v1[1] / L1 * c1))
        out.append((p1[0] + v2[0] / L2 * c2, p1[1] + v2[1] / L2 * c2))
    out.append(pts[-1])
    return out

def add_seg(pts, net_, width=0.25, layer=pcbnew.F_Cu):
    pts = chamfer(pts, 0.8)
    for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
        if abs(x1 - x2) < 1e-6 and abs(y1 - y2) < 1e-6:
            continue
        t = pcbnew.PCB_TRACK(board)
        t.SetStart(V(x1, y1)); t.SetEnd(V(x2, y2))
        t.SetWidth(mm(width)); t.SetLayer(layer); t.SetNet(net_)
        board.Add(t)

def add_via(net_, x, y, dia=0.6, drill=0.3):
    v = pcbnew.PCB_VIA(board)
    v.SetPosition(V(x, y)); v.SetWidth(mm(dia)); v.SetDrill(mm(drill))
    v.SetViaType(pcbnew.VIATYPE_THROUGH)
    v.SetLayerPair(pcbnew.F_Cu, pcbnew.B_Cu); v.SetNet(net_)
    board.Add(v)

# ----------------------------------------------------------------- footprints
def load(libdir, name):
    fp = pcbnew.FootprintLoad(os.path.join(FP, libdir), name)
    if fp is None:
        raise SystemExit("footprint not found: %s/%s" % (libdir, name))
    nick = libdir[:-len(".pretty")] if libdir.endswith(".pretty") else libdir
    try:
        fp.SetFPID(pcbnew.LIB_ID(nick, name))   # match schematic footprint field
    except Exception as e:
        print("SetFPID failed:", e)
    return fp

def set_refs(fp, ref, value, hide_silk=True):
    fp.SetReference(ref); fp.SetValue(value)
    for txt in (fp.Reference(), fp.Value()):
        try:
            txt.SetTextSize(pcbnew.VECTOR2I(mm(0.8), mm(0.8)))
            txt.SetTextThickness(mm(0.12))
        except Exception:
            pass
    try:
        fp.Value().SetVisible(False)
        if hide_silk:
            fp.Reference().SetLayer(pcbnew.F_Fab)   # keep silk clean
    except Exception:
        pass

def place(fp, x, y, rot=0.0, flipped=False):
    fp.SetPosition(V(x, y))
    if rot:
        fp.SetOrientationDegrees(rot)
    board.Add(fp)
    if flipped:
        fp.Flip(fp.GetPosition(), False)
    return fp

def padpos(fp, num):
    p = fp.FindPadByNumber(num)
    q = p.GetPosition()
    return (pcbnew.ToMM(q.x), pcbnew.ToMM(q.y))

LED_FP = ("LED_SMD.pretty", "LED_WS2812B_PLCC4_5.0x5.0mm_P3.2mm")
CAP_FP = ("Capacitor_SMD.pretty", "C_0402_1005Metric")

leds, caps = [], []
for i in range(114):
    x, y, rot = led_xy(i)
    fp = load(*LED_FP); set_refs(fp, "D%d" % (i + 1), "WS2812B")
    place(fp, x, y, rot)
    fp.FindPadByNumber("1").SetNet(NET_5V)
    fp.FindPadByNumber("3").SetNet(NET_GND)
    fp.FindPadByNumber("4").SetNet(dnet(i))
    fp.FindPadByNumber("2").SetNet(NET_DOUT if i == 113 else dnet(i + 1))
    leds.append(fp)

    cx, cy = cap_xy(i)
    cfp = load(*CAP_FP); set_refs(cfp, "C%d" % (i + 1), "100nF")
    place(cfp, cx, cy)
    cfp.FindPadByNumber("1").SetNet(NET_5V)
    cfp.FindPadByNumber("2").SetNet(NET_GND)
    caps.append(cfp)

# --- control section (top margin, split around mounting hole at x=96)
def control(libdir, name, ref, val, x, y, rot=0.0):
    fp = load(libdir, name); set_refs(fp, ref, val)
    place(fp, x, y, rot)
    return fp

j1 = control("Connector_PinHeader_2.54mm.pretty",
             "PinHeader_1x04_P2.54mm_Vertical", "J1", "Conn_01x04", 24.0, 8.0)
j2 = control("Connector_PinHeader_2.54mm.pretty",
             "PinHeader_1x04_P2.54mm_Vertical", "J2", "Conn_01x04", 124.0, 8.0)
u1 = control("Package_TO_SOT_SMD.pretty", "SOT-23-5", "U1", "74AHCT1G125", 58.0, 12.0)
r1 = control("Resistor_SMD.pretty", "R_0603_1608Metric", "R1", "33", 74.0, 12.0)
c115 = control("Capacitor_SMD.pretty", "C_0603_1608Metric", "C115", "100nF", 100.0, 12.0)
c116 = control("Capacitor_SMD.pretty", "C_1210_3225Metric", "C116", "22uF", 112.0, 12.0)
c117 = control("Capacitor_THT.pretty", "CP_Radial_D10.0mm_P5.00mm", "C117", "1000uF", 146.0, 11.5)
jp1 = control("Jumper.pretty", "SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm",
              "JP1", "5V_LINK", 40.0, 12.0)

j1.FindPadByNumber("1").SetNet(NET_DEV)
j1.FindPadByNumber("2").SetNet(NET_GND)
j1.FindPadByNumber("3").SetNet(NET_DIN)
j1.FindPadByNumber("4").SetNet(NET_GND)
for pn in ("1", "2"):
    j2.FindPadByNumber(pn).SetNet(NET_5V)
for pn in ("3", "4"):
    j2.FindPadByNumber(pn).SetNet(NET_GND)
u1.FindPadByNumber("1").SetNet(NET_GND)     # OE
u1.FindPadByNumber("2").SetNet(NET_DIN)     # A
u1.FindPadByNumber("3").SetNet(NET_GND)
u1.FindPadByNumber("4").SetNet(NET_S1)      # Y
u1.FindPadByNumber("5").SetNet(NET_5V)      # VCC
r1.FindPadByNumber("1").SetNet(NET_S1)
r1.FindPadByNumber("2").SetNet(dnet(0))
c115.FindPadByNumber("1").SetNet(NET_5V); c115.FindPadByNumber("2").SetNet(NET_GND)
c116.FindPadByNumber("1").SetNet(NET_5V); c116.FindPadByNumber("2").SetNet(NET_GND)
c117.FindPadByNumber("1").SetNet(NET_5V);  c117.FindPadByNumber("2").SetNet(NET_GND)
jp1.FindPadByNumber("1").SetNet(NET_5V);   jp1.FindPadByNumber("2").SetNet(NET_DEV)

# ----------------------------------------------------------------- data chain
TRANS = {10, 21, 32, 43, 54, 65, 76, 87, 98}
for i in range(113):
    a = padpos(leds[i], "2")        # DOUT
    b = padpos(leds[i + 1], "4")    # DIN
    net_i = NET_DOUT if i == 112 else dnet(i + 1)
    if i in TRANS:
        row = i // COLS
        xo = 176.8 if row % 2 == 0 else 15.2
        add_seg([a, (xo, a[1]), (xo, b[1]), b], net_i)
    elif i == 109:                  # last grid LED -> first corner dot
        add_seg([a, b], net_i)
    elif i == 110:                  # bottom-left dot -> bottom-right
        add_seg([a, (a[0], 188.6), (b[0], 188.6), b], net_i)
    elif i == 111:                  # bottom-right dot -> top-right
        add_seg([a, (189.6, a[1]), (189.6, 10.2), b], net_i)
    elif i == 112:                  # top-right dot -> top-left
        add_seg([a, (a[0], 3.0), (b[0], 3.0), b], net_i)
    else:
        add_seg([a, b], net_i)

# ----------------------------------------------------------------- power vias
DOT_VIA = {110: (5.8, 182.1), 111: (186.2, 185.4),
           112: (186.2, 7.4), 113: (5.8, 6.6)}
for i, fp in enumerate(leds):
    x, y, rot = led_xy(i)
    vx, vy = padpos(fp, "1")        # VDD
    if i in DOT_VIA:
        tx, ty = DOT_VIA[i]
    else:
        dx, dy = vx - x, vy - y
        n = math.hypot(dx, dy)
        tx, ty = vx + dx / n * 1.35, vy + dy / n * 1.35
    add_via(NET_5V, tx, ty)
    add_seg([(vx, vy), (tx, ty)], NET_5V, width=0.3)

for i, fp in enumerate(caps):
    px, py = padpos(fp, "1")        # +5V pad
    tx, ty = px - 1.1, py
    add_via(NET_5V, tx, ty)
    add_seg([(px, py), (tx, ty)], NET_5V, width=0.3)

# ----------------------------------------------------------------- control routing
def pad2xy(fp, n):
    x, y = padpos(fp, n); return (x, y)

# J1.3 (DATA) -> U1.2 (A)
add_seg([(24.0, 13.08), (26.5, 13.08), (26.5, 17.8), (54.0, 17.8),
         (54.0, 12.0), (56.8625, 12.0)], NET_DIN, width=0.3)
# J1.1 (+5V_DEV) -> JP1.2
add_seg([(24.0, 8.0), (30.0, 8.0), (30.0, 9.6), (44.0, 9.6),
         (44.0, 12.0), (40.65, 12.0)], NET_DEV, width=0.3)
# U1.4 (Y) -> R1.1
add_seg([(59.1375, 12.95), (64.0, 12.95), (64.0, 12.0), (73.175, 12.0)], NET_S1, width=0.3)
# R1.2 -> D1 DIN  (net /D0)
add_seg([(74.825, 12.0), (78.0, 12.0), (78.0, 19.2), (16.3, 19.2),
         (16.3, 31.6), (18.55, 30.15)], dnet(0), width=0.3)
# +5V vias (U1.VCC, JP1.1, C115.2, C116.2)
for (px, py, tx, ty) in [(59.1375, 11.05, 60.8, 11.05),
                         (39.35, 12.0, 39.35, 13.8),
                         (99.225, 12.0, 99.225, 10.4),
                         (110.525, 12.0, 110.525, 10.4)]:
    add_via(NET_5V, tx, ty)
    add_seg([(px, py), (tx, ty)], NET_5V, width=0.4)

# ----------------------------------------------------------------- edge + mounting
for (x1, y1), (x2, y2) in [((0, 0), (BW, 0)), ((BW, 0), (BW, BH)),
                           ((BW, BH), (0, BH)), ((0, BH), (0, 0))]:
    s = pcbnew.PCB_SHAPE(board); s.SetShape(pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(V(x1, y1)); s.SetEnd(V(x2, y2))
    s.SetLayer(pcbnew.Edge_Cuts); s.SetWidth(mm(0.1)); board.Add(s)

for k, (x, y) in enumerate([(6, 96), (186, 96), (96, 6), (96, 186)]):
    fp = load("MountingHole.pretty", "MountingHole_3.2mm_M3")
    set_refs(fp, "H%d" % (k + 1), "MountingHole")
    try:
        fp.SetAttributes(fp.GetAttributes() | pcbnew.FP_BOARD_ONLY)
    except Exception:
        pass
    board.Add(fp); fp.SetPosition(V(x, y))

# ----------------------------------------------------------------- silk text
def silk(text, x, y, size=1.2, layer=pcbnew.F_SilkS):
    t = pcbnew.PCB_TEXT(board); t.SetText(text); t.SetPosition(V(x, y))
    t.SetLayer(layer); t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    board.Add(t)

silk("WORD CLOCK LED PANEL  v1", 96, 191.0, 1.4)
silk("5V IN (J2)  |  ESP32-C5 (J1)", 96, 3.0, 1.1)
silk("+5V", 21, 20.2, 1.0)
silk("GND", 171, 20.2, 1.0)

# ----------------------------------------------------------------- zones
def zone(n, layer, solid=False):
    z = pcbnew.ZONE(board)
    z.SetNet(n); z.SetLayer(layer)
    z.SetLocalClearance(mm(0.3)); z.SetMinThickness(mm(0.25))
    if solid:
        try:
            z.SetPadConnection(pcbnew.ZONE_CONNECTION_FULL)
        except Exception:
            pass
    ol = z.Outline(); ol.NewOutline()
    for (x, y) in [(0.4, 0.4), (BW - 0.4, 0.4), (BW - 0.4, BH - 0.4), (0.4, BH - 0.4)]:
        ol.Append(V(x, y))
    board.Add(z); return z

zone(NET_GND, pcbnew.F_Cu, solid=True)
zone(NET_5V, pcbnew.B_Cu)

# title block
tb = board.GetTitleBlock()
tb.SetTitle("Word Clock LED Panel")
tb.SetRevision("v1")
try:
    tb.SetComment(0, "ESP32-C5 + 114x WS2812B, 192x192mm, pitch 15.0mm")
except Exception:
    pass

# ----------------------------------------------------------------- save, reload, fill, save
board.Save(TMP)
print("saved unfilled:", TMP, flush=True)
b = pcbnew.LoadBoard(TMP)
pcbnew.ZONE_FILLER(b).Fill(b.Zones())
b.Save(OUT)
print("saved filled  :", OUT, flush=True)
print("tracks=%d vias=%d footprints=%d zones=%d" % (
    len(list(b.GetTracks())), 0,
    len(list(b.GetFootprints())), len(b.Zones())), flush=True)
