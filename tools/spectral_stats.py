#!/usr/bin/env python3
"""Statistical spectral comparison (piano-eval CSVs): band-averaged tilt and
partial-to-partial fluctuation around a smooth trend. A modal soundboard
places its peaks elsewhere than another piano's, so per-partial errors
penalise it; tilt and fluctuation statistics do not.

usage: python3 -I tools/spectral_stats.py label=file.csv [label=file.csv ...]"""
import sys, warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore")
BANDS = [0, 100, 200, 400, 800, 1600, 3200, 6400, 20000]


def stats(df):
    df = df[df.k <= 16].copy()
    df["f"] = df.k * df.f0 * np.sqrt(1 + df.B * df.k ** 2)
    tilt_rows, flu = [], []
    for _, g in df.groupby(["midi", "layer"]):
        if len(g) < 5:
            continue
        lf, lv = np.log2(g.f.values), g.level_db.values
        lv = lv - np.median(lv)
        tilt_rows.append(pd.DataFrame({"f": g.f.values, "lv": lv}))
        # Fluctuation: residual std around a quadratic fit in log frequency.
        c = np.polyfit(lf, lv, 2)
        flu.append(np.std(lv - np.polyval(c, lf)))
    t = pd.concat(tilt_rows)
    tilt = t.groupby(pd.cut(t.f, BANDS), observed=True).lv.median()
    return tilt, float(np.median(flu))


rows = {}
for arg in sys.argv[1:]:
    label, path = arg.split("=", 1)
    d = pd.read_csv(path)
    for src in ("reference", "model"):
        s = d[d.source == src]
        if len(s) and (src == "model" or "reference" not in rows):
            rows[label if src == "model" else "reference"] = stats(s)
out = pd.DataFrame({k: v[0] for k, v in rows.items()}).T
out.columns = [f"{int(c.left)}-{int(c.right)}" for c in out.columns]
out["fluct_db"] = [v[1] for v in rows.values()]
ref = out.loc["reference"]
out["tilt_err_db"] = (out.drop(columns="fluct_db") - ref.drop("fluct_db")).abs().drop(columns="tilt_err_db", errors="ignore").mean(axis=1)
pd.set_option("display.width", 200)
print(out.round(1).to_string())
