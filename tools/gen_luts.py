#!/usr/bin/env python3
"""生成 wordclock-core/src/anim.rs 用的两张 LUT（sin / gamma）。

为什么用 LUT 而不是 f32 三角函数：
- core 是 no_std，没有 powf/sinf；LUT 让宿主测试与固件运行**逐位一致**。
- 将来做 conformance（网页 JS ↔ 固件）时，这两张表就是唯一的数值基准。

用法：python3 tools/gen_luts.py > /tmp/luts.rs（再粘进 anim.rs，或直接看输出）
"""
import math

print("// 由 tools/gen_luts.py 生成，勿手改")
print("/// sin(2πi/256) * 1000，i∈[0,256)")
print("pub const SIN: [i16; 256] = [")
vals = [round(math.sin(2 * math.pi * i / 256) * 1000) for i in range(256)]
for i in range(0, 256, 16):
    print("    " + ", ".join(str(v) for v in vals[i:i + 16]) + ",")
print("];")
print()
print("/// (i/255)^2.2 * 255 —— 感知 alpha 映射到 8bit 线性光输出")
print("pub const GAMMA: [u8; 256] = [")
g = [round(((i / 255) ** 2.2) * 255) for i in range(256)]
for i in range(0, 256, 16):
    print("    " + ", ".join(str(v) for v in g[i:i + 16]) + ",")
print("];")
