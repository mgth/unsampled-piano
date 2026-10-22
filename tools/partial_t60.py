#!/usr/bin/env python3
"""Effective decay of each partial between 0.3 and 3 s (ff): T60 from a
linear fit of its narrow-band level in 0.1 s windows (beats averaged out),
median by register and partial range.  tools/partial_t60.py name=dir …"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "bass"), (48, 71, "middle"), (72, 95, "treble")]
BANDS = [(1, 1), (2, 2), (3, 4), (5, 8), (9, 12)]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def f0(m): return 440 * 2 ** ((m - 69) / 12)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m, layer=13):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{layer}.{ext}")
        if os.path.exists(p): return p
def t60s(x, m, t0=0.3, t1=3.0, K=12):
    s0 = int(np.argmax(np.abs(x) > 0.1 * np.abs(x).max()))
    y = x[s0: s0 + int(3.2 * 48000)]
    if len(y) < int(3.1 * 48000): return [np.nan] * K
    S = np.abs(np.fft.rfft(y[48000:] * np.hanning(len(y) - 48000))); f = np.fft.rfftfreq(len(y) - 48000, 1 / 48000)
    X = np.fft.rfft(y); ff = np.fft.rfftfreq(len(y), 1 / 48000)
    out = []
    for k in range(1, K + 1):
        fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))
        w = (f > fk * 0.96) & (f < fk * 1.05)
        if not w.any() or fk > 15000: out.append(np.nan); continue
        fp = f[w][np.argmax(S[w])]; bw = max(2.5, 0.006 * fp)
        Z = np.where(np.abs(ff - fp) < bw, X, 0); z = np.fft.irfft(Z, len(y))
        h = 4800; n = len(z) // h
        e = 10 * np.log10((z[: n * h].reshape(n, h) ** 2).mean(1) + 1e-30)
        t = (np.arange(n) + 0.5) * 0.1; sel = (t >= t0) & (t <= t1)
        a = np.polyfit(t[sel], e[sel], 1)[0]
        out.append(min(-60 / a, 99) if a < 0 else 99)
    return out
srcs = [a.split("=", 1) for a in sys.argv[1:]]
res = {nm: {m: t60s(load(find(d, m)), m) for m in range(21, 97, 3) if find(d, m)} for nm, d in srcs}
hdr = " ".join(f"k{a}" + (f"–{b}" if b > a else "") for a, b in BANDS)
print(f"effective T60 0.3–3 s (s), median: {hdr}")
for lo, hi, lab in REGS:
    print(f"\n{lab}")
    for nm, _ in srcs:
        r = np.array([v for m, v in res[nm].items() if lo <= m <= hi])
        print(f"  {nm:>11}: " + " ".join(f"{np.nanmedian(r[:, a - 1: b]):5.1f}" for a, b in BANDS))
