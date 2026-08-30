#!/usr/bin/env bash
# 构建/刷写前执行: source env.sh
# 1) ESP-IDF 环境（eim 安装；esp-idf-sys 以 native 模式依赖它）
if [ -f "$HOME/.espressif/v6.0.2/esp-idf/export.sh" ]; then
    . "$HOME/.espressif/v6.0.2/esp-idf/export.sh"
else
    echo "[env.sh] 未找到 ESP-IDF v6.0.2（eim 安装目录 ~/.espressif）" >&2
fi
# 2) espup 生成的环境（LIBCLANG_PATH 等，espup install 完成后才有）
if [ -f "$HOME/export-esp.sh" ]; then
    . "$HOME/export-esp.sh"
else
    echo "[env.sh] 未找到 ~/export-esp.sh，请先运行 espup install" >&2
fi
