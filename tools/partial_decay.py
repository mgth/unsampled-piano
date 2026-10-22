#!/usr/bin/env python3
"""Per-partial decay by register: drop (dB) of partials 1–12 from the attack
(0.05–0.15 s) to later windows, and each partial's late level re the note's
peak. Sources laid out as tools/split_notes.py writes them (layer 13 = ff).
  tools/partial_decay.py name=dir …"""
import sys
import numpy as np
sys.path.insert(0, __file__.rsplit("/", 1)[0])
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "grave"), (48, 71, "médium"), (72, 95, "aigu")]
import os, subprocess
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def f0(m): return 440 * 2 ** ((m - 69) / 12)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m, layer):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{layer}.{ext}")
        if os.path.exists(p): return p
def onset(x):
    a = np.abs(x); return int(np.argmax(a > 0.1 * a.max()))
K = 12
TIMES = (0.1, 0.5, 1.0, 2.0, 3.0)
def amps(x, m, t, dur=0.2):
    s = onset(x) + int((t - dur / 2) * 48000); n = int(dur * 48000)
    seg = x[s: s + n]
    if len(seg) < n: return np.full(K, np.nan)
    S = np.abs(np.fft.rfft(seg * np.hanning(n))) * 2 / n * 2; f = np.fft.rfftfreq(n, 1 / 48000)
    out = []
    for k in range(1, K + 1):
        fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))
        w = (f > fk * 0.96) & (f < fk * 1.06)
        out.append(20 * np.log10(S[w].max() + 1e-12) if w.any() and fk < 18000 else np.nan)
    return np.array(out)
srcs = [a.split("=", 1) for a in sys.argv[1:]]
res = {}
for nm, d in srcs:
    for m in range(21, 109, 3):
        p = find(d, m, 13)
        if not p: continue
        x = load(p)
        A = np.array([amps(x, m, t) for t in TIMES])  # times × K
        pk = A[0][np.isfinite(A[0])].max()
        res[(nm, m)] = A - pk
for lo, hi, lab in REGS:
    print(f"\n## {lab} : niveau des partiels re partiel le plus fort à 0,1 s (dB)")
    for ti, t in enumerate(TIMES):
        print(f"  t={t:3.1f}s", end="")
        for nm, _ in srcs:
            v = np.nanmean([res[(nm, m)][ti] for m in range(lo, hi + 1) if (nm, m) in res], axis=0)
            print(f" | {nm:>10}: " + " ".join(f"{c:5.0f}" for c in v[:K]), end="")
        print()
