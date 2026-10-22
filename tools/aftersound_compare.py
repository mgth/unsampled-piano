#!/usr/bin/env python3
"""Prompt and aftersound per register from ESPRIT components (piano-survey
--esprit): for each partial (ff layer, k ≤ 6), prompt = strongest component,
aftersound = strongest component decaying ≥ 1.5× slower. Medians of the
prompt T60, aftersound T60, aftersound level re prompt (dB), its distance in cents and the share
of partials that have one.  tools/aftersound_compare.py name=comp.csv …"""
import sys
import numpy as np, pandas as pd
REGS = [(21, 47, "grave"), (48, 71, "médium"), (72, 87, "aigu"), (88, 108, "suraigu")]
def per_partial(path, layer):
    d = pd.read_csv(path); d = d[(d.layer == layer) & (d.k <= 6)]
    rows = []
    for (m, k), g in d.groupby(["midi", "k"]):
        p = g.loc[g.amp_db_rel.idxmax()]
        a = g[g.t60 >= 1.5 * p.t60]
        a = a.loc[a.amp_db_rel.idxmax()] if len(a) else None
        rows.append(dict(midi=m, k=k, tp=p.t60, ta=a.t60 if a is not None else np.nan,
                         db=(a.amp_db_rel - p.amp_db_rel) if a is not None else np.nan,
                         ct=abs(a.cents_rel_main) if a is not None else np.nan))
    return pd.DataFrame(rows)
layer = 13
srcs = [a.split("=", 1) for a in sys.argv[1:]]
print("| registre | " + " | ".join(nm for nm, _ in srcs) + " |\n|---|" + "---|" * len(srcs))
tabs = {nm: per_partial(p, layer) for nm, p in srcs}
for lo, hi, lab in REGS:
    cells = []
    for nm, _ in srcs:
        s = tabs[nm][(tabs[nm].midi >= lo) & (tabs[nm].midi <= hi)]
        cells.append(f"{s.tp.median():4.1f} / {s.ta.median():5.1f} s, {s.db.median():+5.1f} dB, {s.ct.median():4.2f} ct ({100 * s.db.notna().mean():3.0f} %)")
    print(f"| {lab} | " + " | ".join(cells) + " |")
print("\nT60 prompt / T60 arrière-résonance, niveau de l'arrière-résonance re prompt, écart de fréquence (cents), part des partiels qui en ont une")
