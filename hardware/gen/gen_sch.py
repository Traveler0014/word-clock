#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Generate wordclock-led.kicad_sch  (KiCad 10 schematic, version 20260101).

Pure text generation: extracts symbol definitions verbatim from the installed
KiCad symbol libraries and lays out:
  - main circuit : 5V input (J2), DevKitC interface (J1), level shifter U1,
                   series R1, bulk/bypass caps, solder-jumper JP1
  - LED array    : 110 letter LEDs (11x10) + 4 corner dots, WS2812B, wired by
                   net-name labels along the serpentine (D0..D113, DOUT)
  - bypass bank  : 114 x 100nF (one per LED) on +5V/GND rails

Nets are connected by local labels + power symbols, no long wires.
Run with any python3 (standard library only).  KiCad lib path configurable.
"""
import os, re, sys, uuid

def _kicad_share():
    for cand in (r"D:\Program Files\KiCad\10.0\share\kicad",
                 "/mnt/d/Program Files/KiCad/10.0/share/kicad"):
        if os.path.isdir(os.path.join(cand, "symbols")):
            return cand
    raise SystemExit("KiCad share dir not found")

KICAD_SHARE = _kicad_share()
SYMDIR = os.path.join(KICAD_SHARE, "symbols")
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(os.path.dirname(HERE), "wordclock-led.kicad_sch")

ROOT_UUID = str(uuid.uuid4())
PROJECT = "wordclock-led"

# ---------------------------------------------------------------- sexp helpers
def find_span(text, start):
    depth = 0; i = start; n = len(text); in_str = False
    while i < n:
        c = text[i]
        if in_str:
            if c == '\\':
                i += 2; continue
            if c == '"':
                in_str = False
        else:
            if c == '"':
                in_str = True
            elif c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
                if depth == 0:
                    return start, i + 1
        i += 1
    raise ValueError("unbalanced near %d" % start)

def extract_top_symbol(libfile, name):
    text = open(os.path.join(SYMDIR, libfile), encoding='utf-8').read()
    key = '(symbol "%s"' % name
    i = text.index(key)
    return text[slice(*find_span(text, i))]

def sub_blocks(text, tag):
    out = []
    for m in re.finditer(r'\(%s\b' % re.escape(tag), text):
        s, e = find_span(text, m.start())
        out.append(text[s:e])
    return out

def pins_of(sym_text):
    res = []
    for blk in sub_blocks(sym_text, 'pin'):
        if not re.match(r'\(pin\s+\S+\s+\S+', blk):
            continue
        at = re.search(r'\(at\s+(-?[\d.]+)\s+(-?[\d.]+)\s+(-?\d+)', blk)
        nu = re.search(r'\(number\s+"([^"]*)"', blk)
        if not (at and nu):
            continue
        res.append((nu.group(1), float(at.group(1)), float(at.group(2))))
    return res

# symbol raw text, renamed to "Lib:Name"
def load_symbol(libfile, libname, name, rename_to=None, value=None):
    raw = extract_top_symbol(libfile, name)
    if rename_to:
        raw = raw.replace('(symbol "%s"' % name, '(symbol "%s:%s"' % (libname, rename_to), 1)
        raw = raw.replace('"%s_' % name, '"%s_' % rename_to)   # sub-symbol names
        if value is not None:
            raw = raw.replace('(property "Value" "%s"' % name,
                              '(property "Value" "%s"' % value)
    else:
        raw = raw.replace('(symbol "%s"' % name, '(symbol "%s:%s"' % (libname, name), 1)
    return raw

SYMS = {
    "LED:WS2812B":        load_symbol("LED.kicad_sym", "LED", "WS2812B"),
    "Device:C":           load_symbol("Device.kicad_sym", "Device", "C"),
    "Device:R":           load_symbol("Device.kicad_sym", "Device", "R"),
    "Device:C_Polarized": load_symbol("Device.kicad_sym", "Device", "C_Polarized"),
    "74xGxx:74AHCT1G125": load_symbol("74xGxx.kicad_sym", "74xGxx", "74LVC1G125",
                                      rename_to="74AHCT1G125", value="74AHCT1G125"),
    "Connector_Generic:Conn_01x04": load_symbol("Connector_Generic.kicad_sym",
                                                "Connector_Generic", "Conn_01x04"),
    "Jumper:SolderJumper_2_Open":   load_symbol("Jumper.kicad_sym", "Jumper",
                                                "SolderJumper_2_Open"),
    "power:+5V":      load_symbol("power.kicad_sym", "power", "+5V"),
    "power:GND":      load_symbol("power.kicad_sym", "power", "GND"),
    "power:PWR_FLAG": load_symbol("power.kicad_sym", "power", "PWR_FLAG"),
}

PINMAP = {  # lib_id -> list of pin numbers
    "LED:WS2812B": ["1", "2", "3", "4"],
    "Device:C": ["1", "2"],
    "Device:R": ["1", "2"],
    "Device:C_Polarized": ["1", "2"],
    "74xGxx:74AHCT1G125": ["1", "2", "3", "4", "5"],
    "Connector_Generic:Conn_01x04": ["1", "2", "3", "4"],
    "Jumper:SolderJumper_2_Open": ["1", "2"],
    "power:+5V": ["1"], "power:GND": ["1"], "power:PWR_FLAG": ["1"],
}

# ---------------------------------------------------------------- emit helpers
def u():
    return str(uuid.uuid4())

def fnum(v):
    return ("%.4f" % v).rstrip('0').rstrip('.')

def eff(size=1.27, justify=None, hide=False):
    s = "\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size %s %s)\n\t\t\t\t)\n" % (fnum(size), fnum(size))
    if justify:
        s += "\t\t\t\t(justify %s)\n" % justify
    s += "\t\t\t)"
    if hide:
        s = "\t\t\t(hide yes)\n" + s
    return s

parts = []
def emit(s):
    parts.append(s)

def add_wire(x1, y1, x2, y2):
    emit('\t(wire\n\t\t(pts\n\t\t\t(xy %s %s) (xy %s %s)\n\t\t)\n'
         '\t\t(stroke\n\t\t\t(width 0)\n\t\t\t(type solid)\n\t\t)\n\t\t(uuid "%s")\n\t)'
         % (fnum(x1), fnum(y1), fnum(x2), fnum(y2), u()))

def add_label(name, x, y, rot=0, justify="left bottom"):
    emit('\t(label "%s"\n\t\t(at %s %s %d)\n%s\n\t\t(uuid "%s")\n\t)'
         % (name, fnum(x), fnum(y), rot,
            "\t\t(effects\n\t\t\t(font\n\t\t\t\t(size 1.27 1.27)\n\t\t\t)\n\t\t\t(justify %s)\n\t\t)" % justify,
            u()))

def add_symbol(lib_id, ref, value, footprint, x, y, rot=0, hidden_ref=False,
               ref_at=None, val_at=None, hide_value=False, mirror=None):
    ref_at = ref_at or (x + 2.54, y - 11.43)
    val_at = val_at or (x, y + 11.43)
    s = []
    s.append('\t(symbol')
    s.append('\t\t(lib_id "%s")' % lib_id)
    s.append('\t\t(at %s %s %d)' % (fnum(x), fnum(y), rot))
    if mirror:
        s.append('\t\t(mirror %s)' % mirror)
    s.append('\t\t(unit 1)')
    s.append('\t\t(body_style 1)')
    s.append('\t\t(exclude_from_sim no)')
    s.append('\t\t(in_bom yes)')
    s.append('\t\t(on_board yes)')
    s.append('\t\t(in_pos_files yes)')
    s.append('\t\t(dnp no)')
    s.append('\t\t(uuid "%s")' % u())
    # Reference
    s.append('\t\t(property "Reference" "%s"\n\t\t\t(at %s %s 0)\n\t\t\t(show_name no)\n'
             '\t\t\t(do_not_autoplace no)\n%s\n\t\t)'
             % (ref, fnum(ref_at[0]), fnum(ref_at[1]), eff(1.27, hide=hidden_ref)))
    # Value
    s.append('\t\t(property "Value" "%s"\n\t\t\t(at %s %s 0)\n\t\t\t(show_name no)\n'
             '\t\t\t(do_not_autoplace no)\n%s\n\t\t)'
             % (value, fnum(val_at[0]), fnum(val_at[1]), eff(1.27, hide=hide_value)))
    # Footprint
    s.append('\t\t(property "Footprint" "%s"\n\t\t\t(at %s %s 0)\n\t\t\t(show_name no)\n'
             '\t\t\t(do_not_autoplace no)\n%s\n\t\t)'
             % (footprint, fnum(x), fnum(y), eff(1.27, hide=True)))
    s.append('\t\t(property "Datasheet" "~"\n\t\t\t(at %s %s 0)\n\t\t\t(show_name no)\n'
             '\t\t\t(do_not_autoplace no)\n%s\n\t\t)'
             % (fnum(x), fnum(y), eff(1.27, hide=True)))
    s.append('\t\t(property "Description" ""\n\t\t\t(at %s %s 0)\n\t\t\t(show_name no)\n'
             '\t\t\t(do_not_autoplace no)\n%s\n\t\t)'
             % (fnum(x), fnum(y), eff(1.27, hide=True)))
    for pn in PINMAP[lib_id]:
        s.append('\t\t(pin "%s"\n\t\t\t(uuid "%s")\n\t\t)' % (pn, u()))
    s.append('\t\t(instances\n\t\t\t(project "%s"\n\t\t\t\t(path "/%s"\n\t\t\t\t\t(reference "%s")\n'
             '\t\t\t\t\t(unit 1)\n\t\t\t\t)\n\t\t\t)\n\t\t)' % (PROJECT, ROOT_UUID, ref))
    s.append('\t)')
    emit("\n".join(s))

pwr_n = [0]
def add_power(net, x, y, rot=0):
    pwr_n[0] += 1
    ref = ("#FLG0%02d" % pwr_n[0]) if net == "PWR_FLAG" else ("#PWR0%02d" % pwr_n[0])
    val = net if net != "PWR_FLAG" else "PWR_FLAG"
    add_symbol("power:%s" % net, ref, val, "", x, y, rot,
               hidden_ref=True, ref_at=(x, y - 2.54),
               val_at=(x, y + (2.54 if net != "GND" else 3.81)))

# ---------------------------------------------------------------- layout
# LED pins (rotation 0, y-up symbol -> screen y-down):
#   VDD(1)  -> (X,   Y-7.62)
#   DOUT(2) -> (X+7.62, Y)
#   VSS(3)  -> (X,   Y+7.62)
#   DIN(4)  -> (X-7.62, Y)
LEDX = lambda X: (X, X - 7.62, X + 7.62)  # vdd/vss x == X

H, V = 30.0, 26.0
GRID_ORIGIN = (40.0, 50.0)

def led_geo(i):
    """Serpentine placement; odd rows reversed + mirrored so DIN/DOUT face the
    chain direction while VDD/VSS stay top/bottom."""
    if i < 110:
        r, k = divmod(i, 11)
        c = k if r % 2 == 0 else 10 - k
        return GRID_ORIGIN[0] + c * H, GRID_ORIGIN[1] + r * V, (r % 2 == 1)
    return GRID_ORIGIN[0] + (i - 110) * H, GRID_ORIGIN[1] + 10 * V + 10.0, False

def led_pins(X, Y, mir):
    # VDD top / VSS bottom in both cases; mirror y swaps DIN <-> DOUT
    if mir:
        return dict(vdd=(X, Y - 7.62), vss=(X, Y + 7.62),
                    din=(X + 7.62, Y), dout=(X - 7.62, Y))
    return dict(vdd=(X, Y - 7.62), vss=(X, Y + 7.62),
                din=(X - 7.62, Y), dout=(X + 7.62, Y))

LED_FP = "LED_SMD:LED_WS2812B_PLCC4_5.0x5.0mm_P3.2mm"

P = []
for i in range(114):
    X, Y, mir = led_geo(i)
    add_symbol("LED:WS2812B", "D%d" % (i + 1), "WS2812B", LED_FP, X, Y,
               ref_at=(X - 6.5, Y - 12.7), val_at=(X + 6.5, Y + 11.5),
               hide_value=True, mirror=("y" if mir else None))
    p = led_pins(X, Y, mir)
    P.append(p)
    # net name on the incoming (DIN) node -> deterministic netlist names
    add_label("D%d" % i, p["din"][0], p["din"][1],
              rot=0 if mir else 180,
              justify="left bottom" if mir else "right bottom")

# --- explicit series wiring of the data chain (DOUT -> next DIN) ---
for i in range(113):
    add_wire(P[i]["dout"][0], P[i]["dout"][1],
             P[i + 1]["din"][0], P[i + 1]["din"][1])
add_label("DOUT", P[113]["dout"][0], P[113]["dout"][1])

# --- per-row power rails (+5V on VDD / GND on VSS) ---
for r in range(10):
    Y = GRID_ORIGIN[1] + r * V
    xs = [GRID_ORIGIN[0] + c * H for c in range(11)]
    for a, b in zip(xs, xs[1:]):
        add_wire(a, Y - 7.62, b, Y - 7.62)
        add_wire(a, Y + 7.62, b, Y + 7.62)
    add_label("+5V", xs[0], Y - 7.62)
    add_label("GND", xs[-1], Y + 7.62, rot=180, justify="right bottom")
# dots row
Yd = GRID_ORIGIN[1] + 10 * V + 10.0
xds = [GRID_ORIGIN[0] + k * H for k in range(4)]
for a, b in zip(xds, xds[1:]):
    add_wire(a, Yd - 7.62, b, Yd - 7.62)
    add_wire(a, Yd + 7.62, b, Yd + 7.62)
add_label("+5V", xds[0], Yd - 7.62)
add_label("GND", xds[-1], Yd + 7.62, rot=180, justify="right bottom")

# ---------------------------------------------------------------- bypass bank
CAPX0, CAPY0, CH, CV, CCOLS = 420.0, 50.0, 20.0, 24.0, 19
for k in range(114):
    r, c = divmod(k, CCOLS)
    X = CAPX0 + c * CH
    Y = CAPY0 + r * CV
    add_symbol("Device:C", "C%d" % (k + 1), "100nF",
               "Capacitor_SMD:C_0402_1005Metric", X, Y,
               ref_at=(X - 1.8, Y - 4.5), val_at=(X + 1.8, Y + 4.5), hide_value=True)
for r in range(6):
    Y = CAPY0 + r * CV
    xs = [CAPX0 + c * CH for c in range(CCOLS)]
    for a, b in zip(xs, xs[1:]):
        add_wire(a, Y - 3.81, b, Y - 3.81)
        add_wire(a, Y + 3.81, b, Y + 3.81)
    add_label("+5V", xs[0], Y - 3.81)
    add_label("GND", xs[-1], Y + 3.81, rot=180, justify="right bottom")

# ---------------------------------------------------------------- main circuit
add_symbol("Connector_Generic:Conn_01x04", "J1", "Conn_01x04 (ESP32-C5)",
           "Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical",
           470, 300, ref_at=(462, 288), val_at=(462, 312))
add_symbol("Connector_Generic:Conn_01x04", "J2", "Conn_01x04 (5V IN)",
           "Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical",
           470, 400, ref_at=(462, 388), val_at=(462, 412))

add_symbol("74xGxx:74AHCT1G125", "U1", "74AHCT1G125",
           "Package_TO_SOT_SMD:SOT-23-5", 640, 300,
           ref_at=(634, 286), val_at=(634, 314))
add_symbol("Device:R", "R1", "33", "Resistor_SMD:R_0603_1608Metric", 570, 370,
           ref_at=(566, 365.5), val_at=(566, 374.5))
add_symbol("Device:C", "C115", "100nF", "Capacitor_SMD:C_0603_1608Metric", 720, 300,
           ref_at=(718, 295.5), val_at=(718, 304.5))
add_symbol("Device:C", "C116", "22uF", "Capacitor_SMD:C_1210_3225Metric", 760, 300,
           ref_at=(758, 295.5), val_at=(758, 304.5))
add_symbol("Device:C_Polarized", "C117", "1000uF",
           "Capacitor_THT:CP_Radial_D10.0mm_P5.00mm", 800, 300,
           ref_at=(798, 295.5), val_at=(798, 304.5))
add_symbol("Jumper:SolderJumper_2_Open", "JP1", "5V_LINK",
           "Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm", 570, 430,
           ref_at=(566, 425.5), val_at=(566, 434.5))

# nets - labels on pins (no wires needed)
# J1: 1=+5V_DEV 2=GND 3=DIN_IN 4=GND
add_label("+5V_DEV", 470 - 5.08, 300 - 2.54, rot=180, justify="right bottom")
add_power("GND", 470 - 5.08, 300)
add_label("DIN_IN", 470 - 5.08, 300 + 2.54, rot=180, justify="right bottom")
add_power("GND", 470 - 5.08, 300 + 5.08)
# J2: 1,2=+5V  3,4=GND
add_power("+5V", 470 - 5.08, 400 - 2.54)
add_power("+5V", 470 - 5.08, 400)
add_power("GND", 470 - 5.08, 400 + 2.54)
add_power("GND", 470 - 5.08, 400 + 5.08)
# U1: 1 OE->GND  2 A->DIN_IN  3 GND  4 Y->S1  5 VCC->+5V
add_power("GND", 640, 300 - 10.16)
add_label("DIN_IN", 640 - 15.24, 300, rot=180, justify="right bottom")
add_power("GND", 640 - 5.08, 300 + 10.16)
add_label("S1", 640 + 12.7, 300)
add_power("+5V", 640 - 5.08, 300 - 10.16)
# R1: 1->S1  2->D0
add_label("S1", 570, 370 - 3.81)
add_label("D0", 570, 370 + 3.81)
# C115 / C116 / C117
for (x, y) in [(720, 300), (760, 300), (800, 300)]:
    add_power("+5V", x, y - 3.81)
    add_power("GND", x, y + 3.81)
# JP1: 1->+5V  2->+5V_DEV
add_power("+5V", 570 - 5.08, 430)
add_label("+5V_DEV", 570 + 5.08, 430)

# PWR_FLAGs (wire + label so they bind for sure)
for name, x in [("+5V", 500.0), ("GND", 540.0), ("+5V_DEV", 580.0)]:
    y = 500.0
    add_power("PWR_FLAG", x, y)    # flag pin at (x,y)
    add_wire(x, y, x + 7.62, y)
    add_label(name, x + 7.62, y)

# ---------------------------------------------------------------- serialize
head = []
head.append('(kicad_sch')
head.append('\t(version 20260101)')
head.append('\t(generator "eeschema")')
head.append('\t(generator_version "10.0")')
head.append('\t(uuid "%s")' % ROOT_UUID)
head.append('\t(paper "A1")')
head.append('\t(title_block')
head.append('\t\t(title "Word Clock LED Panel")')
head.append('\t\t(date "2026-09-10")')
head.append('\t\t(rev "v1")')
head.append('\t\t(comment 1 "ESP32-C5 + 114x WS2812B, 192x192mm, pitch 15.0mm")')
head.append('\t)')
head.append('\t(lib_symbols')
for name, txt in SYMS.items():
    head.append("\n".join("\t\t" + ln if ln.strip() else ln for ln in txt.splitlines()))
head.append('\t)')

body = "\n".join(parts)
tail = ('\t(sheet_instances\n\t\t(path "/"\n\t\t\t(page "1")\n\t\t)\n\t)\n'
        '\t(embedded_fonts no)\n)')

out = "\n".join(head) + "\n" + body + "\n" + tail + "\n"
with open(OUT, "w", encoding="utf-8") as f:
    f.write(out)
print("wrote %s (%d bytes, %d symbol/label/wire items)" % (OUT, len(out), len(parts)))
