#!/usr/bin/env python3
"""Grid search of felt exponent offset, hardness and hammer speed range
against the reference brightness/dynamics curves (piano-eval)."""
import itertools, subprocess, sys
from pathlib import Path
import numpy as np, pandas as pd

ref_csv = Path("out/salamander/eval_phase1.csv")
ref = pd.read_csv(ref_csv)
ref = ref[ref.source == "reference"]
grid_dir = Path("out/salamander/grid")


def curves(df):
    n = df.drop_duplicates(["midi", "layer"])
    c = n.pivot_table(index="midi", columns="layer", values="centroid_hz")
    c = np.log(c.div(c[8], axis=0)).median()
    pk = n.pivot_table(index="midi", columns="layer", values="peak_dbfs")
    pk = pk.sub(pk[16], axis=0).median()
    p = df[df.k <= 12].copy()
    p["rel"] = p.level_db - p.groupby(["midi", "layer"]).level_db.transform("max")
    return c, pk, p.set_index(["midi", "layer", "k"]).rel


rc, rp, rprof = curves(ref)
rows = []
for po, hd, vmax in itertools.product([-1.5, -1.0, -0.5, 0.0], [0.5, 1.0, 2.0], [4.0, 6.0, 8.0]):
    out = grid_dir / f"p{po}_h{hd}_v{vmax}.csv"
    if not out.exists():
        subprocess.run(["target/release/piano-eval", "data/salamander/Samples", "--model-only", "--out", str(out),
                        f"--p-offset={po}", f"--hardness={hd}", f"--vmax={vmax}"], check=True,
                       stderr=subprocess.DEVNULL)
    m = pd.read_csv(out)
    mc, mp, mprof = curves(m)
    prof = (mprof - rprof).abs().dropna().median()
    rows.append(dict(p_offset=po, hardness=hd, vmax=vmax,
                     centroid_rms=float(np.sqrt(((mc - rc) ** 2).mean())),
                     dyn_rms_db=float(np.sqrt(((mp - rp) ** 2).mean())),
                     profile_db=float(prof)))
r = pd.DataFrame(rows)
r["score"] = r.centroid_rms / r.centroid_rms.min() + r.profile_db / r.profile_db.min()
print(r.sort_values("score").head(12).to_string(index=False))
r.to_csv(grid_dir / "summary.csv", index=False)
