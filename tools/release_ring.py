#!/usr/bin/env python3
"""What rings on after the key is released (sequences of
tools/note_sequence_midi.py, key held HOLD s): octave-band levels in the
window 0.3–1.3 s after the release, in dB re the same band just before it
(0.5 s window), and re the note's peak, per register and layer.
  tools/release_ring.py name=dir …   (HOLD env, default 4)"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "bass"), (48, 71, "middle"), (72, 88, "treble (dampers)"), (89, 108, "top (no dampers)")]
CEN = [250, 500, 1000, 2000, 4000, 8000]
HOLD = float(os.environ.get("HOLD", 4.0))
LAYER = int(os.environ.get("LAYER", 13))
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{LAYER}.{ext}")
        if os.path.exists(p): return p
def bands(seg):
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2 / len(seg); f = np.fft.rfftfreq(len(seg), 1 / 48000)
    return np.array([10 * np.log10(S[(f >= c / 1.414) & (f < c * 1.414)].sum() + 1e-30) for c in CEN + [24000]])
print("bands (Hz)              " + "".join(f"{c:>7}" for c in CEN) + "    all")
for a in sys.argv[1:]:
    nm, d = a.split("=", 1)
    for lo, hi, lab in REGS:
        rel, peak = [], []
        for m in (k for k in range(21, 109, 3) if lo <= k <= hi):
            p = find(d, m)
            if not p: continue
            x = load(p); s0 = int(np.argmax(np.abs(x) > 0.1 * np.abs(x).max())); r = s0 + int(HOLD * 48000)
            before = bands(x[r - 24000: r]); after = bands(x[r + 14400: r + 62400])
            if len(x) < r + 62400: continue
            pk = bands(x[s0: s0 + 4800]).max()
            rel.append(after - before); peak.append(after - pk)
        if not rel: continue
        rel, peak = np.mean(rel, 0), np.mean(peak, 0)
        bb = rel[-1]
        print(f"{nm:11} {lab:17} re before: " + "".join(f"{v:7.1f}" for v in rel[:-1]) + f" | re peak: {peak[-1]:6.1f}")
