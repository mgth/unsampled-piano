#!/usr/bin/env python3
"""Third-octave spectrum (25 Hz–2 kHz) of the bass keys A0–B2 (ff), summed
over the keys, in a window after the onset, in dB re the total energy
100 Hz–1 kHz: the low-frequency balance the board gives to the bass.
  tools/third_octaves.py t0,dur name=dir …"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
CEN = [25 * 2 ** (i / 3) for i in range(27)]
t0, dur = map(float, sys.argv[1].split(","))
lo, hi = (int(v) for v in os.environ.get("KEYS", "21,47").split(","))
print("Hz       " + "".join(f"{c:6.0f}" for c in CEN))
for a in sys.argv[2:]:
    nm, d = a.split("=", 1)
    acc = np.zeros(len(CEN))
    for m in range(21, 109, 3):
        p = next((q for q in (os.path.join(d, f"{name(m)}v13.{e}") for e in ("wav", "flac")) if os.path.exists(q)), None)
        if not (lo <= m <= hi) or p is None: continue
        x = load(p); s0 = int(np.argmax(np.abs(x) > 0.1 * np.abs(x).max()))
        seg = x[s0 + int(t0 * 48000): s0 + int((t0 + dur) * 48000)]
        S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / 48000)
        b = np.array([S[(f >= c * 2 ** (-1 / 6)) & (f < c * 2 ** (1 / 6))].sum() for c in CEN])
        acc += b / S[(f >= 100) & (f < 1000)].sum()
    print(f"{nm:8} " + "".join(f"{10 * np.log10(v + 1e-30):6.1f}" for v in acc))
