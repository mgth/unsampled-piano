#!/usr/bin/env python3
"""Percussiveness and spectral presence per register, ff and mf:
attack peak (5 ms RMS) minus the 150–300 ms level, and the attack (0–60 ms)
and sustain (0.3–0.8 s) energy per octave band relative to the whole band.
  tools/presence.py name=dir …"""
import os, sys
import numpy as np
ARGS = sys.argv[1:]; sys.argv = ["x"]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "compare_sources.py")).read().split("sources = []")[0])
BANDS = [(0, 250), (250, 1000), (1000, 2000), (2000, 4000), (4000, 8000), (8000, 20000)]
def stats(x):
    s = onset(x)
    e5 = np.array([np.mean(x[i:i + 240] ** 2) for i in range(s, s + 2400, 120)])
    peak = 10 * np.log10(e5.max() + 1e-20)
    sus = 10 * np.log10(np.mean(x[s + 7200: s + 14400] ** 2) + 1e-20)
    def bands(a, b):
        seg = x[s + int(a * 48000): s + int(b * 48000)]
        S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / 48000)
        tot = S.sum()
        return [10 * np.log10(S[(f >= lo) & (f < hi)].sum() / tot + 1e-12) for lo, hi in BANDS]
    return peak - sus, bands(0, 0.06), bands(0.3, 0.8)
srcs = [(a.split("=")[0], a.split("=")[1]) for a in ARGS]
for layer, tag in ((13, "ff"), (8, "mf")):
    print(f"\n### {tag}: percussiveness (5 ms peak − 150–300 ms level, dB)")
    print("| register | " + " | ".join(n for n, _ in srcs) + " |\n|---|" + "---|" * len(srcs))
    res = {}
    for lo, hi, lab in REGS:
        cells = []
        for n, d in srcs:
            st = [stats(load(find(d, m, layer))) for m in range(21, 109, 3) if lo <= m <= hi and find(d, m, layer)]
            res[(lab, n)] = st
            cells.append(f"{np.mean([s[0] for s in st]):5.1f}")
        print(f"| {lab} | " + " | ".join(cells) + " |")
    for which, title in ((1, "attack 0–60 ms"), (2, "sustain 0.3–0.8 s")):
        print(f"\n{tag}, {title}: energy share (dB) per band " + " ".join(f"{a}-{b}" for a, b in BANDS))
        for lo, hi, lab in REGS:
            line = " | ".join(f"{n}: " + " ".join(f"{v:5.1f}" for v in np.mean([s[which] for s in res[(lab, n)]], axis=0)) for n, _ in srcs)
            print(f"  {lab:14} {line}")
