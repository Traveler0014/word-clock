#!/usr/bin/env python3
"""UART 日志监听（CH9102F 口，开发期常驻）。

- 自动发现 /dev/ttyACM*（枚举编号会变）并自动重连
- dtr/rts 必须为 False：CH9102F 的 DTR/RTS 接自动下载电路，拉住会把
  IO0 摁低，芯片每次复位都进下载模式（烧完不跑、日志静默）
- 用法：python3 tools/uart_sniff.py
- 烧录前必须停掉本脚本（占用 USB-JTAG 口会令 espflash 同步失败；
  本脚本只碰 UART 口，但若枚举编号漂移到 JTAG 口需留意）
"""
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
