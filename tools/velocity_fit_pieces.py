#!/usr/bin/env python3
"""Velocity law for recorded performances: the velocities a Disklavier
records under a pianist's fingers do not follow the scale it plays back
(tools/velocity_fit.py fits the latter, on notes the instrument played
itself). Within performances (tools/piece_bench.py results), the onset
level of partial 1 against the recorded velocity is binned per register,
relative to each performance's notes at velocity 50–70; the model plays
every third key at hammer speeds from 0.1 to 10 m/s, and the law
  v(vel) = vmin·(vmax/vmin)^(t^curve),  t = (vel − 1)/126
is searched so that the model's level follows those bins, with v(60) kept
at the default law's (the mf timbre unchanged).
  tools/velocity_fit_pieces.py out/pieces/<name>/result.json …"""
import json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

WIDE = (0.1, 10.0)
BINS = [(15, 29), (30, 44), (45, 59), (60, 74), (75, 89), (90, 104), (105, 119)]

def law(vmin, vmax, curve):
    return lambda vel: vmin * (vmax / vmin) ** (((np.asarray(vel, float) - 1) / 126) ** curve)

def targets(results):
    """{register: [(bin centre, level re v50–70 dB, n)]} from the recordings."""
    out = {}
    for path in results:
        r = json.load(open(path))
        for a in r["notes"]["recording"]["attack"]:
            if a.get("p1") is None: continue
            out.setdefault(a["reg"], {}).setdefault(a["piece"], []).append((a["v"], a["p1"]))
    res = {}
    for reg, pieces in out.items():
        bins = {i: [] for i in range(len(BINS))}
        for rows in pieces.values():
            ref = [p for v, p in rows if 50 <= v <= 70]
            if len(ref) < 3: continue
            r0 = np.median(ref)
            for v, p in rows:
                for i, (lo, hi) in enumerate(BINS):
                    if lo <= v <= hi: bins[i].append(p - r0)
        res[reg] = [((lo + hi) / 2, float(np.median(bins[i])), len(bins[i])) for i, (lo, hi) in enumerate(BINS) if len(bins[i]) >= 10]
    return res

def main():
    tg = targets(sys.argv[1:])
    out = os.path.join(b.ROOT, "out", "velocity", "fit_pieces"); os.makedirs(out, exist_ok=True)
    preset = os.path.join(out, "wide.toml")
    open(preset, "w").write(f"[velocity]\nvmin = {WIDE[0]}\nvmax = {WIDE[1]}\ncurve = 1.0\n")
    probe = list(range(1, 128, 6)) + [127]
    exe = os.path.join(b.ROOT, "target", "release", "unsampled-piano-render")
    def render(j):
        m, vel = j; p = os.path.join(out, f"{m}_{vel}.wav")
        if not os.path.exists(p):
            subprocess.run([exe, "note", "--note", str(m), "--velocity", str(vel), "--hold", "1", "--tail", "0.1",
                            "--preset", preset] + b.listening_args() + [p], check=True, capture_output=True)
        return j, b.peak_db(b.load(p))
    with ThreadPoolExecutor(8) as ex: lv = dict(ex.map(render, [(m, v) for m in b.KEYS for v in probe]))
    speeds = law(*WIDE, 1.0)(probe)
    regs = {reg: [m for m in b.KEYS if lo <= m <= hi] for lo, hi, reg in b.REGS}
    def model_curve(reg, f, vels):
        rows = []
        for m in regs[reg]:
            L = np.interp(np.log(f(list(vels) + [60])), np.log(speeds), [lv[(m, v)] for v in probe])
            rows.append(L[:-1] - L[-1])
        return np.mean(rows, 0)
    def err(f):
        e, w = 0.0, 0.0
        for reg in ("bass", "middle", "treble"):
            if reg not in tg: continue
            vels = [c for c, _, _ in tg[reg]]; real = np.array([l for _, l, _ in tg[reg]]); n = np.array([k for _, _, k in tg[reg]])
            g = model_curve(reg, f, vels) - real
            e += np.sum(np.sqrt(n) * g * g); w += np.sum(np.sqrt(n))
        return np.sqrt(e / w)
    default = b.VelocityDefault if hasattr(b, "VelocityDefault") else (0.26, 4.4, 0.75)
    v60 = law(*default)(60)
    best = None
    for vmin in np.geomspace(0.03, 0.6, 40):
        for curve in np.linspace(0.4, 1.6, 49):
            t60 = (59 / 126) ** curve
            vmax = vmin * np.exp(np.log(v60 / vmin) / t60)
            if vmax > 12: continue
            e = err(law(vmin, vmax, curve))
            if best is None or e < best[0]: best = (e, vmin, vmax, curve)
    e, vmin, vmax, curve = best
    print(f"default law {default}: rms {err(law(*default)):.2f} dB against the performances")
    print(f"fitted: vmin {vmin:.3f}, vmax {vmax:.3f}, curve {curve:.3f}: rms {e:.2f} dB (v(60) = {v60:.2f} m/s)")
    fit = law(vmin, vmax, curve)
    for reg in ("bass", "middle", "treble"):
        if reg not in tg: continue
        vels = [c for c, _, _ in tg[reg]]
        print(f"{reg:7} velocity    " + " ".join(f"{v:6.0f}" for v in vels))
        print(f"{'':7} recordings  " + " ".join(f"{l:+6.1f}" for _, l, _ in tg[reg]))
        print(f"{'':7} default     " + " ".join(f"{x:+6.1f}" for x in model_curve(reg, law(*default), vels)))
        print(f"{'':7} fitted      " + " ".join(f"{x:+6.1f}" for x in model_curve(reg, fit, vels)))

if __name__ == "__main__":
    main()
