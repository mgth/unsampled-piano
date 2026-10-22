#!/usr/bin/env python3
"""Decay envelope of the model against Salamander, key by key (every third
key, layer 13 ↔ velocity 100, key held 4 s), what the ear calls "sustain".
Levels relative to the peak, after a high-pass at 0.7·f0 (removes the
recordings' low-frequency floor, which dominates the treble tails).
  tools/envelope_compare.py <samples dir> <work dir> [preset.toml]
Prints model/reference per key and the mean model − reference per register."""
import os, subprocess, sys
import numpy as np
ref_dir, work = sys.argv[1], sys.argv[2]
preset = sys.argv[3] if len(sys.argv) > 3 else None
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
TIMES = [0.1, 0.3, 0.6, 1.0, 2.0, 3.0]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def levels(x, f0):
    X = np.fft.rfft(x); f = np.fft.rfftfreq(len(x), 1 / 48000); X[f < 0.7 * f0] = 0
    x = np.fft.irfft(X, len(x))
    h = 960; n = len(x) // h
    e = 10 * np.log10((x[: n * h].reshape(n, h) ** 2).mean(1) + 1e-20)
    k = int(np.argmax(e))
    return [e[min(k + int(t / 0.02), n - 1)] - e[k] for t in TIMES]
os.makedirs(work, exist_ok=True)
rows = []
print("key   " + "  ".join(f"{t:>4}s ref/model" for t in TIMES))
for m in range(21, 109, 3):
    ref = os.path.join(ref_dir, f"{name(m)}v13.flac")
    if not os.path.exists(ref):
        continue
    mod = os.path.join(work, f"m{m}.wav")
    if not os.path.exists(mod):
        cmd = ["cargo", "run", "-q", "--release", "-p", "piano-render", "--"]
        cmd += ["--preset", preset] if preset else []
        cmd += ["--tail", "4", "note", mod, "--note", str(m), "--velocity", "100"]
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)
    f0 = 440 * 2 ** ((m - 69) / 12)
    r, d = levels(load(ref), f0), levels(load(mod), f0)
    rows.append((m, r, d))
    print(f"{name(m):4}", " ".join(f"{a:6.1f}/{b:6.1f}" for a, b in zip(r, d)))
ms = np.array([x[0] for x in rows]); r = np.array([x[1] for x in rows]); d = np.array([x[2] for x in rows])
for lo, hi in [(21, 47), (48, 71), (72, 95), (96, 108)]:
    s = (ms >= lo) & (ms <= hi)
    print(f"{name(lo)}-{name(hi)} mean model-ref:", " ".join(f"{t}s {v:+5.1f}" for t, v in zip(TIMES, (d[s] - r[s]).mean(0))))
