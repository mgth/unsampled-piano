#!/usr/bin/env python3
"""Octave-band levels (dB re the note's strongest band at the attack) in
0.4 s windows after the onset, averaged over the keys of a register (ff):
which frequency range carries the sustain. TIMES (env, comma-separated
seconds) sets the windows, default 0.05,0.3,3.
  tools/band_decay.py lo hi name=dir …"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m, layer=13):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{layer}.{ext}")
        if os.path.exists(p): return p
CEN = [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000]
def bands(x, t, dur=0.4):
    s0 = int(np.argmax(np.abs(x) > 0.1 * np.abs(x).max()))
    seg = x[s0 + int(t * 48000): s0 + int((t + dur) * 48000)]
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / 48000)
    return np.array([10 * np.log10(S[(f >= c / 1.414) & (f < c * 1.414)].sum() + 1e-30) for c in CEN])
lo, hi = int(sys.argv[1]), int(sys.argv[2])
print("bande (Hz)      " + " ".join(f"{c:>6g}" for c in CEN))
for a in sys.argv[3:]:
    nm, d = a.split("=", 1)
    r = {float(t): [] for t in os.environ.get("TIMES", "0.05,0.3,3").split(",")}
    for m in range(21, 109, 3):
        if not (lo <= m <= hi) or not find(d, m): continue
        x = load(find(d, m)); ref = bands(x, 0.0, 0.1).max()
        for t in r: r[t].append(bands(x, t) - ref)
    for t, v in r.items():
        print(f"{nm:>10} {t:4.2f}s " + " ".join(f"{c:6.1f}" for c in np.mean(v, 0)))
