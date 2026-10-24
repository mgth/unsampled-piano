#!/usr/bin/env python3
"""Lays out BiVib's binaural samples (Disklavier grand DC3 M4 or upright
DU1A, Papetti et al., 10 recorded MIDI velocities per key) as a bench note
set: the samples recorded at velocities 45, 56 and 100 become the layers
4, 8 and 13, which the model plays at the same velocities.
  tools/bivib_set.py "<BiVib>/Grand/binaural samples/grand-open" <out dir>"""
import os, re, sys

LAYER = {45: 4, 56: 8, 100: 13}
src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
n = 0
for f in sorted(os.listdir(src)):
    m = re.fullmatch(r"([A-G]#?-?\d)_(\d+)_\d+\.wav", f)
    if not m or int(m.group(2)) not in LAYER: continue
    dst = os.path.join(out, f"{m.group(1)}v{LAYER[int(m.group(2))]}.wav")
    if os.path.lexists(dst): os.remove(dst)
    os.symlink(os.path.join(os.path.abspath(src), f), dst); n += 1
print(n, "files")
