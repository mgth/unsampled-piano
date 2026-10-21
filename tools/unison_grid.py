#!/usr/bin/env python3
"""Grid search of unison width, bridge susceptance and conductance scale so
that the model's ESPRIT statistics match the reference (Salamander)."""
import itertools, json, subprocess, sys, warnings
import numpy as np, pandas as pd

warnings.filterwarnings("ignore")
ref = pd.read_json("out/salamander/esprit_fitted.json").set_index("midi")
COLS = ["t60_prompt", "t60_after", "after_db", "after_found", "spread"]


def score(m):
    m = m.set_index("midi").reindex(ref.index)
    e = {}
    e["prompt_log"] = np.nanmedian(np.abs(np.log(m.t60_prompt / ref.t60_prompt)))
    e["after_log"] = np.nanmedian(np.abs(np.log(m.t60_after / ref.t60_after)))
    e["after_db"] = np.nanmedian(np.abs(m.after_db - ref.after_db))
    e["found"] = float(np.nanmedian(np.abs(m.after_found.fillna(0) - ref.after_found)))
    e["spread_c"] = np.nanmedian(np.abs(m.spread - ref.spread))
    e["score"] = e["prompt_log"] + e["after_log"] + e["after_db"] / 10 + e["found"] + e["spread_c"] / 2
    return e


rows = []
grid = list(itertools.product([0.7, 1.0, 1.5, 2.5], [1.0, 1.5, 2.5, 4.0], [0.7, 1.0]))
for wm, bg, gs in grid:
    name = f"w{wm}_b{bg}_g{gs}"
    path = f"out/trials/{name}/esprit_fitted.json"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["tools/unison_trial.sh", name, f"--unison-mid={wm}", f"--unison-treble={3 * wm}",
                        f"--bridge-b-over-g={bg}", f"--bridge-g={4.6e-4 * gs}"], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    r = dict(width_mid=wm, b_over_g=bg, g_scale=gs, **score(pd.read_json(path)))
    rows.append(r)
    print({k: (round(v, 3) if isinstance(v, float) else v) for k, v in r.items()}, flush=True)
df = pd.DataFrame(rows).sort_values("score")
df.to_csv("out/trials/unison_grid.csv", index=False)
print(df.head(8).to_string(index=False))
