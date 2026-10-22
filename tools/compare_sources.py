#!/usr/bin/env python3
"""Compares note sets laid out as <note>v<layer>.{wav,flac} (layers 4/8/13
≈ pp/mf/ff), every third key from A0: decay envelope, partial profile at the
attack and in the sustain, attack brightness, dynamics, and (from
piano-survey CSVs when given) inharmonicity and partial decays.
  tools/compare_sources.py name=dir[:survey.csv] …   (first = reference set)"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "bass A0–B2"), (48, 71, "middle C3–B4"), (72, 95, "treble C5–B6"), (96, 108, "top treble C7–C8")]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def f0(m): return 440 * 2 ** ((m - 69) / 12)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def find(d, m, layer):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{layer}.{ext}")
        if os.path.exists(p): return p
def bandpass_env(x, lo):
    X = np.fft.rfft(x); f = np.fft.rfftfreq(len(x), 1 / 48000); X[f < lo] = 0
    return np.fft.irfft(X, len(x))
def env_levels(x, m, times=(0.1, 0.3, 0.6, 1.0, 2.0, 3.0)):
    y = bandpass_env(x, 0.7 * f0(m)); h = 960; n = len(y) // h
    e = 10 * np.log10((y[: n * h].reshape(n, h) ** 2).mean(1) + 1e-20); k = int(np.argmax(e))
    return [e[min(k + int(t / 0.02), n - 1)] - e[k] if k + int(t / 0.02) < n else np.nan for t in times]
def onset(x):
    a = np.abs(x); return int(np.argmax(a > 0.1 * a.max()))
def partials(x, m, t0, dur, K=16):
    """Partial amplitudes (dB re partial 1) in a window starting t0 s after onset."""
    s = onset(x) + int(t0 * 48000); n = int(dur * 48000)
    seg = x[s: s + n]
    if len(seg) < n: return np.full(K, np.nan)
    S = np.abs(np.fft.rfft(seg * np.hanning(n))); f = np.fft.rfftfreq(n, 1 / 48000)
    out = []
    for k in range(1, K + 1):
        fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))  # loose inharmonic search window
        w = (f > fk * 0.94) & (f < fk * 1.08)
        out.append(20 * np.log10(S[w].max() + 1e-12) if w.any() and fk < 20000 else np.nan)
    out = np.array(out); return out - out[0]
def centroid(x, m, t0, dur):
    s = onset(x) + int(t0 * 48000); n = int(dur * 48000); seg = x[s: s + n]
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / 48000)
    sel = f > 0.7 * f0(m)
    return (S[sel] * f[sel]).sum() / S[sel].sum() / f0(m)
def peak_db(x):
    h = 480; n = len(x) // h; return 10 * np.log10((x[: n * h].reshape(n, h) ** 2).mean(1).max())

sources = []
for a in sys.argv[1:]:
    nm, rest = a.split("=", 1); d, _, sv = rest.partition(":")
    sources.append((nm, d, sv or None))
keys = list(range(21, 109, 3))
data = {}
for nm, d, sv in sources:
    r = {}
    for m in keys:
        for layer in (4, 8, 13):
            p = find(d, m, layer)
            if p: r[(m, layer)] = load(p)
    data[nm] = r

def table(title, fn, cols, layer=13, fmt="{:+6.1f}"):
    print(f"\n### {title}\n")
    print("| register | " + " | ".join(f"{nm}" for nm, _, _ in sources) + " |")
    print("|---|" + "---|" * len(sources))
    for lo, hi, lab in REGS:
        cells = []
        for nm, _, _ in sources:
            vals = [fn(x, m) for (m, l), x in data[nm].items() if l == layer and lo <= m <= hi]
            vals = [v for v in vals if v is not None]
            if not vals: cells.append("—"); continue
            v = np.nanmean(np.array(vals, dtype=float), axis=0)
            cells.append(" ".join(fmt.format(c) for c in np.atleast_1d(v)))
        print(f"| {lab} | " + " | ".join(cells) + " |")
    print(f"\n{cols}")

table("Sustain ff: level (dB) re peak at 0.3 / 1 / 2 / 3 s (high-pass 0.7·f0)", lambda x, m: [env_levels(x, m)[i] for i in (1, 3, 4, 5)], "columns: 0.3 s, 1 s, 2 s, 3 s")
table("Sustain mf: same", lambda x, m: [env_levels(x, m)[i] for i in (1, 3, 4, 5)], "columns: 0.3 s, 1 s, 2 s, 3 s", layer=8)
table("Attack timbre ff (0–60 ms): partials 2, 3, 4, 6, 8, 12 in dB re partial 1", lambda x, m: partials(x, m, 0.0, 0.06)[[1, 2, 3, 5, 7, 11]], "")
table("Sustain timbre ff (0.5–1 s): partials 2, 3, 4, 6, 8, 12 in dB re partial 1", lambda x, m: partials(x, m, 0.5, 0.5)[[1, 2, 3, 5, 7, 11]], "")
table("Attack timbre mf (0–60 ms)", lambda x, m: partials(x, m, 0.0, 0.06)[[1, 2, 3, 5, 7, 11]], "", layer=8)
table("Brightness: centroid / f0, attack 0–50 ms | 0.5–1 s (ff)", lambda x, m: [centroid(x, m, 0.0, 0.05), centroid(x, m, 0.5, 0.5)], "", fmt="{:6.1f}")

print("\n### Dynamics: peak ff − pp and ff − mf (dB); attack brightness ff / pp\n")
print("| register | " + " | ".join(nm for nm, _, _ in sources) + " |")
print("|---|" + "---|" * len(sources))
for lo, hi, lab in REGS:
    cells = []
    for nm, _, _ in sources:
        r = data[nm]; a, b, c = [], [], []
        for m in keys:
            if lo <= m <= hi and (m, 13) in r and (m, 4) in r and (m, 8) in r:
                a.append(peak_db(r[(m, 13)]) - peak_db(r[(m, 4)])); b.append(peak_db(r[(m, 13)]) - peak_db(r[(m, 8)]))
                c.append(centroid(r[(m, 13)], m, 0, 0.05) / centroid(r[(m, 4)], m, 0, 0.05))
        cells.append(f"{np.mean(a):+5.1f} {np.mean(b):+5.1f} ×{np.mean(c):.2f}" if a else "—")
    print(f"| {lab} | " + " | ".join(cells) + " |")

# Survey statistics (inharmonicity, decays) when available.
sv = [(nm, p) for nm, _, p in sources if p and os.path.exists(p)]
if sv:
    import pandas as pd
    print("\n### Inharmonicity B (median) and T60 of partials 1–4 (early decay, s)\n")
    print("| register | " + " | ".join(nm for nm, _ in sv) + " |")
    print("|---|" + "---|" * len(sv))
    frames = {nm: pd.read_csv(p) for nm, p in sv}
    for lo, hi, lab in REGS:
        cells = []
        for nm, df in frames.items():
            s = df[(df.midi >= lo) & (df.midi <= hi) & (df.layer == 13)]
            notes = s.groupby("midi")
            b = notes["B"].first().median()
            early = s[(s.k >= 1) & (s.k <= 4)].groupby("midi")["t60_early"].median()
            tt = early.median()
            cells.append(f"B {b:.1e} T60 {tt:.2f}")
        print(f"| {lab} | " + " | ".join(cells) + " |")
