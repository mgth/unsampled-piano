#!/usr/bin/env python3
"""The instrument's fixed lines: off-partial peaks (0.1–0.5 s, 0.8–6 kHz,
≥ 12 dB above their ±150 Hz median, within 45 dB of the strongest partial)
found at the same frequency (±0.5 %) on at least `--min` different notes
of a set: resonators the instrument rings on every note (undamped string
segments, frame, board) rather than the notes' own partials.
  lines2.py <set dir> [--layer 13] [--min 3]"""
import os, argparse, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

def offpeaks(path, m):
    x = b.load(path); s = b.onset(x) + int(0.1 * b.SR); n = int(0.4 * b.SR)
    seg = x[s:s + n] * np.hanning(n); N = 1 << 16
    L = 10 * np.log10(np.abs(np.fft.rfft(seg, N)) ** 2 + 1e-30); f = np.fft.rfftfreq(N, 1 / b.SR)
    f1 = b.f0(m); on = np.zeros_like(f, dtype=bool); top = -1e9
    # the note's inharmonicity from its first partials
    pk = {}
    for k in range(1, 7):
        g = k * f1; w = (f > g * 0.95) & (f < g * 1.06)
        if w.any(): pk[k] = f[w][np.argmax(L[w])]
    f1 = pk.get(1, f1); B = 0.0
    ks = np.array([k for k in pk if k > 1]); fk = np.array([pk[k] for k in ks])
    if len(ks) >= 2:
        y = (fk / (ks * f1)) ** 2 - 1; B = float(np.clip((y * ks ** 2).sum() / (ks ** 4).sum(), 0, 0.01))
    for k in range(1, 120):
        g = k * f1 * np.sqrt(1 + B * k * k)
        if g > 7000: break
        w = (f > g * 0.975) & (f < g * 1.025)
        if not w.any(): continue
        i = np.where(w)[0][np.argmax(L[w])]; top = max(top, L[i])
        on |= (f > f[i] * 0.985) & (f < f[i] * 1.015)
        on |= (f > g * 0.985) & (f < g * 1.015)
    band = (f >= 800) & (f <= 6000) & ~on
    out = []
    for i in np.where(band)[0][1:-1]:
        if L[i] > L[i - 1] and L[i] >= L[i + 1] and L[i] - top > -45:
            w = (f > f[i] - 150) & (f < f[i] + 150)
            if L[i] - np.median(L[w]) >= 12: out.append((f[i], L[i] - top))
    return out

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("set"); ap.add_argument("--layer", type=int, default=13); ap.add_argument("--min", type=int, default=3); ap.add_argument("--lo", type=int, default=84); ap.add_argument("--step", type=int, default=3)
    a = ap.parse_args()
    found = {}
    keys = list(range(a.lo, 109, a.step))
    n_notes = 0
    for m in keys:
        p = b.find(a.set, m, a.layer)
        if not p: continue
        n_notes += 1
        for fr, lv in offpeaks(p, m):
            key = None
            for g in found:
                if abs(g - fr) / fr < 0.005: key = g; break
            if key is None: found[fr] = [(m, lv)]
            else: found[key].append((m, lv))
    common = [(g, v) for g, v in found.items() if len({m for m, _ in v}) >= a.min]
    common.sort(key=lambda t: -len(t[1]))
    print(f"{a.set}: {n_notes} notes; lines shared by ≥ {a.min} notes: {len(common)}")
    for g, v in common[:12]:
        lv = np.median([l for _, l in v])
        print(f"   {g:7.0f} Hz on {len({m for m, _ in v}):2d} notes, median {lv:6.1f} dB re strongest partial, e.g. " + " ".join(b.name(m) for m, _ in v[:6]))

if __name__ == "__main__":
    main()
