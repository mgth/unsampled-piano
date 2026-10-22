#!/usr/bin/env python3
"""Median log2(T60 model / T60 reference) of the main ESPRIT component of
each partial (layers ≥ 8), by register and partial frequency band, for one
or more calibration directories made by tools/calibrate.sh.
  tools/t60_bins.py <calib dir> [<calib dir> …]"""
import sys
import numpy as np, pandas as pd
def load(p):
    d = pd.read_csv(p); d = d[(d["rank"] == 0) & (d.layer >= 8)]
    return d.set_index(["midi", "layer", "k"])
REGS = [(21, 47), (48, 71), (72, 87), (88, 108)]
FB = [0, 300, 700, 1500, 3000, 20000]
for name in sys.argv[1:]:
    ref = load(f"{name}/ref/components.csv")
    m = load(f"{name}/model/components.csv")["t60"]
    j = pd.concat([ref["t60"].rename("r"), m.rename("m"), ref["partial_freq"].rename("f")], axis=1).dropna().reset_index()
    j["lr"] = np.log2(j.m / j.r)
    print("==", name)
    print("register " + " ".join(f"{a}-{b}Hz".rjust(14) for a, b in zip(FB[:-1], FB[1:])) + "   k=1")
    for lo, hi in REGS:
        s = j[(j.midi >= lo) & (j.midi <= hi)]
        cells = []
        for a, b in zip(FB[:-1], FB[1:]):
            t = s[(s.f >= a) & (s.f < b)]
            cells.append(f"{t.lr.median():+5.2f}({len(t):4d})" if len(t) > 5 else "")
        k1 = s[s.k == 1]
        print(f"{lo:3}-{hi:3} " + " ".join(c.rjust(14) for c in cells) + f"   {k1.lr.median():+5.2f}({len(k1)})")
