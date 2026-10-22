#!/usr/bin/env python3
"""Brightness of attack and sustain: octave-band levels 1–8 kHz (dB re the
strongest band at the attack, ff, 0.4 s windows at 0.05 s and 0.6 s),
model minus the mean of the references, per register; and the RMS of
these gaps (the 8 kHz band of the treble left out: recording noise in
the references).  tools/band_score.py ref=dir … -- name=dir …"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "grave"), (48, 71, "médium"), (72, 95, "aigu")]
CEN = [1000, 2000, 4000, 8000]
TIMES = (0.05, 0.6)
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v13.{ext}")
        if os.path.exists(p): return p
def bands(x, t, cen, dur=0.4):
    s0 = int(np.argmax(np.abs(x) > 0.1 * np.abs(x).max()))
    seg = x[s0 + int(t * 48000): s0 + int((t + dur) * 48000)]
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / 48000)
    return np.array([10 * np.log10(S[(f >= c / 1.414) & (f < c * 1.414)].sum() + 1e-30) for c in cen])
ALL = [31.5, 63, 125, 250, 500] + CEN
def profile(d):
    out = {}
    for lo, hi, lab in REGS:
        r = []
        for m in range(lo, hi + 1, 3):
            p = find(d, m)
            if not p: continue
            x = load(p); ref = bands(x, 0.0, ALL, 0.1).max()
            r.append([bands(x, t, CEN) - ref for t in TIMES])
        out[lab] = np.mean(r, 0)
    return out
args = sys.argv[1:]; cut = args.index("--")
refs = [profile(a.split("=", 1)[1]) for a in args[:cut]]
target = {lab: np.mean([r[lab] for r in refs], 0) for _, _, lab in REGS}
print(f"{'':10}" + "".join(f"{lab:>34}" for _, _, lab in REGS) + "   rms")
print(f"{'':10}" + "".join("   att 1k  2k  4k  8k | ten 1k  2k  4k  8k"[:34].rjust(34) for _ in REGS))
for a in args[cut + 1:]:
    nm, d = a.split("=", 1); p = profile(d); gaps = []
    cells = ""
    for _, _, lab in REGS:
        g = p[lab] - target[lab]
        cells += "  " + " ".join(f"{v:+4.0f}" for v in g[0]) + " |" + " ".join(f"{v:+4.0f}" for v in g[1])
        gaps += list(g[:, :3].ravel()) if lab == "aigu" else list(g.ravel())
    print(f"{nm:10}{cells}  {np.sqrt(np.mean(np.square(gaps))):5.1f}")
