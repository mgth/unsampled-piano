#!/usr/bin/env python3
"""Level of the phantom partials (sum frequencies f_j + f_k, a few Hz below
each transverse partial n of an inharmonic string) relative to partial n,
median over n = 8..30, ff, bass keys; references first.
  tools/phantom_levels.py name=dir …"""
import os, sys
import numpy as np, pandas as pd
ARGS = sys.argv[1:]; sys.argv = ["x"]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "compare_sources.py")).read().split("sources = []")[0])
def b_law(name):
    # Each piano's own inharmonicity when surveyed, else Salamander's (the
    # model's law).
    p = f"out/compare/{name}.survey.csv"
    if not os.path.exists(p): p = "out/compare/salamander.survey.csv"
    return pd.read_csv(p).query("layer == 13").groupby("midi")["B"].first()
keys = [24, 27, 30, 33, 36, 39, 42, 45]
print("touche  " + " ".join(f"{a.split('=')[0]:>12}" for a in ARGS))
rows = []
for m in keys:
    row = []
    for a in ARGS:
        d = a.split("=")[1]; p = find(d, m, 13)
        if p is None: row.append(np.nan); continue
        x = load(p); s = onset(x) + 2400; n = 48000
        S = 20 * np.log10(np.abs(np.fft.rfft(x[s:s + n] * np.hanning(n), 4 * n)) + 1e-12); f = np.fft.rfftfreq(4 * n, 1 / 48000)
        b = float(b_law(a.split("=")[0]).get(m, 1e-4)); rel = []
        for k in range(8, 31):
            fk = k * f0(m) * np.sqrt(1 + b * k * k)
            w = (f > fk - 6) & (f < fk + 6)
            if not w.any() or fk > 15000: continue
            pk = S[w].max(); fpk = f[w][np.argmax(S[w])]
            ph = (f > fpk - 0.4 * f0(m)) & (f < fpk - 3)
            rel.append(S[ph].max() - pk)
        row.append(np.median(rel))
    rows.append(row)
    print(f"{name(m):6} " + " ".join(f"{v:12.1f}" for v in row))
print("médiane " + " ".join(f"{v:12.1f}" for v in np.nanmedian(np.array(rows), axis=0)))
