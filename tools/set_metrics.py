#!/usr/bin/env python3
"""Compact description of a note set (tools/split_notes.py layout, layers
8 = mf and 13 = ff), per register: attack brightness (spectral centroid /
f0, 0–50 ms) at mf and ff, dynamics ff − mf (peak dB), ff sustain level at
1, 2 and 3 s re the peak (high-pass 0.7·f0), and, given an ESPRIT
components CSV, the prompt T60 and the unison spread (cents between the
two strongest components of partial 1).
  tools/set_metrics.py [--json] name=dir[:components.csv] …"""
import os, sys
import numpy as np, pandas as pd
import json
sys.argv, ARGS = ["x"], sys.argv[1:]
JSON = "--json" in ARGS
ARGS = [a for a in ARGS if a != "--json"]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "compare_sources.py")).read().split("sources = []")[0])
REG = [(21, 47, "bass"), (48, 71, "middle"), (72, 95, "treble")]
def metrics(d, comp):
    out = {}
    for lo, hi, lab in REG:
        c8, c13, dyn, env = [], [], [], []
        for m in range(lo, hi + 1):
            a, b = find(d, m, 13), find(d, m, 8)
            if not a: continue
            x = load(a); c13.append(centroid(x, m, 0, 0.05)); e = env_levels(x, m); env.append([e[3], e[4], e[5]])
            if b:
                y = load(b); c8.append(centroid(y, m, 0, 0.05)); dyn.append(peak_db(x) - peak_db(y))
        out[lab] = dict(c_mf=np.mean(c8) if c8 else np.nan, c_ff=np.mean(c13), dyn=np.mean(dyn) if dyn else np.nan,
                        env=np.nanmean(env, 0))
        if comp and os.path.exists(comp):
            k = pd.read_csv(comp); k = k[(k.layer == 13) & (k.midi >= lo) & (k.midi <= hi)]
            p = k[(k["rank"] == 0) & (k.k <= 2)].t60.median()
            k1 = k[(k.k == 1) & (k["rank"] <= 1)]
            spread = k1.groupby("midi").cents_rel_main.apply(lambda s: s.abs().max()).median()
            out[lab].update(t60=p, spread=spread)
    return out
if JSON:
    res = {}
    for a in ARGS:
        nm, rest = a.split("=", 1); d, _, comp = rest.partition(":")
        o = metrics(d, comp or None)
        res[nm] = {lab: {k: (list(map(float, v)) if k == "env" else float(v)) for k, v in r.items()} for lab, r in o.items()}
    print(json.dumps(res)); sys.exit()
print(f"{'':14}" + "".join(f"{lab:>42}" for _, _, lab in REG))
print(f"{'':14}" + "  cent mf/ff  ff−mf  1s/2s/3s   T60  spread" * 3)
for a in ARGS:
    nm, rest = a.split("=", 1); d, _, comp = rest.partition(":")
    o = metrics(d, comp or None); row = f"{nm:14}"
    for _, _, lab in REG:
        r = o[lab]
        row += f"  {r['c_mf']:4.1f}/{r['c_ff']:4.1f} {r['dyn']:5.1f} {r['env'][0]:4.0f}/{r['env'][1]:3.0f}/{r['env'][2]:3.0f} {r.get('t60', np.nan):5.1f} {r.get('spread', np.nan):5.2f}"
    print(row)
