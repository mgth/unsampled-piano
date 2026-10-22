#!/usr/bin/env python3
"""Scores model note sets against the reference sets on what the hammer
sets: attack partial profile (ff, mf), attack brightening pp→ff and the
dynamics, per register. Target = mean of the references (dynamics: the sets
played at the same MIDI velocities, Salamander and the commercial piano); tolerance =
their spread, at least 3 dB (brightening: 15 %).
  tools/hammer_score.py <refs.json> <model dir> [<model dir> …]
  tools/hammer_score.py --refs <out.json> salamander=<dir> iowa=<dir> commercial=<dir>"""
import json, os, sys
import numpy as np
sys.argv, ARGS = ["x"], sys.argv[1:]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "compare_sources.py")).read().split("sources = []")[0])
KEYS = list(range(21, 109, 3))
def metrics(d):
    out = {}
    cache = {}
    def get(m, l):
        if (m, l) not in cache:
            p = find(d, m, l); cache[(m, l)] = load(p) if p else None
        return cache[(m, l)]
    for lo, hi, lab in REGS:
        ks = [m for m in KEYS if lo <= m <= hi]
        for l, tag in ((13, "ff"), (8, "mf")):
            prof = [partials(get(m, l), m, 0.0, 0.06)[[1, 2, 3, 5]] for m in ks if get(m, l) is not None]
            out[f"{lab}|attack_{tag}"] = np.nanmean(np.array(prof), axis=0).tolist() if prof else None
        br, d1, d2 = [], [], []
        for m in ks:
            a, b, c = get(m, 13), get(m, 8), get(m, 4)
            if a is None or b is None or c is None: continue
            br.append(np.log(centroid(a, m, 0, 0.05) / centroid(c, m, 0, 0.05)))
            d1.append(peak_db(a) - peak_db(c)); d2.append(peak_db(a) - peak_db(b))
        out[f"{lab}|bright"] = float(np.mean(br)) if br else None
        # Presence of the attack: share of energy (dB) at 2–4 and 4–8 kHz in
        # the first 60 ms, ff and mf.
        for l, tag in ((13, "ff"), (8, "mf")):
            hf = []
            for m in ks:
                x = get(m, l)
                if x is None: continue
                s0 = onset(x); seg = x[s0:s0 + 2880]
                S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / 48000); tot = S.sum()
                hf.append([10 * np.log10(S[(f >= a) & (f < b)].sum() / tot + 1e-12) for a, b in ((2000, 4000), (4000, 8000))])
            out[f"{lab}|hf_{tag}"] = np.mean(hf, axis=0).tolist() if hf else None
        # Brightening by partial: gain ff − pp of partials ≥ 6 minus that of
        # partials 1–2 (0–100 ms, absolute levels).
        sl = []
        for m in (ks if hi < 72 else []):
            a, c = get(m, 13), get(m, 4)
            if a is None or c is None: continue
            def prof(x):
                s0 = onset(x); n = 4800; N = 4 * n
                S = np.abs(np.fft.rfft(x[s0:s0 + n] * np.hanning(n), N)); f = np.fft.rfftfreq(N, 1 / 48000)
                out = []
                for k in range(1, 13):
                    fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))
                    w = (f > fk * 0.95) & (f < fk * 1.07)
                    out.append(20 * np.log10(S[w].max() + 1e-12) if w.any() and fk < 18000 else np.nan)
                return np.array(out)
            g = prof(a) - prof(c)
            if np.isfinite(g[:2]).all() and np.isfinite(g[5:]).sum() >= 2:
                sl.append(np.nanmean(g[5:]) - np.mean(g[:2]))
        out[f"{lab}|slope"] = float(np.mean(sl)) if sl else None
        out[f"{lab}|dyn"] = [float(np.mean(d1)), float(np.mean(d2))] if d1 else None
    return out
if ARGS[0] == "--refs":
    refs = {a.split("=")[0]: metrics(a.split("=")[1]) for a in ARGS[2:]}
    json.dump(refs, open(ARGS[1], "w")); sys.exit()
refs = json.load(open(ARGS[0]))
def score(mm, verbose=False):
    errs = []
    for key, v in mm.items():
        # The undamped top strings still ring from the preceding notes of a
        # recorded sequence: the quiet C7–C8 peaks are not their own. Their
        # partials 3–6 (8–25 kHz) sit at the references' noise level.
        if v is None or key == "suraigu C7–C8|dyn" or key.startswith("suraigu C7–C8|attack"): continue
        srcs = ["salamander", "commercial"] if key.endswith("|dyn") else list(refs)
        vals = [np.array(refs[s][key], dtype=float) for s in srcs if refs[s].get(key) is not None]
        if not vals: continue
        vals = np.array(vals); tgt = np.nanmean(vals, axis=0)
        tol = np.maximum(np.nanstd(vals, axis=0), 0.15 if key.endswith("|bright") else 4.0 if ("|slope" in key or "|hf_" in key) else 3.0)
        e = ((np.array(v, dtype=float) - tgt) / tol) ** 2
        e = e[np.isfinite(e)]
        errs.extend(e.tolist())
        if verbose: print(f"  {key:28} model {np.round(v, 2)}  target {np.round(tgt, 2)}")
    return float(np.mean(errs))
for d in ARGS[1:]:
    mm = metrics(d)
    print(f"{score(mm):7.3f}  {d}")
    if len(ARGS) == 2: score(mm, True)
