#!/usr/bin/env python3
"""Sweeps each musician's control (crate::macros) from −1 to +1 and reads
the effect it is meant to have, on a few notes heard with close
microphones (tools/bench.py), so that its span and its steps can be set.

  tools/macro_sweep.py [control …] [--preset P]

Per control, at −1, −0.5, 0, +0.5, +1 (difference from 0):
- tone: attack centroid above 0.7·f0, C3/C5 at velocity 60 (%);
- touch: level difference velocity 100 − 30, C4 (dB);
- sustain: level of C3 and C5 at 2 s re the peak (dB);
- resonance: level 0.3–0.8 s after a C4 played with the pedal down,
  away from its own partials (dB);
- release: drop of C4 0.25 s after the key-up (dB);
- unisons: level of C4 and C5 over 1–2 s re the peak (dB);
- mechanism: level of the first 30 ms below 0.75·f0 of C7 (dB);
- size: A0's inharmonicity (log10 B change) and C2's level (dB);
- lid: centroid of C4 at velocity 80 (%);
- space: level 0.2–0.6 s after the release of a staccato C4, re its peak (dB),
  heard with the engine's own set-up (its room);
- stretch: C8's tuning re equal temperament, read from the model (cents)."""
import argparse, os, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b, midifile

EXE = os.path.join(b.ROOT, "target", "release", "unsampled-piano-render")
VALUES = (-1.0, -0.5, 0.0, 0.5, 1.0)

def preset(base, ctl, v, work):
    p = os.path.join(work, f"{ctl}_{v:+.1f}.toml")
    text = open(base).read() if base else ""
    open(p, "w").write(text + f"\n[macros]\n{ctl} = {v}\n")
    return p

def note(p, m, vel, work, hold=2.5, listening=True, tag=""):
    w = os.path.join(work, f"n_{os.path.basename(p)}_{m}_{vel}{tag}.wav")
    cmd = [EXE, "--preset", p] + (b.listening_args() if listening else []) + ["note", "--note", str(m), "--velocity", str(vel), "--hold", str(hold), "--tail", "0.8", w]
    subprocess.run(cmd, check=True, capture_output=True)
    x = b.load(w); os.remove(w); return x

def midi(p, notes, pedal, work, tag):
    mid = os.path.join(work, f"{tag}_{os.path.basename(p)}.mid"); w = os.path.join(work, f"{tag}_{os.path.basename(p)}.wav")
    midifile.write(mid, notes, pedal)
    subprocess.run([EXE, "--preset", p] + b.listening_args() + ["midi", mid, w, "--tail", "1.5"], check=True, capture_output=True)
    x = b.load(w); os.remove(w); os.remove(mid); return x

def level(x, t0, t1):
    s = int(t0 * b.SR); e = int(t1 * b.SR); return 10 * np.log10(np.mean(x[s:e] ** 2) + 1e-20)

def env_db(x, t):
    pk = np.max(np.abs(x)) ** 2; s = int(t * b.SR); return 10 * np.log10(np.mean(x[s:s + 4800] ** 2) / pk + 1e-20)

def off_partials(x, m, t0, t1):
    seg = x[int(t0 * b.SR):int(t1 * b.SR)]; P = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / b.SR)
    keep = (f > 60) & (f < 6000)
    for k in range(1, 60):
        fk = b.f0(m) * k * np.sqrt(1 + 3e-4 * k * k); keep &= ~((f > fk * 0.98) & (f < fk * 1.02))
    return 10 * np.log10(P[keep].sum() + 1e-20)

def measure(ctl, p, work):
    if ctl == "tone":
        return [100 * (b.centroid(note(p, m, 60, work, 0.5), m, 0, 0.05)) for m in (48, 72)]
    if ctl == "touch":
        return [b.peak_db(note(p, 60, 100, work, 0.5)) - b.peak_db(note(p, 60, 30, work, 0.5))]
    if ctl == "sustain":
        return [env_db(note(p, m, 80, work, 3.0), 2.0) for m in (48, 72)]
    if ctl == "resonance":
        x = midi(p, [(60, 90, 0.5, 2.5)], [(0.0, 127)], work, "res")
        return [off_partials(x, 60, 0.8, 1.3) - level(x, 0.5, 0.6)]
    if ctl == "release":
        x = midi(p, [(60, 90, 0.5, 1.5)], [(0.0, 0)], work, "rel")
        return [level(x, 1.45, 1.5) - level(x, 1.75, 1.8)]
    if ctl == "unisons":
        out = []
        for m in (60, 72):
            x = note(p, m, 80, work, 2.5); out.append(level(x, b.onset(x) / b.SR + 1.0, b.onset(x) / b.SR + 2.0) - b.peak_db(x))
        return out
    if ctl == "mechanism":
        x = note(p, 96, 90, work, 0.5); s = b.onset(x); seg = x[s:s + 1440]
        P = np.abs(np.fft.rfft(seg * np.hanning(len(seg)), 8192)) ** 2; f = np.fft.rfftfreq(8192, 1 / b.SR)
        return [10 * np.log10(P[(f > 60) & (f < 0.75 * b.f0(96))].sum() + 1e-20)]
    if ctl == "size":
        return [b.peak_db(note(p, 36, 80, work, 0.5))]
    if ctl == "lid":
        return [100 * b.centroid(note(p, 60, 80, work, 0.5, listening=False), 60, 0, 0.05)]
    if ctl == "space":
        x = note(p, 60, 80, work, 0.15, listening=False)
        return [level(x, 0.35, 0.75) - b.peak_db(x)]
    if ctl == "stretch":
        return []
    raise SystemExit(f"unknown control {ctl}")

UNITS = {"tone": "% (×100 centroid/f0)", "touch": "dB", "sustain": "dB", "resonance": "dB", "release": "dB", "unisons": "dB",
         "mechanism": "dB", "size": "dB", "lid": "% (×100 centroid/f0)", "space": "dB", "stretch": ""}

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("controls", nargs="*"); ap.add_argument("--preset")
    a = ap.parse_args()
    ctls = a.controls or [c for c in UNITS if c != "stretch"]
    work = tempfile.mkdtemp(dir=os.path.join(b.ROOT, "out"))
    for ctl in ctls:
        with ThreadPoolExecutor(5) as ex:
            res = list(ex.map(lambda v: measure(ctl, preset(a.preset, ctl, v, work), work), VALUES))
        ref = res[2]
        rows = [" ".join(f"{(r[i] - ref[i]):+7.1f}" if UNITS[ctl].startswith("dB") else f"{100 * (r[i] / ref[i] - 1):+6.0f}%" for i in range(len(r))) for r in res]
        print(f"{ctl:10} " + " | ".join(f"{v:+.1f}: {row}" for v, row in zip(VALUES, rows)))
    for f in os.listdir(work): os.remove(os.path.join(work, f))
    os.rmdir(work)

if __name__ == "__main__":
    main()
