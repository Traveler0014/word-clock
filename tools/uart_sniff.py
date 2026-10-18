#!/usr/bin/env python3
"""UART 日志嗅探 —— 字钟项目常用常驻工具。

用法:
    python3 tools/uart_sniff.py                 # 自动找 UART 口（VID 1a86:55d3）
    python3 tools/uart_sniff.py --log uart.log  # 同时落盘
    python3 tools/uart_sniff.py --port /dev/ttyACM1 --baud 115200
    python3 tools/uart_sniff.py --grep word_clock   # 只打印匹配行

关键点（踩过的坑，勿改）:
  * DTR/RTS 必须在 open() 之前就置为 False。
    ESP32 devkit 的自动复位电路把 DTR/RTS 接到 EN/BOOT，拉住会复位芯片
    甚至摁进下载模式 —— 表现是日志每开一次就从 `word-clock boot` 重来。
  * pyserial 的 Serial(port, dtr=False) 会报 ValueError（POSIX 不支持 open 时传），
    所以必须先构造后 open，再预置 dtr/rts。
  * 端口号会漂移（ttyACM0/ttyACM1 取决于枚举顺序），因此默认按 VID:PID 查找：
    303a:1001 = USB-JTAG（烧录/调试），1a86:55d3 = UART（日志，本工具默认）。
"""

from __future__ import annotations

import argparse
import glob
import os
import sys
import time

try:
    import serial
except ImportError:  # pragma: no cover
    sys.exit("需要 pyserial: pip install pyserial")

UART_VID_PID = ("1a86", "55d3")  # CH343/CH9102F，接 UART0 日志


def _read_id(path: str) -> str | None:
    try:
        with open(path) as f:
            return f.read().strip().lower()
    except OSError:
        return None


def find_port(vid: str, pid: str) -> str | None:
    """按 USB VID:PID 找 /dev/ttyACM* / /dev/ttyUSB*。"""
    for dev in sorted(glob.glob("/dev/ttyACM*") + glob.glob("/dev/ttyUSB*")):
        name = os.path.basename(dev)
        base = f"/sys/class/tty/{name}/device"
        # base 是 usb 接口目录（.../1-1:1.0）；idVendor/idProduct 在其父目录（usb 设备）
        for cand in (f"{base}/../idVendor", f"{base}/../../idVendor"):
            v = _read_id(cand)
            if v is None:
                continue
            p = _read_id(os.path.join(os.path.dirname(cand), "idProduct"))
            if v == vid and p == pid:
                return dev
    return None


def open_serial(port: str, baud: int) -> "serial.Serial":
    """打开串口且不触碰 DTR/RTS（见文件头说明）。"""
    s = serial.Serial()
    s.port = port
    s.baudrate = baud
    s.timeout = 0.2
    s.dtr = False  # 必须在 open() 之前
    s.rts = False
    s.open()
    return s


def main() -> int:
    ap = argparse.ArgumentParser(description="字钟 UART 日志嗅探")
    ap.add_argument("--port", help="显式指定串口，默认按 VID:PID 自动查找")
    ap.add_argument("--baud", type=int, default=115200)
    ap.add_argument("--log", help="同时追加写入该文件（原始行，无时间戳前缀）")
    ap.add_argument("--grep", help="只打印包含该子串的行（大小写敏感）")
    ap.add_argument("--quiet", action="store_true", help="不打印，只落盘")
    args = ap.parse_args()

    port = args.port or find_port(*UART_VID_PID)
    if not port:
        print(
            f"未找到 UART 口 (VID:PID={':'.join(UART_VID_PID)})。\n"
            "检查: Windows 侧 `usbipd list` 是否为 Attached、WSL 侧 `lsusb`、"
            "以及 scripts/windows/usb-up.ps1 是否跑过。",
            file=sys.stderr,
        )
        return 1

    try:
        s = open_serial(port, args.baud)
    except serial.SerialException as e:
        print(f"打开 {port} 失败: {e}", file=sys.stderr)
        return 2

    logf = open(args.log, "a", buffering=1) if args.log else None
    print(f"# listening on {port} @ {args.baud} (dtr/rts=False)  [Ctrl-C 退出]", file=sys.stderr)

    buf = b""
    try:
        while True:
            chunk = s.read(4096)
            if not chunk:
                continue
            buf += chunk
            while b"\n" in buf:
                raw, buf = buf.split(b"\n", 1)
                line = raw.rstrip(b"\r").decode("utf-8", "replace")
                stamp = time.strftime("%H:%M:%S")
                if logf:
                    logf.write(line + "\n")
                if not args.quiet and (args.grep is None or args.grep in line):
                    print(f"{stamp} {line}", flush=True)
    except KeyboardInterrupt:
        print("\n# bye", file=sys.stderr)
        return 0
    finally:
        s.close()
        if logf:
            logf.close()


if __name__ == "__main__":
    sys.exit(main())
