#!/usr/bin/env python3
"""Voicing along the keyboard calibrated on the absolute attack brightness
(centroid of the first 50 ms) per note, keeping brightness-vs-velocity and
dynamics."""
import itertools, subprocess, warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore")
ref_all = pd.read_csv("out/salamander/eval_p5.csv")
ref = ref_all[ref_all.source == "reference"].drop_duplicates(["midi", "layer"])
rc = ref.pivot_table(index="midi", columns="layer", values="centroid_hz")


def stats(m):
    m = m.drop_duplicates(["midi", "layer"])
    mc = m.pivot_table(index="midi", columns="layer", values="centroid_hz")
    lr = np.log(mc / rc).median(axis=1)
    rel = lambda c: np.log(c.div(c[8], axis=0)).median()
    pk = lambda d: (lambda t: t.sub(t[16], axis=0).median())(d.pivot_table(index="midi", columns="layer", values="peak_dbfs"))
    return dict(abs_bias=float(lr.median()), abs_spread=float(lr.std()), abs_rms=float(np.sqrt((lr ** 2).mean())),
                bass=float(lr[lr.index < 50].median()), treble=float(lr[lr.index > 80].median()),
                vel_rms=float(np.sqrt(((rel(mc) - rel(rc)) ** 2).mean())), dyn_rms=float(np.sqrt(((pk(m) - pk(ref)) ** 2).mean())))


rows = []
for hd, bm, lc in itertools.product([0.2, 0.35], [0.0], [700.0, 1000.0, 1500.0]):
    hs, po = 0.02, -0.5
    path = f"out/trials/board/attack3_hd{hd}_bm{bm}_lc{lc:.0f}.csv"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["target/release/piano-eval", "data/salamander/Samples", "--model-only", "--out", path,
                        f"--hardness-slope={hs}", f"--hardness={hd}", f"--p-offset={po}", f"--bridge-mass={bm}",
                        f"--board-loss-corner={lc}"], check=True, stderr=subprocess.DEVNULL)
    rows.append(dict(hardness=hd, bridge_mass=bm, loss_corner=lc, **stats(pd.read_csv(path))))
df = pd.DataFrame(rows)
df["obj"] = df.abs_rms + df.vel_rms / 0.04 * 0.1 + df.dyn_rms / 10
pd.set_option("display.width", 200)
print(df.sort_values("obj").round(3).to_string(index=False))
