#!/usr/bin/env python3
"""Per-partial growth with velocity: level of partials 1–8 (0.03–0.2 s after
the onset) at BiVib's ten velocities, slope in dB per 10 velocity steps over
23–100, per register (median over the keys); and the slope relative to
partial 1's (how much faster the upper partials grow).

  velslope.py <bivib grand-open dir> <model dir out/velocity/<name>>"""
import os, re, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

VELS = [23, 34, 45, 56, 67, 78, 89, 100]
REG = [("bass", 21, 47), ("middle", 48, 71), ("treble", 72, 95), ("top", 96, 108)]
KS = list(range(1, 9))

def levels(x, m):
    s = b.onset(x) + int(0.03 * b.SR); n = int(0.17 * b.SR); seg = x[s:s + n] * np.hanning(n)
    N = 1 << 15; S = np.abs(np.fft.rfft(seg, N)) ** 2; f = np.fft.rfftfreq(N, 1 / b.SR)
    out = []
    for k in KS:
        g = k * b.f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))
        res = 2.5 * b.SR / n; w = (f > g - max(0.05 * g, res)) & (f < g + max(0.06 * g, res))
        if not w.any() or g > 12000: out.append(np.nan); continue
        pk = S[w].max(); med = np.median(S[w])
        out.append(10 * np.log10(pk + 1e-30) if pk > 3 * med else np.nan)
    return np.array(out)

def files_bivib(d):
    out = {}
    for f in os.listdir(d):
        mm = re.fullmatch(r"([A-G]#?)(-?\d)_(\d+)_\d+\.wav", f)
        if mm: out[(b.NAMES.index(mm.group(1)) + 12 * (int(mm.group(2)) + 1), int(mm.group(3)))] = os.path.join(d, f)
    return out

def files_model(d):
    out = {}
    for f in os.listdir(d):
        mm = re.fullmatch(r"([A-G]#?)(-?\d)_(\d+)\.wav", f)
        if mm: out[(b.NAMES.index(mm.group(1)) + 12 * (int(mm.group(2)) + 1), int(mm.group(3)))] = os.path.join(d, f)
    return out

def slopes(files):
    res = {}
    for m in b.KEYS:
        L = []
        for v in VELS:
            p = files.get((m, v))
            if not p: break
            L.append(levels(b.load(p), m))
        if len(L) < len(VELS): continue
        L = np.array(L); sl = []
        for j in range(len(KS)):
            y = L[:, j]; ok = np.isfinite(y)
            sl.append(np.polyfit(np.array(VELS)[ok], y[ok], 1)[0] * 10 if ok.sum() >= 5 else np.nan)
        res[m] = np.array(sl)
    return res

def main():
    for label, d, ff in (("BiVib Disklavier", sys.argv[1], files_bivib), ("model", sys.argv[2], files_model)):
        res = slopes(ff(d))
        print(f"\n== {label} ({len(res)} keys): slope dB per 10 velocity steps, partials 1–8; then re partial 1")
        print("register " + " ".join(f"{'p' + str(k):>6}" for k in KS) + "  |" + " ".join(f"{'p' + str(k):>6}" for k in KS[1:]))
        for reg, lo, hi in REG:
            rows = np.array([res[m] for m in res if lo <= m <= hi])
            if len(rows) == 0: continue
            med = np.nanmedian(rows, 0); rel = med[1:] - med[0]
            print(f"{reg:8} " + " ".join(f"{v:6.2f}" for v in med) + "  |" + " ".join(f"{v:6.2f}" for v in rel))

if __name__ == "__main__":
    main()
