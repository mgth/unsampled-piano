#!/usr/bin/env python3
"""Calibrate the felt hardness along the keyboard (voicing) on the per-note
spectral slope of the reference, keeping brightness-vs-velocity and dynamics."""
import itertools, subprocess, warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore")
ref_all = pd.read_csv("out/salamander/eval_phase4.csv")
ref = ref_all[ref_all.source == "reference"]


def slopes(d):
    d = d[(d.k <= 12) & (d.layer >= 6) & (d.layer <= 12)].copy()
    d["f"] = d.k * d.f0 * np.sqrt(1 + d.B * d.k ** 2)
    return pd.Series({m: np.polyfit(np.log2(g.f), g.level_db, 1)[0] for m, g in d.groupby("midi") if g.k.nunique() >= 3})


def curves(df):
    n = df.drop_duplicates(["midi", "layer"])
    c = n.pivot_table(index="midi", columns="layer", values="centroid_hz")
    c = np.log(c.div(c[8], axis=0)).median()
    pk = n.pivot_table(index="midi", columns="layer", values="peak_dbfs")
    return c, pk.sub(pk[16], axis=0).median()


rs = slopes(ref)
rc, rp = curves(ref)
rows = []
for hs, hd, po in itertools.product([0.0, -0.02, -0.04, -0.06, -0.08], [1.0, 2.0], [-1.0, -0.5]):
    path = f"out/trials/board/voicing_hs{hs}_hd{hd}_po{po}.csv"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["target/release/piano-eval", "data/salamander/Samples", "--model-only", "--out", path,
                        "--bridge-mass=0.07", f"--hardness-slope={hs}", f"--hardness={hd}", f"--p-offset={po}"],
                       check=True, stderr=subprocess.DEVNULL)
    m = pd.read_csv(path)
    ms = slopes(m)
    mc, mp = curves(m)
    d = (ms - rs).dropna()
    rows.append(dict(hardness_slope=hs, hardness=hd, p_offset=po, slope_rms=float(np.sqrt((d ** 2).mean())),
                     slope_bias=float(d.mean()), treble_bias=float(d[d.index >= 60].mean()),
                     centroid_rms=float(np.sqrt(((mc - rc) ** 2).mean())), dyn_rms=float(np.sqrt(((mp - rp) ** 2).mean()))))
df = pd.DataFrame(rows)
df["score"] = df.slope_rms / 4 + df.centroid_rms / 0.04 + df.dyn_rms / 2
pd.set_option("display.width", 200)
print(df.sort_values("score").head(10).round(3).to_string(index=False))
df.to_csv("out/trials/voicing_grid.csv", index=False)
