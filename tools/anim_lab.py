#!/usr/bin/env python3
"""anim_lab 静态服务器：把 tools/anim_lab.html 送到浏览器，并提供参数存档接口。

为什么需要它：WSL2 里浏览器在 Windows 侧，file:// 跨不过去；
另外动效参数要能"存一份"给固件实现对照。

用法:
    python3 tools/anim_lab.py            # http://<本机IP>:8901/
    python3 tools/anim_lab.py --port 9000

接口（给将来"API 传动效"预留的种子）:
    GET  /                  → 实验台页面
    GET  /params            → 最近一次保存的参数 JSON
    POST /params            → 覆盖保存（body 为 JSON）
    GET  /golden/<name>.csv → 已导出的 golden 帧（若存在）
"""
from __future__ import annotations

import argparse
import json
import socket
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
PAGE = HERE / "anim_lab.html"
STORE = HERE / "anim_lab.params.json"


def lan_ip() -> str:
    """取默认路由出口地址（不真的发包）。"""
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("10.255.255.255", 1))
        return s.getsockname()[0]
    except OSError:
        return "127.0.0.1"
    finally:
        s.close()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _send(self, code: int, body: bytes, ctype: str) -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802
        path = self.path.split("?")[0]
        if path in ("/", "/index.html", "/anim_lab.html"):
            self._send(200, PAGE.read_bytes(), "text/html; charset=utf-8")
        elif path == "/params":
            body = STORE.read_bytes() if STORE.exists() else b"{}"
            self._send(200, body, "application/json; charset=utf-8")
        else:
            self._send(404, b"not found\n", "text/plain; charset=utf-8")

    def do_POST(self) -> None:  # noqa: N802
        if self.path.split("?")[0] != "/params":
            self._send(404, b"not found\n", "text/plain; charset=utf-8")
            return
        length = int(self.headers.get("Content-Length") or 0)
        raw = self.rfile.read(length)
        try:
            json.loads(raw)  # 只做合法性校验
        except json.JSONDecodeError as exc:
            self._send(400, f"bad json: {exc}\n".encode(), "text/plain; charset=utf-8")
            return
        STORE.write_bytes(raw)
        self._send(200, b'{"ok":true}', "application/json; charset=utf-8")

    def log_message(self, fmt: str, *args) -> None:
        print(f"[anim_lab] {self.address_string()} {fmt % args}", flush=True)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8901)
    ap.add_argument("--bind", default="0.0.0.0")
    args = ap.parse_args()
    if not PAGE.exists():
        raise SystemExit(f"找不到 {PAGE}")
    srv = ThreadingHTTPServer((args.bind, args.port), Handler)
    print(f"动效实验台 → http://{lan_ip()}:{args.port}/", flush=True)
    print("（Windows 侧浏览器打开上面这个地址；Ctrl+C 结束）", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\n已停止", flush=True)


if __name__ == "__main__":
    main()
