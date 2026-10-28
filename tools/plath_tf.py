#!/usr/bin/env python3
"""Plath's concert grand, anechoic: ear pressure per bridge acceleration,
sampled at every partial of every key (0.1–1.0 s after the onset).

  plath_tf.py <plath dir> <out.json> [--state PROD08] [--take 2]

Writes a list of (key, partial, f Hz, 10·log10(mean(pL², pR²)/a²) dB re
(Pa/(m/s²))², level of the partial re the note's strongest partial) and
prints the median per third octave."""
import argparse, glob, json, os, sys
import numpy as np

SR = 50000.0
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]

def analyse(path, key):
    d = np.loadtxt(path, delimiter=";", skiprows=1, usecols=(2, 3, 4), dtype=np.float32)
    a, pl, pr = d[:, 0].astype(float), d[:, 1].astype(float), d[:, 2].astype(float)
    a -= a.mean(); pl -= pl.mean(); pr -= pr.mean()
    m = 20 + key; f1 = 440 * 2 ** ((m - 69) / 12)
    env = np.abs(pl) + np.abs(pr)
    s = int(np.argmax(env > 0.1 * env.max()))
    s0, s1 = s + int(0.1 * SR), s + int(1.0 * SR)
    W = 16384; hop = W // 2
    def spec(x):
        seg = x[s0:s1]; n = (len(seg) - W) // hop + 1
        idx = np.arange(W)[None, :] + hop * np.arange(n)[:, None]
        return (np.abs(np.fft.rfft(seg[idx] * np.hanning(W)[None, :], axis=1)) ** 2).mean(0)
    A, PL, PR = spec(a), spec(pl), spec(pr)
    f = np.fft.rfftfreq(W, 1 / SR); df = f[1] - f[0]
    # noise of the acceleration channel before the onset
    pre = a[int(0.02 * SR):max(s - int(0.02 * SR), int(0.1 * SR))]
    if len(pre) >= W:
        n = (len(pre) - W) // hop + 1; idx = np.arange(W)[None, :] + hop * np.arange(n)[:, None]
        AN = (np.abs(np.fft.rfft(pre[idx] * np.hanning(W)[None, :], axis=1)) ** 2).mean(0)
    else:
        AN = np.zeros_like(A)
    rows = []; B = 0.0
    # inharmonicity from the first partials
    peaks = {}
    for k in range(1, 7):
        g = k * f1; w = (f > g * 0.96) & (f < g * 1.05)
        if not w.any(): break
        i = np.argmax(PL[w] + PR[w]); peaks[k] = f[w][i]
    if 1 in peaks and len(peaks) >= 3:
        f1 = peaks[1]; ks = np.array([k for k in peaks if k > 1]); fk = np.array([peaks[k] for k in ks])
        y = (fk / (ks * f1)) ** 2 - 1; B = max(float((y * ks ** 2).sum() / (ks ** 4).sum()), 0.0)
    best = -1e9
    for k in range(1, 60):
        g = k * f1 * np.sqrt(1 + B * k * k)
        if g > 8000: break
        w = (f > g * 0.985 - 2 * df) & (f < g * 1.015 + 2 * df)
        if not w.any(): continue
        P = PL[w] + PR[w]; i = np.argmax(P); c = np.where(w)[0][i]
        if 10 * np.log10(P[i] / (np.median(P) + 1e-30)) < 10: continue
        sl = slice(max(c - 2, 0), c + 3)
        ea = A[sl].sum(); en = AN[sl].sum(); ep = 0.5 * (PL[sl].sum() + PR[sl].sum())
        if ea < 10 * en or ea <= 0: continue
        lev = 10 * np.log10(ep + 1e-30); best = max(best, lev)
        rows.append([key, k, float(f[c]), 10 * np.log10(ep / ea), lev])
    for r in rows: r[4] = r[4] - best
    return rows

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data"); ap.add_argument("out"); ap.add_argument("--state", default="PROD08"); ap.add_argument("--take", type=int, default=2)
    a = ap.parse_args()
    rows = []
    for key in range(1, 89):
        files = sorted(glob.glob(os.path.join(a.data, f"D1_{a.state}_{key:02d}_*M{a.take:04d}.csv")))
        if not files: continue
        try: rows += analyse(files[0], key)
        except Exception as ex: print(f"key {key}: {ex}", file=sys.stderr)
        print(f"key {key}: {len(rows)} points so far", file=sys.stderr)
    json.dump(rows, open(a.out, "w"))
    r = np.array(rows)
    print("third octave  n   median dB   p25   p75")
    c = 31.25
    while c < 9000:
        sel = (r[:, 2] >= c / 2 ** (1 / 6)) & (r[:, 2] < c * 2 ** (1 / 6))
        if sel.sum() >= 3:
            v = r[sel, 3]
            print(f"{c:8.0f} {sel.sum():5d} {np.median(v):8.1f} {np.percentile(v, 25):6.1f} {np.percentile(v, 75):6.1f}")
        c *= 2 ** (1 / 3)

if __name__ == "__main__":
    main()
