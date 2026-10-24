#!/usr/bin/env python3
"""Lays out MAPS's isolated notes (ISOL/NO, a Disklavier upright, Emiya et
al.) as a bench note set: <note>v8 from the M files and <note>v13 from the
F ones, pedal up (S0). The P files are left out: their velocities (2–38)
are well below the bench's pp (40). Each key's velocity is MAPS's own draw.
  tools/maps_set.py <MAPS ISOL/NO dir> <out dir>"""
import os, re, sys

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
n = 0
for f in sorted(os.listdir(src)):
    m = re.fullmatch(r"MAPS_ISOL_NO_([MF])_S0_M(\d+)_\w+\.wav", f)
    if not m: continue
    k = int(m.group(2)); layer = {"M": 8, "F": 13}[m.group(1)]
    dst = os.path.join(out, f"{NAMES[k % 12]}{k // 12 - 1}v{layer}.wav")
    if os.path.lexists(dst): os.remove(dst)
    os.symlink(os.path.join(os.path.abspath(src), f), dst); n += 1
print(n, "files")
