#!/usr/bin/env python3
"""The strings' stage alone: the bridge acceleration of the model against
Plath's concert grand, partial by partial (docs/stage_swap.md). For each
partial between --lo and --hi Hz: its level early (0.05–0.2 s) and in the
sustain (0.3–1 s), model minus real (dB), and its decay rate over 0.1–1 s
on each side (dB/s). Levels within a signal are re its strongest partial
early, so that the two instruments' overall levels do not matter.

  tools/bridge_partials.py <plath dir> --keys 33,45 [--velocity 105] [--lo 300] [--hi 2200]
                           [--state PROD08] [--take 2] [--preset P]
"""
import argparse, glob, os, subprocess, sys, tempfile
import numpy as np
from scipy.signal import resample_poly
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

SR = 48000
EXE = os.path.join(b.ROOT, "target", "release", "examples", "bridge_signal")

def onset(x):
    a = np.abs(x); return int(np.argmax(a > 0.1 * a.max()))

def spectrum(x, t0, t1):
    seg = x[t0:t1]; n = len(seg); N = 1 << 17
    return np.abs(np.fft.rfft(seg * np.hanning(n), N)) ** 2, np.fft.rfftfreq(N, 1 / SR)

def partials(x, m, lo, hi):
    """k → (f, early level, sustain level, decay dB/s), levels re the strongest partial early."""
    s = onset(x)
    Pe, f = spectrum(x, s + int(0.05 * SR), s + int(0.2 * SR))
    Ps, _ = spectrum(x, s + int(0.3 * SR), s + int(1.0 * SR))
    # the partials' frequencies from the early spectrum: f1 and B from 1–6
    f1 = b.f0(m); pk = {}
    for k in range(1, 7):
        w = (f > k * f1 * 0.95) & (f < k * f1 * 1.06); pk[k] = f[w][np.argmax(Pe[w])]
    f1 = pk[1]; ks = np.array([2, 3, 4, 5, 6]); y = (np.array([pk[k] for k in ks]) / (ks * f1)) ** 2 - 1
    B = max(float((y * ks ** 2).sum() / (ks ** 4).sum()), 0)
    # decay from a frame-by-frame track
    W = 8192; hop = 480; n = (int(1.05 * SR) - W) // hop + 1
    idx = np.arange(W)[None, :] + hop * np.arange(n)[:, None] + s
    St = np.abs(np.fft.rfft(x[idx] * np.hanning(W)[None, :], axis=1)) ** 2; ft = np.fft.rfftfreq(W, 1 / SR); t = (np.arange(n) * hop + W / 2) / SR
    out = {}; top = -1e9
    for k in range(1, 120):
        g = k * f1 * np.sqrt(1 + B * k * k)
        if g > hi: break
        w = (f > g * 0.985) & (f < g * 1.015); c = np.where(w)[0][np.argmax(Pe[w])]
        g = f[c]
        le = 10 * np.log10(Pe[c - 3: c + 4].sum() + 1e-30); ls = 10 * np.log10(Ps[c - 3: c + 4].sum() + 1e-30)
        top = max(top, le)
        ct = int(np.argmin(np.abs(ft - g))); e = 10 * np.log10(St[:, max(ct - 1, 0): ct + 2].sum(1) + 1e-30)
        sel = (t > 0.1) & (t < 1.0); rate = np.polyfit(t[sel], e[sel], 1)[0]
        if g >= lo: out[k] = [g, le, ls, rate]
    return {k: (g, le - top, ls - top, r) for k, (g, le, ls, r) in out.items()}

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("plath"); ap.add_argument("--keys", default="33,45"); ap.add_argument("--velocity", type=int, default=105)
    ap.add_argument("--lo", type=float, default=300); ap.add_argument("--hi", type=float, default=2200)
    ap.add_argument("--state", default="PROD08"); ap.add_argument("--take", type=int, default=2); ap.add_argument("--preset")
    a = ap.parse_args()
    tmp = tempfile.mkdtemp()
    for m in [int(k) for k in a.keys.split(",")]:
        files = sorted(glob.glob(os.path.join(a.plath, f"D1_{a.state}_{m - 20:02d}_*M{a.take:04d}.csv")))
        d = np.loadtxt(files[0], delimiter=";", skiprows=1, usecols=(2,), dtype=np.float32).astype(float)
        ar = resample_poly(d - d.mean(), 24, 25)
        prefix = os.path.join(tmp, b.name(m))
        subprocess.run([EXE, a.preset or os.path.join(b.ROOT, "presets", "default.toml"), prefix, "--note", str(m), "--velocity", str(a.velocity),
                        "--hold", "1.5", "--tail", "0.1"], check=True, capture_output=True)
        am = np.diff(np.fromfile(prefix + "_bridge.f32", np.float32).astype(float), prepend=0.0) * SR
        R, M = partials(ar, m, a.lo, a.hi), partials(am, m, a.lo, a.hi)
        print(f"\n{b.name(m)} (velocity {a.velocity}), bridge acceleration per partial, model − real: early 0.05–0.2 s, sustain 0.3–1 s; decay 0.1–1 s")
        print("    k     Hz  real early | early diff  sustain diff | decay real  model (dB/s)")
        for k in sorted(set(R) & set(M)):
            print(f"  {k:3d} {R[k][0]:6.0f}  {R[k][1]:6.1f}    | {M[k][1] - R[k][1]:+6.1f}      {M[k][2] - R[k][2]:+6.1f}     | {R[k][3]:6.1f}  {M[k][3]:6.1f}")
        ks = [k for k in R if k in M and R[k][0] >= 1000]
        if ks:
            dr = np.median([R[k][3] for k in ks]); dm = np.median([M[k][3] for k in ks])
            print(f"  median decay of the partials above 1 kHz: real {dr:.1f}, model {dm:.1f} dB/s")
    for f in os.listdir(tmp): os.remove(os.path.join(tmp, f))
    os.rmdir(tmp)

if __name__ == "__main__":
    main()
