#!/usr/bin/env python3
"""Moves a preset made with the two-layer hammer (core_p 2.5 and core
depths) onto the graded felt (core depths 0, core_p 7: docs/felt.md),
keeping its voicing: the attack's brightness and level are measured on the
preset as it is, then the graded felt's hardness, core hardness and
hardness slopes, and the preset's velocity law, are fitted so that the new
preset matches them.

  tools/refelt_preset.py <preset.toml> [--write]

Probe: keys C1–C7 every six semitones at velocities 30, 60 and 100, close
microphones (tools/bench.py). Measures: the attack's spectral centroid
above 0.7·f0 (first 50 ms, log) and its peak level re the key at velocity
100. Without --write, prints the fitted preset; with it, rewrites the file
(keep a copy first)."""
import argparse, os, re, shutil, subprocess, sys, tempfile, tomllib
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from scipy.optimize import minimize
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

KEYS = list(range(24, 97, 6))
VELS = (30, 60, 100)
FELT = ("hardness", "hardness_slope", "hardness_slope_bass", "core_hardness", "core_p", "core_depth_bass", "core_depth_mid", "core_depth_treble")
EXE = os.path.join(b.ROOT, "target", "release", "unsampled-piano-render")

def measure(preset, work):
    def one(j):
        m, v = j; w = os.path.join(work, f"{m}_{v}.wav")
        subprocess.run([EXE, "--preset", preset] + b.listening_args() + ["note", "--note", str(m), "--velocity", str(v), "--hold", "0.4", "--tail", "0.1", w],
                       check=True, capture_output=True)
        x = b.load(w); os.remove(w)
        return j, (np.log(b.centroid(x, m, 0, 0.05)), b.peak_db(x))
    with ThreadPoolExecutor(8) as ex: r = dict(ex.map(one, [(m, v) for m in KEYS for v in VELS]))
    c = np.array([[r[(m, v)][0] for v in VELS] for m in KEYS])
    lv = np.array([[r[(m, v)][1] for v in VELS] for m in KEYS]); lv -= lv[:, -1:]
    return c, lv

def strip(text):
    """The preset without its felt keys and the frozen felt comment."""
    out, skip_comment = [], False
    for line in text.split("\n"):
        if line.startswith("# Frozen") and "felt and velocity" in line: skip_comment = True; continue
        if skip_comment and line.startswith("# ("): continue
        skip_comment = False
        if re.match(rf"^({'|'.join(FELT)})\s*=", line): continue
        out.append(line)
    return "\n".join(out)

def build(base, vel_sec, p):
    h, ch, hs, hsb, vmin, curve = p
    head = (f"# Graded felt (docs/felt.md), fitted by tools/refelt_preset.py on this preset's two-layer voicing.\n"
            f"hardness = {h:.4f}\nhardness_slope = {hs:.4f}\nhardness_slope_bass = {hsb:.4f}\ncore_hardness = {ch:.4f}\n")
    # Header comments stay first; the felt goes before the first key.
    lines = base.split("\n"); i = 0
    while i < len(lines) and (lines[i].startswith("#") or not lines[i].strip()): i += 1
    text = "\n".join(lines[:i] + [head.rstrip("\n")] + lines[i:])
    vel = f"[velocity]\nvmin = {vmin:.4f}\nvmax = {vel_sec.get('vmax', 4.2)}\ncurve = {curve:.4f}\n"
    text = re.sub(r"\[velocity\][^\[]*", "", text).rstrip("\n") + "\n\n" + vel
    return text

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("preset"); ap.add_argument("--write", action="store_true")
    a = ap.parse_args()
    text = open(a.preset).read(); t = tomllib.loads(text)
    if t.get("core_hardness", 1.0) == 0.0:
        print(f"{a.preset}: no core, nothing to move"); return
    work = tempfile.mkdtemp(dir=os.path.join(b.ROOT, "out"))
    c0, l0 = measure(a.preset, work)
    base = strip(text); vel = t.get("velocity", {})
    x0 = [t.get("hardness", 0.3), 2.0, t.get("hardness_slope", 0.02), t.get("hardness_slope_bass", 0.02), vel.get("vmin", 0.26), vel.get("curve", 0.75)]
    cand = os.path.join(work, "cand.toml"); best = [None, 1e9]
    def cost(q):
        p = [x0[0] * np.exp(q[0]), 2.0 * np.exp(q[1]), q[2], q[3], x0[4] * np.exp(q[4]), np.clip(q[5], 0.4, 1.6)]
        open(cand, "w").write(build(base, vel, p))
        c, lv = measure(cand, work)
        e = np.sqrt(np.mean(((c - c0) / 0.08) ** 2) + np.mean(((lv - l0) / 1.5) ** 2))
        if e < best[1]: best[0], best[1] = p, e
        print(f"  h {p[0]:.3f} core {p[1]:.2f} slope {p[2]:+.3f}/{p[3]:+.3f} vmin {p[4]:.3f} curve {p[5]:.2f}: cost {e:.2f}", flush=True)
        return e
    minimize(cost, [0.0, 0.0, x0[2], x0[3], 0.0, x0[5]], method="Nelder-Mead", options={"maxfev": 60, "xatol": 0.02, "fatol": 0.02,
             "initial_simplex": np.array([[0, 0, x0[2], x0[3], 0, x0[5]], [0.4, 0, x0[2], x0[3], 0, x0[5]], [0, 0.7, x0[2], x0[3], 0, x0[5]],
                                          [0, 0, x0[2] + 0.02, x0[3], 0, x0[5]], [0, 0, x0[2], x0[3] + 0.02, 0, x0[5]], [0, 0, x0[2], x0[3], 0.3, x0[5]],
                                          [0, 0, x0[2], x0[3], 0, x0[5] + 0.2]])})
    out = build(base, vel, best[0])
    c, lv = measure_text(out, work)
    print(f"{a.preset}: brightness gap {np.sqrt(np.mean((c - c0) ** 2)) * 100:.0f} % rms, level gap {np.sqrt(np.mean((lv - l0) ** 2)):.1f} dB rms")
    if a.write:
        open(a.preset, "w").write(out)
    else:
        print(out)
    shutil.rmtree(work, ignore_errors=True)

def measure_text(text, work):
    p = os.path.join(work, "final.toml"); open(p, "w").write(text); return measure(p, work)

if __name__ == "__main__":
    main()
