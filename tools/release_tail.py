#!/usr/bin/env python3
"""Tails after the release in a tools/release_test_midi.py rendering:
for each segment, octave-band levels in 0.3–1.3 s and 1.3–2.3 s after the
last key-up, in dB re the segment's loudest band while playing.
  tools/release_tail.py <segs.txt> name=wav …  (OFFSET env: seconds to add
  to the segment times, for recordings that start later)"""
import os, subprocess, sys
import numpy as np
CEN = [250, 500, 1000, 2000, 4000, 8000]
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)
def bands(seg):
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2 / len(seg); f = np.fft.rfftfreq(len(seg), 1 / 48000)
    return np.array([10 * np.log10(S[(f >= c / 1.414) & (f < c * 1.414)].sum() + 1e-30) for c in CEN])
segs = [(l.split()[0], float(l.split()[1]), float(l.split()[2])) for l in open(sys.argv[1])]
off = float(os.environ.get("OFFSET", 0.0))
srcs = [(a.split("=", 1)[0], load(a.split("=", 1)[1])) for a in sys.argv[2:]]
print(f"{'segment':16}{'source':12}  0.3–1.3 s after release: " + "".join(f"{c:>6}" for c in CEN) + "  |  1.3–2.3 s")
for name, t0, t1 in segs:
    for nm, x in srcs:
        a, r = int((t0 + off) * 48000), int((t1 + off) * 48000)
        ref = bands(x[a: r]).max()
        e1 = bands(x[r + 14400: r + 62400]) - ref; e2 = bands(x[r + 62400: r + 110400]) - ref
        print(f"{name:16}{nm:12}  " + " " * 25 + "".join(f"{v:6.0f}" for v in e1) + "  | " + "".join(f"{v:6.0f}" for v in e2))
