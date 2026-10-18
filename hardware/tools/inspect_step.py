#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
inspect_step.py — 列出 PCBA STEP 模型里每个元件实例的包围盒（世界坐标）。

用途：核对元件高度 / 占位 / 位置（见 docs/pcba-heights.md）。

依赖 OpenCASCADE 的 python 绑定（cadquery-ocp 自带）：
    uv venv /tmp/steptool --python 3.11
    uv pip install --python /tmp/steptool/bin/python cadquery-ocp
    /tmp/steptool/bin/python hardware/tools/inspect_step.py hardware/pcba.step

注意：EasyEDA 导出的 STEP 对**插件**（排针/电解）只有焊盘占位方块（0.76mm），
插件本体高度要查 LCSC 规格书。
"""
import sys
import json

from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.XCAFApp import XCAFApp_Application
from OCP.XCAFDoc import XCAFDoc_DocumentTool
from OCP.collections import Sequence_TDF_Label
from OCP.TDataStd import TDataStd_Name
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TDF import TDF_Label
from OCP.TopLoc import TopLoc_Location


def name_of(label):
    a = TDataStd_Name()
    try:
        if label.FindAttribute(TDataStd_Name.GetID_s(), a):
            return a.Get().ToExtString()
    except Exception:
        pass
    return "<noname>"


def bbox(shape):
    b = Bnd_Box()
    BRepBndLib.Add_s(shape, b, True)
    if b.IsVoid():
        return None
    return (b.GetXMin(), b.GetYMin(), b.GetZMin(),
            b.GetXMax(), b.GetYMax(), b.GetZMax())


def main(path):
    app = XCAFApp_Application.GetApplication_s()
    doc = TDocStd_Document(TCollection_ExtendedString("MDTV-XCAF"))
    app.NewDocument(TCollection_ExtendedString("MDTV-XCAF"), doc)

    reader = STEPCAFControl_Reader()
    reader.SetNameMode(True)
    if reader.ReadFile(path) != 1:
        raise SystemExit("ReadFile failed")
    reader.Transfer(doc)

    st = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
    rows = []

    def walk(label, stack, loc):
        n = name_of(label)
        stack = stack + [n]
        try:
            loc = st.GetLocation_s(label).Multiplied(loc)
        except Exception:
            pass
        if st.IsComponent_s(label):          # 元件实例 → 递归到被引用的形状
            ref = TDF_Label()
            if st.GetReferredShape_s(label, ref):
                walk(ref, stack[:-1], loc)
                return
        if st.IsAssembly_s(label):
            subs = Sequence_TDF_Label()
            if st.GetComponents_s(label, subs):
                for s in list(subs):
                    walk(s, stack, loc)
            return
        try:
            sh = st.GetShape_s(label)
            if sh is not None and not sh.IsNull():
                if not loc.IsIdentity():
                    sh = sh.Moved(loc)
                bb = bbox(sh)
                if bb:
                    rows.append((stack, bb))
        except Exception as e:
            print("ERR", n, e, file=sys.stderr)

    free = Sequence_TDF_Label()
    st.GetFreeShapes(free)
    for f in list(free):
        walk(f, [], TopLoc_Location())

    out = []
    for stack, bb in rows:
        x0, y0, z0, x1, y1, z1 = bb
        out.append({
            "name": " / ".join(stack),
            "xmin": round(x0, 3), "ymin": round(y0, 3), "zmin": round(z0, 3),
            "xmax": round(x1, 3), "ymax": round(y1, 3), "zmax": round(z1, 3),
            "dx": round(x1 - x0, 3), "dy": round(y1 - y0, 3), "dz": round(z1 - z0, 3),
        })

    print(f"{'name':70s} {'x':>17s} {'y':>17s} {'z':>15s}   dz")
    for r in out:
        print(f"{r['name'][:70]:70s} "
              f"[{r['xmin']:7.2f},{r['xmax']:7.2f}] "
              f"[{r['ymin']:7.2f},{r['ymax']:7.2f}] "
              f"[{r['zmin']:6.2f},{r['zmax']:6.2f}] {r['dz']:6.3f}")
    print(f"# {len(out)} leaf parts")

    with open("step_boxes.json", "w") as f:
        json.dump(out, f, indent=1)
    print("# wrote step_boxes.json")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "hardware/pcba.step")
