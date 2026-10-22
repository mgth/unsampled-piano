#!/usr/bin/env python3
"""Scores model note sets against the reference sets on what the hammer
sets: attack partial profile (ff, mf), attack brightening pp→ff and the
dynamics, per register. Target = mean of the references (dynamics: the sets
played at the same MIDI velocities, Salamander and Commercial); tolerance =
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
        out[f"{lab}|dyn"] = [float(np.mean(d1)), float(np.mean(d2))] if d1 else None
    return out
if ARGS[0] == "--refs":
    refs = {a.split("=")[0]: metrics(a.split("=")[1]) for a in ARGS[2:]}
    json.dump(refs, open(ARGS[1], "w")); sys.exit()
refs = json.load(open(ARGS[0]))
def score(mm, verbose=False):
    errs = []
    for key, v in mm.items():
        if v is None: continue
        srcs = ["salamander", "commercial"] if key.endswith("|dyn") else list(refs)
        vals = [np.array(refs[s][key], dtype=float) for s in srcs if refs[s].get(key) is not None]
        if not vals: continue
        vals = np.array(vals); tgt = np.nanmean(vals, axis=0)
        tol = np.maximum(np.nanstd(vals, axis=0), 0.15 if key.endswith("|bright") else 3.0)
        e = ((np.array(v, dtype=float) - tgt) / tol) ** 2
        e = e[np.isfinite(e)]
        errs.extend(e.tolist())
        if verbose: print(f"  {key:28} model {np.round(v, 2)}  target {np.round(tgt, 2)}")
    return float(np.mean(errs))
for d in ARGS[1:]:
    mm = metrics(d)
    print(f"{score(mm):7.3f}  {d}")
    if len(ARGS) == 2: score(mm, True)
