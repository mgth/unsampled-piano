#!/usr/bin/env python3
"""Early decay of the partials of the top keys (ff): level of partials 1
and 2 at 50, 100, 200 and 400 ms after the onset, in dB re their level in
the first 20 ms (narrow-band envelopes), per key and averaged.
  tools/early_decay.py lo hi name=dir …"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
TIMES = (0.05, 0.1, 0.2, 0.4)
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v13.{ext}")
        if os.path.exists(p): return p
def drops(x, m, k):
    s0 = int(np.argmax(np.abs(x) > 0.1 * np.abs(x).max())); seg = x[s0: s0 + 48000]
    S = np.abs(np.fft.rfft(x[s0 + 4800: s0 + 28800] * np.hanning(24000))); f = np.fft.rfftfreq(24000, 1 / 48000)
    fk = k * 440 * 2 ** ((m - 69) / 12) * np.sqrt(1 + 1.5e-3 * k * k)
    w = (f > fk * 0.95) & (f < fk * 1.08)
    if not w.any(): return [np.nan] * len(TIMES)
    fp = f[w][np.argmax(S[w])]
    F = np.fft.rfft(seg); ff = np.fft.rfftfreq(len(seg), 1 / 48000); F[np.abs(ff - fp) > max(15, 0.01 * fp)] = 0
    y = np.fft.irfft(F, len(seg)) ** 2
    lvl = lambda a, b: 10 * np.log10(y[int(a * 48000): int(b * 48000)].mean() + 1e-30)
    ref = lvl(0.0, 0.02)
    return [lvl(t, t + 0.02) - ref for t in TIMES]
lo, hi = int(sys.argv[1]), int(sys.argv[2])
keys = [k for k in range(21, 109, 3) if lo <= k <= hi]
srcs = [a.split("=", 1) for a in sys.argv[3:]]
print(f"partial 1 / partial 2: dB at {', '.join(f'{int(t*1000)}' for t in TIMES)} ms re the first 20 ms")
mean = {nm: [] for nm, _ in srcs}
for m in keys:
    row = f"{name(m):5}"
    for nm, d in srcs:
        p = find(d, m)
        if not p: row += " " * 26; continue
        x = load(p); a, b = drops(x, m, 1), drops(x, m, 2); mean[nm].append(a + b)
        row += f" {nm[:4]} " + " ".join(f"{v:3.0f}" for v in a) + " /" + " ".join(f"{v:3.0f}" for v in b)
    print(row)
print("mean")
for nm, v in mean.items():
    v = np.nanmean(v, 0); print(f"  {nm:11} " + " ".join(f"{x:4.0f}" for x in v[:4]) + "  /" + " ".join(f"{x:4.0f}" for x in v[4:]))
