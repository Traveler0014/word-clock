#!/usr/bin/env python3
"""字钟点阵调试服务（宿主机，零依赖）。

用途：浏览器里对照"固件应亮的字母"与"实物实际亮的字母"，一键回传观测，
      服务端立刻算出是哪种走线映射假设能解释它。

用法:
    python3 tools/debug_web.py                 # http://localhost:8787
    python3 tools/debug_web.py --port 9000 --host 127.0.0.1

页面功能:
  * 11×10 字母阵 + 4 角点，鼠标点选 = 标记"实物亮起"
  * 输入时刻 → 显示该时刻固件期望点亮的格子（蓝框），可与观测叠加
  * 底部 114 格"索引条带视图"：一眼看出整体错位/位移
  * 提交后服务端回报四种走线假设（蛇形/直排/反向蛇形/上下翻转）及 ±偏移
    的错误格子数，并把观测 JSON 落到 .debug/obs-*.json

注意：字库与 firmware/wordclock-core 保持一致，改 Rust 后请同步。
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

COLS, ROWS = 11, 10
LETTER_LEDS = COLS * ROWS
CORNER_LEDS = 4
TOTAL_LEDS = LETTER_LEDS + CORNER_LEDS

GRID = [
    "ITLISASAMPM", "ACQUARTERDC", "TWENTYFIVEX", "HALFSTENFTO",
    "PASTERUNINE", "ONESIXTHREE", "FOURFIVETWO", "EIGHTELEVEN",
    "SEVENTWELVE", "TENSEOCLOCK",
]

W = dict(
    IT=(0, 0, 2), IS=(0, 3, 2), A=(1, 0, 1), QUARTER=(1, 2, 7),
    TWENTY=(2, 0, 6), FIVE=(2, 6, 4), HALF=(3, 0, 4), TEN=(3, 5, 3),
    TO=(3, 9, 2), PAST=(4, 0, 4), OCLOCK=(9, 5, 6),
)
HOURS = [(5, 0, 3), (6, 8, 3), (5, 6, 5), (6, 0, 4), (6, 4, 4), (5, 3, 3),
         (8, 0, 5), (7, 0, 5), (4, 7, 4), (9, 0, 3), (7, 5, 6), (8, 5, 6)]
HOUR_NAMES = ["ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT",
              "NINE", "TEN", "ELEVEN", "TWELVE"]

HERE = os.path.dirname(os.path.abspath(__file__))
DEBUG_DIR = os.path.join(os.path.dirname(HERE), ".debug")


def _lan_url() -> str | None:
    """取本机对外 IP（WSL2 下即 Windows 能直连的地址）。"""
    import socket

    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("8.8.8.8", 80))  # 不会真的发包，只用来选出口网卡
        return s.getsockname()[0]
    except OSError:
        return None
    finally:
        s.close()


def xy_to_index(row: int, col: int) -> int:
    return row * COLS + (col if row % 2 == 0 else COLS - 1 - col)


def frame(hour24: int, minute: int) -> tuple[list[bool], list[str]]:
    """返回 (114 位点亮掩码, 参与显示的单词列表)。"""
    lit = [False] * TOTAL_LEDS
    words: list[str] = []

    def put(name: str, w):
        words.append(name)
        r, c, n = w
        for i in range(n):
            lit[xy_to_index(r, c + i)] = True

    put("IT", W["IT"])
    put("IS", W["IS"])
    step, dots = minute // 5, minute % 5
    h = hour24 % 12 or 12
    if step >= 7:
        h = h % 12 + 1
    hw, hname = HOURS[h - 1], HOUR_NAMES[h - 1]
    plan = {
        0: (("OCLOCK", W["OCLOCK"]),),
        1: (("FIVE", W["FIVE"]), ("PAST", W["PAST"])),
        2: (("TEN", W["TEN"]), ("PAST", W["PAST"])),
        3: (("A", W["A"]), ("QUARTER", W["QUARTER"]), ("PAST", W["PAST"])),
        4: (("TWENTY", W["TWENTY"]), ("PAST", W["PAST"])),
        5: (("TWENTY", W["TWENTY"]), ("FIVE", W["FIVE"]), ("PAST", W["PAST"])),
        6: (("HALF", W["HALF"]), ("PAST", W["PAST"])),
        7: (("TWENTY", W["TWENTY"]), ("FIVE", W["FIVE"]), ("TO", W["TO"])),
        8: (("TWENTY", W["TWENTY"]), ("TO", W["TO"])),
        9: (("A", W["A"]), ("QUARTER", W["QUARTER"]), ("TO", W["TO"])),
        10: (("TEN", W["TEN"]), ("TO", W["TO"])),
        11: (("FIVE", W["FIVE"]), ("TO", W["TO"])),
    }[step]
    for name, w in plan:
        put(name, w)
    put(hname, hw)
    for i in range(dots):
        lit[LETTER_LEDS + i] = True
    return lit, words


def _label_of(i: int, mode: str) -> int:
    """LED 索引 i 在走线假设 mode 下，落在哪个物理格子上 → 返回该格子的 UI 标签索引。

    坐标系约定（很重要，弄反了结果会全错）：
      * expected[] 与观测标记都用 **UI 标签索引** = 蛇形编号（与固件 xy_to_index 一致）；
        即“标签 j 的格子”位于物理 (r, c)，j = r*11 + (c if r 偶数 else 10-c)。
      * 硬件真实走线 mode 决定 LED i 落在哪个物理格子上；本函数把它换回标签空间，
        这样才能和观测直接逐位比较。
    """
    if i >= LETTER_LEDS:
        return i  # 角点不做置换
    r, c = i // COLS, i % COLS
    if mode == "serp":              # 硬件也是蛇形（与固件假设一致）
        pr, pc = r, (c if r % 2 == 0 else COLS - 1 - c)
    elif mode == "straight":        # 硬件直排：每行都左→右
        pr, pc = r, c
    elif mode == "serp_rev":        # 蛇形但起始方向相反
        pr, pc = r, (COLS - 1 - c if r % 2 == 0 else c)
    elif mode == "rows_flipped":    # 上下翻转 + 蛇形
        pr, pc = ROWS - 1 - r, (c if r % 2 == 0 else COLS - 1 - c)
    else:
        raise ValueError(mode)
    return pr * COLS + (pc if pr % 2 == 0 else COLS - 1 - pc)


HYPOTHESES = ["serp", "straight", "serp_rev", "rows_flipped"]


def analyse(expected: list[bool], observed: list[bool]) -> list[dict]:
    """对每种走线假设/偏移，算“按该假设应看到的标签集合”与观测的差异。"""
    out = []
    for mode in HYPOTHESES:
        for shift in (0, 1, -1, 2, -2, 3, -3):
            transformed = [False] * TOTAL_LEDS
            for i in range(LETTER_LEDS):
                if not expected[i]:
                    continue
                j = (i + shift) % LETTER_LEDS
                transformed[_label_of(j, mode)] = True
            for i in range(LETTER_LEDS, TOTAL_LEDS):
                if expected[i]:
                    transformed[i] = True
            wrong = sum(1 for a, b in zip(transformed, observed) if a != b)
            out.append({"mode": mode, "shift": shift, "wrong": wrong,
                        "ok": wrong == 0})
    out.sort(key=lambda d: d["wrong"])
    return out


class Handler(BaseHTTPRequestHandler):
    server_version = "WordClockDebug/1.0"

    def log_message(self, fmt, *a):  # 只记 POST（提交观测），避免刷屏
        if getattr(self, "command", "") == "POST":
            print(f"[{dt.datetime.now():%H:%M:%S}] {self.command} {self.path} -> {a[1] if len(a) > 1 else ''}", flush=True)

    def _send(self, code: int, body: bytes, ctype: str):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def _json(self, obj, code: int = 200):
        self._send(code, json.dumps(obj, ensure_ascii=False).encode(), "application/json; charset=utf-8")

    def do_GET(self):
        u = urlparse(self.path)
        if u.path in ("/", "/index.html"):
            with open(os.path.join(HERE, "debug_web.html"), "rb") as f:
                return self._send(200, f.read(), "text/html; charset=utf-8")
        if u.path == "/api/frame":
            q = parse_qs(u.query)
            at = q.get("at", [""])[0]
            if at:
                try:
                    hh, mm = (int(x) for x in at.split(":"))
                except ValueError:
                    return self._json({"error": "at 需要 HH:MM"}, 400)
            else:
                now = dt.datetime.now()
                hh, mm = now.hour, now.minute
            lit, words = frame(hh, mm)
            return self._json({
                "at": f"{hh:02d}:{mm:02d}",
                "grid": GRID, "cols": COLS, "rows": ROWS,
                "letters": LETTER_LEDS, "total": TOTAL_LEDS,
                "expected": lit, "words": words,
                "serverTime": dt.datetime.now().strftime("%F %T"),
            })
        if u.path == "/api/last":
            files = sorted(f for f in (os.listdir(DEBUG_DIR) if os.path.isdir(DEBUG_DIR) else [])
                           if f.startswith("obs-"))
            if not files:
                return self._json({"error": "no observation yet"}, 404)
            with open(os.path.join(DEBUG_DIR, files[-1])) as f:
                return self._send(200, f.read().encode(), "application/json; charset=utf-8")
        return self._json({"error": "not found"}, 404)

    def do_POST(self):
        if urlparse(self.path).path != "/api/report":
            return self._json({"error": "not found"}, 404)
        n = int(self.headers.get("Content-Length", "0"))
        try:
            payload = json.loads(self.rfile.read(n) or b"{}")
            # 容错：前端可能传稀疏/短数组（只点了部分格子），统一补成 114 位稠密
            raw = payload.get("observed") or []
            observed = [bool(x) for x in raw[:TOTAL_LEDS]]
            observed += [False] * (TOTAL_LEDS - len(observed))
            at = payload.get("at") or dt.datetime.now().strftime("%H:%M")
            hh, mm = (int(x) for x in at.split(":"))
        except Exception as e:  # noqa: BLE001
            return self._json({"error": f"bad payload: {e}"}, 400)

        expected, words = frame(hh, mm)
        result = analyse(expected, observed)
        os.makedirs(DEBUG_DIR, exist_ok=True)
        stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
        record = {
            "savedAt": dt.datetime.now().isoformat(timespec="seconds"),
            "at": f"{hh:02d}:{mm:02d}", "words": words,
            "expected": expected, "observed": observed,
            "observedCount": sum(observed), "expectedCount": sum(expected),
            "ranking": result[:8],
        }
        path = os.path.join(DEBUG_DIR, f"obs-{stamp}.json")
        with open(path, "w") as f:
            json.dump(record, f, ensure_ascii=False, indent=1)
        print(f"[{dt.datetime.now():%H:%M:%S}] 保存观测 {path} "
              f"(at={record['at']} 期望{record['expectedCount']}格 / 实测{record['observedCount']}格 "
              f"最优={result[0]['mode']} shift={result[0]['shift']:+d} wrong={result[0]['wrong']})", flush=True)
        return self._json({"saved": path, "at": f"{hh:02d}:{mm:02d}",
                           "words": words, "ranking": result[:8],
                           "observedCount": sum(observed),
                           "expectedCount": sum(expected)})


def main() -> int:
    ap = argparse.ArgumentParser(description="字钟点阵调试页")
    ap.add_argument("--host", default="0.0.0.0")
    ap.add_argument("--port", type=int, default=8787)
    args = ap.parse_args()
    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    url = _lan_url()
    print(f"字钟调试页: http://localhost:{args.port}/  (Ctrl-C 退出)")
    if url:
        # WSL2 的 localhost 转发在本机可能不通，Windows 浏览器请直接用这个地址
        print(f"Windows 浏览器请用: http://{url}:{args.port}/")
    print(f"观测落盘目录: {DEBUG_DIR}")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\nbye")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
