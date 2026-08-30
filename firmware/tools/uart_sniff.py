#!/usr/bin/env python3
"""持久监听：设备断开自动重开，自动匹配任意 /dev/ttyACM*（编号会变）。"""
import glob
import sys
import time

import serial

BAUD = 115200


def find_port():
    ports = sorted(glob.glob("/dev/ttyACM*"))
    return ports[0] if ports else None


while True:
    port = find_port()
    if not port:
        print(f"[sniff] no ttyACM* {time.strftime('%H:%M:%S')}, retrying...", flush=True)
        time.sleep(1)
        continue
    try:
        ser = serial.Serial(port, BAUD, timeout=1)
        # UART 口（CH9102F）严禁拉 DTR/RTS：自动下载电路会拽住 IO0/EN，
        # 把芯片摁在下载模式（表现为“烧完不跑、日志静默”）
        ser.dtr = False
        ser.rts = False
        print(f"[sniff] connected {port} {time.strftime('%H:%M:%S')}", flush=True)
        while True:
            data = ser.read(512)
            if data:
                sys.stdout.write(data.decode(errors="replace"))
                sys.stdout.flush()
    except (serial.SerialException, OSError):
        print(f"[sniff] port lost {time.strftime('%H:%M:%S')}, retrying...", flush=True)
        time.sleep(1)
