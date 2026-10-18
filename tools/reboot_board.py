#!/usr/bin/env python3
"""复位板子 —— 用来观察固件开机自检（全链 红→绿→蓝→灭，各 400ms）。

用途：判断"灯链是否整条都通"。
  * 整屏依次闪红/绿/蓝 → 链通，问题在数据位（信号完整性 / 电平 / 串阻）
  * 只有上面几行闪      → 链断在那里之后（行间 DOUT→DIN 焊点 / 杜邦线 / 供电）

用法:
    python3 tools/reboot_board.py           # 复位，然后盯着面板看
    python3 tools/reboot_board.py --watch   # 复位并顺带抓 15s UART 日志

原理：ESP32 devkit 的自动复位电路把 DTR/RTS 接到 EN/BOOT，打开串口本身就会
产生一次复位脉冲（见 tools/uart_sniff.py 的说明），所以不需要额外协议。
"""

from __future__ import annotations

import argparse
import sys
import time

try:
    import serial
except ImportError:  # pragma: no cover
    sys.exit("需要 pyserial: pip install pyserial")

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from uart_sniff import UART_VID_PID, find_port  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description="复位板子看自检闪灯")
    ap.add_argument("--port", help="默认按 VID:PID 自动找")
    ap.add_argument("--watch", action="store_true", help="复位后抓日志")
    ap.add_argument("--secs", type=float, default=15.0)
    args = ap.parse_args()

    port = args.port or find_port(*UART_VID_PID)
    if not port:
        print("未找到 UART 口（usbipd attach 了吗？）", file=sys.stderr)
        return 1

    s = serial.Serial()
    s.port = port
    s.baudrate = 115200
    s.timeout = 0.2
    s.dtr = False
    s.rts = False
    s.open()  # ← 打开瞬间产生复位脉冲
    print(f"已复位 ({port}) @ {time.strftime('%H:%M:%S')}")
    print("盯面板：应整屏 红 0.4s → 绿 0.4s → 蓝 0.4s → 全灭")
    if not args.watch:
        time.sleep(0.3)
        s.close()
        return 0

    buf = b""
    t0 = time.time()
    try:
        while time.time() - t0 < args.secs:
            chunk = s.read(4096)
            if not chunk:
                continue
            buf += chunk
            while b"\n" in buf:
                raw, buf = buf.split(b"\n", 1)
                line = raw.rstrip(b"\r").decode("utf-8", "replace")
                if "word_clock" in line or line.startswith(("E (", "W (rmt", "I (boot")):
                    print(f"{time.strftime('%H:%M:%S')} {line}", flush=True)
    except KeyboardInterrupt:
        pass
    finally:
        s.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
