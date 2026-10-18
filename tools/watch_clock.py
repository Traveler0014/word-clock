#!/usr/bin/env python3
"""持续校核固件的时间帧日志（配合 tools/uart_sniff.py --log 用）。

固件每分钟打一行 `HH:MM -> N leds`。本工具用同样的字库逻辑独立算出该时刻
**应该**有几个灯亮（字母 + 角点），逐行比对，并检测是否漏分钟 —— 用来在
长时间观察中发现"字库/时间跳变/掉帧"类问题，不需要盯着面板。

用法:
    python3 tools/uart_sniff.py --log /tmp/wc.log &     # 先抓日志
    python3 tools/watch_clock.py --log /tmp/wc.log      # 再持续校核（Ctrl-C 退出）
    python3 tools/watch_clock.py --log /tmp/wc.log --from-start   # 从头回放
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import re
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("frame_preview", os.path.join(HERE, "frame_preview.py"))
fp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(fp)

LINE = re.compile(r"(\d{2}):(\d{2})\s*->\s*(\d+)\s*leds")
WORDS_RE = re.compile(r"word_clock: (\d{2}:\d{2})")


def expected_count(hour: int, minute: int) -> int:
    lit, dots = fp.frame(hour, minute)
    return len([i for i in lit if i < fp.LETTER_LEDS]) + dots


def describe(hour: int, minute: int) -> str:
    lit, dots = fp.frame(hour, minute)
    rows = fp.render(lit, dots)
    return "".join(ch for r in rows for ch in r if ch != ".")


def main() -> int:
    ap = argparse.ArgumentParser(description="校核固件时间帧日志")
    ap.add_argument("--log", required=True, help="uart_sniff --log 写出的文件")
    ap.add_argument("--from-start", action="store_true", help="从头回放而不是只看新行")
    ap.add_argument("--interval", type=float, default=1.0, help="轮询间隔秒")
    args = ap.parse_args()

    if not os.path.exists(args.log):
        return print(f"日志不存在: {args.log}", file=sys.stderr) or 1

    with open(args.log, "r") as f:
        if not args.from_start:
            f.seek(0, os.SEEK_END)

        ok = bad = 0
        last_minutes = None
        print(f"# 校核 {args.log}（Ctrl-C 退出）")
        try:
            while True:
                line = f.readline()
                if not line:
                    time.sleep(args.interval)
                    continue
                m = LINE.search(line)
                if not m:
                    continue
                hh, mm, n = int(m.group(1)), int(m.group(2)), int(m.group(3))
                exp = expected_count(hh, mm)
                words = describe(hh, mm)
                stamps = hh * 60 + mm

                gap = ""
                if last_minutes is not None:
                    d = (stamps - last_minutes) % 1440
                    if d > 1:
                        gap = f"  ⚠ 漏了 {d - 1} 分钟"
                last_minutes = stamps

                if n == exp:
                    ok += 1
                    print(f"  ✓ {hh:02d}:{mm:02d} {n:3d} 灯  {words}{gap}")
                else:
                    bad += 1
                    print(f"  ✗ {hh:02d}:{mm:02d} {n:3d} 灯（期望 {exp}）{words}{gap}  <<< 不一致")
                print(f"     累计 ok={ok} bad={bad}", end="\r")
        except KeyboardInterrupt:
            print(f"\n# 结束：ok={ok} bad={bad}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
