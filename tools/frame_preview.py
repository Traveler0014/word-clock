#!/usr/bin/env python3
"""点阵预览 / 映射诊断。

用途（宿主机，不改固件）:
  1) 打印任意时刻"固件应该点亮"的字母阵，肉眼和实物对比；
  2) 把你在实物上看到的点亮情况（10 行 × 11 列）粘进来，反查是哪种走线映射
     假设能解释它 —— 用来定位"蛇形/直排/行翻转/条带偏移"这类映射 bug。

字库逻辑与 firmware/wordclock-core/src/lib.rs 保持一致（改 Rust 后请同步本文件）。

用法:
    python3 tools/frame_preview.py                # 当前时间（本机时区）
    python3 tools/frame_preview.py --at 18:24     # 指定时间
    python3 tools/frame_preview.py --at 22:35 --hex   # 另附 114 位索引表
    python3 tools/frame_preview.py --decode       # 交互输入实物观测，做映射反查
    python3 tools/frame_preview.py --decode --obs "ITLISASAMPM .CQUARTERDC ..."
"""

from __future__ import annotations

import argparse
import datetime as dt
import sys

GRID = [
    "ITLISASAMPM",
    "ACQUARTERDC",
    "TWENTYFIVEX",
    "HALFSTENFTO",
    "PASTERUNINE",
    "ONESIXTHREE",
    "FOURFIVETWO",
    "EIGHTELEVEN",
    "SEVENTWELVE",
    "TENSEOCLOCK",
]
COLS, ROWS = 11, 10
LETTER_LEDS = COLS * ROWS      # 110
CORNER_LEDS = 4
TOTAL_LEDS = LETTER_LEDS + CORNER_LEDS  # 114

W = dict(
    IT=(0, 0, 2), IS=(0, 3, 2), A=(1, 0, 1), QUARTER=(1, 2, 7),
    TWENTY=(2, 0, 6), FIVE=(2, 6, 4), HALF=(3, 0, 4), TEN=(3, 5, 3),
    TO=(3, 9, 2), PAST=(4, 0, 4), OCLOCK=(9, 5, 6),
)
HOURS = [(5, 0, 3), (6, 8, 3), (5, 6, 5), (6, 0, 4), (6, 4, 4), (5, 3, 3),
         (8, 0, 5), (7, 0, 5), (4, 7, 4), (9, 0, 3), (7, 5, 6), (8, 5, 6)]


def xy_to_index(row: int, col: int) -> int:
    """蛇形走线（固件假设）：偶数行左→右，奇数行右→左。"""
    return row * COLS + (col if row % 2 == 0 else COLS - 1 - col)


def frame(hour24: int, minute: int) -> tuple[set[int], int]:
    """返回 (点亮的 LED 索引集合, 角点数量)。"""
    lit: set[int] = set()

    def put(w):
        r, c, n = w
        for i in range(n):
            lit.add(xy_to_index(r, c + i))

    put(W["IT"]); put(W["IS"])
    step, dots = minute // 5, minute % 5
    h = hour24 % 12 or 12
    if step >= 7:
        h = h % 12 + 1
    hw = HOURS[h - 1]
    plan = {
        0: (hw, W["OCLOCK"]),
        1: (W["FIVE"], W["PAST"], hw),
        2: (W["TEN"], W["PAST"], hw),
        3: (W["A"], W["QUARTER"], W["PAST"], hw),
        4: (W["TWENTY"], W["PAST"], hw),
        5: (W["TWENTY"], W["FIVE"], W["PAST"], hw),
        6: (W["HALF"], W["PAST"], hw),
        7: (W["TWENTY"], W["FIVE"], W["TO"], hw),
        8: (W["TWENTY"], W["TO"], hw),
        9: (W["A"], W["QUARTER"], W["TO"], hw),
        10: (W["TEN"], W["TO"], hw),
        11: (W["FIVE"], W["TO"], hw),
    }[step]
    for w in plan:
        put(w)
    return lit, dots


def render(lit: set[int], dots: int, physical: str = "serp") -> list[str]:
    """把点亮索引集合渲染成 10 行字符串（点亮的字母保留，其余为 '.'）。"""
    rows = [["." for _ in range(COLS)] for _ in range(ROWS)]
    for i in range(LETTER_LEDS):
        if i not in lit:
            continue
        r, c = i // COLS, i % COLS
        if physical == "straight":      # 硬件直排：索引顺序 = 行内左→右
            pr, pc = r, c
        elif physical == "serp_rev":    # 蛇形但起始方向相反
            pr, pc = r, (c if r % 2 else COLS - 1 - c)
        elif physical == "rows_flipped":  # 上下翻转 + 蛇形
            pr, pc = ROWS - 1 - r, (c if r % 2 == 0 else COLS - 1 - c)
        else:                            # serp：与固件假设一致
            pr, pc = r, (c if r % 2 == 0 else COLS - 1 - c)
        rows[pr][pc] = GRID[pr][pc]
    return ["".join(r) for r in rows]


def show(title: str, rows: list[str], lit: set[int], dots: int) -> None:
    print(f"-- {title}")
    print("   " + "".join(str(c % 10) for c in range(COLS)))
    for n, r in enumerate(rows):
        print(f"{n:2d} {r}")
    letters = "".join(ch for r in rows for ch in r if ch != ".")
    print(f"   点亮 {sum(1 for i in lit if i < LETTER_LEDS)} 字母 + {dots} 角点"
          f" = {len([i for i in lit if i < LETTER_LEDS]) + dots}")
    print(f"   字母序列: {letters}")


def parse_obs(text: str) -> list[str]:
    """把观测（10 行 × 11 列，亮的写字母、不亮的写 . 或 _）规整成 10×11。"""
    chars = [ch.upper() for ch in text if ch.isalpha() or ch in "._"]
    chars = ["." if ch == "_" else ch for ch in chars]
    if len(chars) != LETTER_LEDS:
        sys.exit(f"观测需要 {LETTER_LEDS} 个字符（10 行 × 11 列），收到 {len(chars)} 个")
    return ["".join(chars[i * COLS:(i + 1) * COLS]) for i in range(ROWS)]


def decode(obs: list[str], hour: int, minute: int) -> None:
    lit, dots = frame(hour, minute)
    print(f"-- 反查 {hour:02d}:{minute:02d} 的观测")
    print("   观测:")
    for r in obs:
        print("     " + r)
    for phys in ("serp", "straight", "serp_rev", "rows_flipped"):
        exp = render(lit, dots, phys)
        diff = [(n, e, o) for n, (e, o) in enumerate(zip(exp, obs)) if e != o]
        tag = "完全一致 ✓" if not diff else f"{len(diff)} 行不一致"
        print(f"\n   假设 {phys:14s}: {tag}")
        if diff:
            for n, e, o in diff[:4]:
                print(f"     行{n} 期望 {e}")
                print(f"     行{n} 实际 {o}")
        else:
            print("     → 实物走线就是这个假设")
            return


def main() -> int:
    ap = argparse.ArgumentParser(description="字钟点阵预览 / 映射反查")
    ap.add_argument("--at", help="HH:MM（24 小时制），默认当前时间")
    ap.add_argument("--hex", action="store_true", help="附加 114 位索引表")
    ap.add_argument("--decode", action="store_true", help="用观测反查走线假设")
    ap.add_argument("--obs", help="观测字符串（配合 --decode，10×11 字符）")
    args = ap.parse_args()

    if args.at:
        hh, mm = (int(x) for x in args.at.split(":"))
    else:
        now = dt.datetime.now()
        hh, mm = now.hour, now.minute

    if args.decode:
        text = args.obs
        if not text:
            print("粘贴实物观测：10 行，每行 11 个字符（亮的写字母，不亮写 . ）", file=sys.stderr)
            print("例如: ITLIS.S.S.PM / ACQUARTERDC / ...", file=sys.stderr)
            text = sys.stdin.read()
        decode(parse_obs(text), hh, mm)
        return 0

    lit, dots = frame(hh, mm)
    show(f"{hh:02d}:{mm:02d} 期望点阵 (serpentine)", render(lit, dots), lit, dots)
    if args.hex:
        cells = [xy_to_index(i // COLS, i % COLS) for i in range(LETTER_LEDS)]
        print("   索引→(行,列): " + " ".join(
            f"{i}:{i // COLS},{i % COLS}" for i in range(0, TOTAL_LEDS, 11)))
        print(f"   角点索引: {LETTER_LEDS}..{TOTAL_LEDS - 1}")
        del cells
    return 0


if __name__ == "__main__":
    sys.exit(main())
