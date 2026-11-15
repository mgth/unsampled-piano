#!/usr/bin/env python3
"""The Steinway D's 17 ribs as the built board takes them (soundboard.structure.ribs),
from the dimensioned drawing of Boutillon, Ege & Paulello, "Comparison of the
vibroacoustical characteristics of different pianos" (Acoustics 2012, arXiv
1210.3948, figure 2): each rib from rim to rim on the Concert grand's outline, at
`angle` degrees from the spine (51.2, measured on the drawing; Plath 2019 gives 40
from the keyboard's line), the
17th placed in the tail's corner by its own length, the others by the drawing's
clear gaps between two ribs plus their half widths. Heights are the drawing's, the
board's 9 mm added. Prints each rib's length against the drawing's (stderr) and the
TOML value (stdout). docs/bass_gaps.md, "The board as it is drawn".

  tools/steinway_d_ribs.py [angle] [short of the rim, m] [preset.toml]

The outline is read from the preset (the Concert grand's by default): its
`soundboard.a`, `b` and `outline`. On the drawing's inner line, the preset's
since 2026-11-15, ribs 6 to 17 have their drawn lengths within 4 mm
(docs/outline.md)."""
import os, sys, tomllib, numpy as np
PRESET = sys.argv[3] if len(sys.argv) > 3 else os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "presets/instruments/Concert grand/concert_grand.toml")
with open(PRESET, "rb") as f: board = tomllib.load(f)["soundboard"]
a, b = float(board["a"]), float(board["b"])
out = np.array(board["outline"], dtype=float) * [a, b]
LEN = [241, 370, 488, 553, 655, 776, 936, 1123, 1130, 1106, 1080, 1054, 1029, 1002, 956, 802, 577]
SPACE = [80, 91, 86, 89, 90, 100, 102, 105, 108, 107, 119, 118, 117, 114, 106, 112]
WIDTH = [22, 21, 21.5, 23, 24, 25.5, 25.5, 25.5, 25.5, 25.5, 25.5, 25.5, 25.5, 25.5, 25.5, 25, 25]
HEIGHT = [17, 19, 19, 22, 23, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 27, 27]
ang = np.radians(float(sys.argv[1]) if len(sys.argv) > 1 else 51.2); short = float(sys.argv[2]) if len(sys.argv) > 2 else 0.005
d = np.array([np.cos(ang), -np.sin(ang)]); n = np.array([np.sin(ang), np.cos(ang)])
def chord(c):
    """The outline's crossings with the line P·n = c: its two ends."""
    pts = []
    for q0, q1 in zip(out, np.roll(out, -1, 0)):
        u0, u1 = q0 @ n - c, q1 @ n - c
        if (u0 < 0) != (u1 < 0): pts.append(q0 + (q1 - q0) * u0 / (u0 - u1))
    pts.sort(key=lambda p: p @ d); return pts[0], pts[-1]
# The 17th: the line whose chord is 577 mm long, in the tail's corner.
lo, hi = out[:, 0].max() * n[0] + b * n[1] - 1.0, max(q @ n for q in out) - 1e-4
for _ in range(60):
    c = 0.5 * (lo + hi); p, q = chord(c)
    if np.linalg.norm(q - p) > LEN[16] / 1000: lo = c
    else: hi = c
cs = [c]
for i in range(15, -1, -1): cs.append(cs[-1] - (SPACE[i] + 0.5 * (WIDTH[i] + WIDTH[i + 1])) / 1000)
cs = cs[::-1]; res = []
for i, c in enumerate(cs):
    p, q = chord(c); L = np.linalg.norm(q - p)
    print(f"# rib {i + 1:2}: {L * 1000:5.0f} mm, drawing {LEN[i]:5}  ({(L * 1000 - LEN[i]):+5.0f})  from ({p[0]:.3f}, {p[1]:.3f}) to ({q[0]:.3f}, {q[1]:.3f})", file=sys.stderr)
    p, q = p + d * short, q - d * short; h = round(0.009 + HEIGHT[i] / 1000, 4)
    res.append([round(float(p[0] / a), 4), round(float(p[1] / b), 4), round(float(q[0] / a), 4), round(float(q[1] / b), 4), h, h])
while len(res) < 20: res.append([0.0] * 6)
print(res)
