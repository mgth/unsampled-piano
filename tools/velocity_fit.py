#!/usr/bin/env python3
"""Fits the MIDI velocity → hammer speed law to BiVib's Disklavier grand.

The model plays every third key at hammer speeds from 0.1 to 10 m/s (a
wide velocity law), which gives its peak level against the speed; the law
  v(vel) = vmin·(vmax/vmin)^(t^curve),  t = (vel − 1)/126
is then searched so that the level re velocity 100 follows the
Disklavier's at its ten velocities, bass to treble, with v(100) kept where
it is (the felt and the brightness were set there).
  tools/velocity_fit.py "<BiVib>/Grand/binaural samples/grand-open" [--top-weight 0.5]
      [--liang <Liang>/isolated-note_no-pedal [--liang-weight 1]]   (also Liang's Disklavier)"""
import argparse, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

VELS = [12, 23, 34, 45, 56, 67, 78, 89, 100, 111]
WIDE = (0.1, 10.0)

def law(vmin, vmax, curve):
    return lambda vel: vmin * (vmax / vmin) ** (((np.asarray(vel, float) - 1) / 126) ** curve)

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("bivib"); ap.add_argument("--top-weight", type=float, default=0.5)
    ap.add_argument("--vmin", type=float, default=0.6); ap.add_argument("--vmax", type=float, default=4.2)
    ap.add_argument("--liang", help="Liang's isolated-note recording, without extension (…/isolated-note_no-pedal)")
    ap.add_argument("--liang-weight", type=float, default=1.0, help="weight of a Liang key against a BiVib key")
    a = ap.parse_args()
    out = os.path.join(b.ROOT, "out", "velocity", "fit"); os.makedirs(out, exist_ok=True)
    preset = os.path.join(out, "wide.toml")
    open(preset, "w").write(f"[velocity]\nvmin = {WIDE[0]}\nvmax = {WIDE[1]}\n")
    probe = list(range(1, 128, 6)) + [127]
    exe = os.path.join(b.ROOT, "target", "release", "unsampled-piano-render")
    def render(j):
        m, vel = j; p = os.path.join(out, f"{m}_{vel}.wav")
        if not os.path.exists(p):
            subprocess.run([exe, "note", "--note", str(m), "--velocity", str(vel), "--hold", "2", "--tail", "0.2",
                            "--preset", preset] + b.listening_args() + [p], check=True, capture_output=True)
        x = b.load(p); return j, b.peak_db(x)
    with ThreadPoolExecutor(8) as ex: lv = dict(ex.map(render, [(m, v) for m in b.KEYS for v in probe]))
    speeds = law(*WIDE, 1.0)(probe)
    model = {m: np.array([lv[(m, v)] for v in probe]) for m in b.KEYS}
    # The Disklavier, re velocity 100.
    files = {}
    for f in os.listdir(a.bivib):
        mm = re.fullmatch(r"([A-G]#?)(-?\d)_(\d+)_\d+\.wav", f)
        if mm: files[(b.NAMES.index(mm.group(1)) + 12 * (int(mm.group(2)) + 1), int(mm.group(3)))] = os.path.join(a.bivib, f)
    real = {}
    for m in b.KEYS:
        if all((m, v) in files for v in VELS):
            L = np.array([b.peak_db(b.load(files[(m, v)])) for v in VELS]); real[m] = L - L[VELS.index(100)]
    # Liang's Disklavier grand (isolated notes at 49, 80 and 96), re 96.
    liang = {}
    if a.liang:
        import midifile, piece_bench as pb
        notes, _ = midifile.read(a.liang + ".mid"); x = pb.load(a.liang + ".aif"); t0, dr = pb.align(x, notes, 1.9)
        lv = {}
        for m, v, on, off in notes:
            s0 = int((t0 + dr * on - 0.02) * b.SR); lv[(m, v)] = b.peak_db(x[s0:s0 + b.SR // 2])
        for m in {m for m, v in lv}:
            if all((m, v) in lv for v in (49, 80, 96)):
                near = min(b.KEYS, key=lambda k: abs(k - m))
                if abs(near - m) <= 1: liang[near] = np.array([lv[(m, 49)] - lv[(m, 96)], lv[(m, 80)] - lv[(m, 96)]])
    def liang_of(m, f):
        L = np.interp(np.log(f([49, 80, 96])), np.log(speeds), model[m]); return (L - L[2])[:2]
    def curve_of(m, f):
        L = np.interp(np.log(f(VELS)), np.log(speeds), model[m]); return L - L[VELS.index(100)]
    def weight(m): return a.top_weight if m >= 96 else 1.0
    def err(f):
        e = [weight(m) * np.mean((curve_of(m, f) - real[m]) ** 2) for m in real]
        w = sum(weight(m) for m in real)
        if liang:
            e += [a.liang_weight * weight(m) * np.mean((liang_of(m, f) - liang[m]) ** 2) for m in liang]
            w += a.liang_weight * sum(weight(m) for m in liang)
        return np.sqrt(np.sum(e) / w)
    v100 = law(a.vmin, a.vmax, 1.0)(100)
    best = None
    for vmin in np.geomspace(0.05, 0.8, 40):
        for curve in np.linspace(0.4, 1.6, 49):
            t100 = (99 / 126) ** curve
            vmax = vmin * np.exp(np.log(v100 / vmin) / t100)
            if vmax > 12: continue
            e = err(law(vmin, vmax, curve))
            if best is None or e < best[0]: best = (e, vmin, vmax, curve)
    cur = law(a.vmin, a.vmax, 1.0)
    e, vmin, vmax, curve = best
    print(f"current law {a.vmin}–{a.vmax} m/s: rms {err(cur):.2f} dB")
    print(f"fitted: vmin {vmin:.3f}, vmax {vmax:.3f}, curve {curve:.3f}: rms {e:.2f} dB (v(100) = {v100:.2f} m/s)")
    # The best exponential law (curve 1) with the same anchor, for comparison.
    ex1 = min((err(law(vm, vm * np.exp(np.log(v100 / vm) / (99 / 126)), 1.0)), vm) for vm in np.geomspace(0.05, 0.8, 80))
    print(f"best exponential (curve 1): vmin {ex1[1]:.3f}, vmax {ex1[1] * np.exp(np.log(v100 / ex1[1]) / (99 / 126)):.3f}: rms {ex1[0]:.2f} dB")
    fit = law(vmin, vmax, curve)
    print("speed (m/s) at", VELS, ":", " ".join(f"{s:.2f}" for s in fit(VELS)))
    for lo, hi, reg in b.REGS:
        ks = [m for m in real if lo <= m <= hi]
        r = np.mean([real[m] for m in ks], 0); c = np.mean([curve_of(m, cur) for m in ks], 0); n = np.mean([curve_of(m, fit) for m in ks], 0)
        print(f"{reg:7} Disklavier " + " ".join(f"{x:+6.1f}" for x in r))
        print(f"{'':7} current    " + " ".join(f"{x:+6.1f}" for x in c))
        print(f"{'':7} fitted     " + " ".join(f"{x:+6.1f}" for x in n))
        kl = [m for m in liang if lo <= m <= hi]
        if kl:
            print(f"{'':7} Liang 49/80 re 96: " + " ".join(f"{x:+5.1f}" for x in np.mean([liang[m] for m in kl], 0))
                  + "   current " + " ".join(f"{x:+5.1f}" for x in np.mean([liang_of(m, cur) for m in kl], 0))
                  + "   fitted " + " ".join(f"{x:+5.1f}" for x in np.mean([liang_of(m, fit) for m in kl], 0)))

if __name__ == "__main__":
    main()
