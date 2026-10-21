#!/usr/bin/env python3
"""Recalibrate the decays of the full instrument (board, radiation,
sympathy): horizontal coupling, passive cross coupling and board mobility,
scored on signed T60 biases and aftersound level versus the reference."""
import itertools, subprocess, sys, warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore")
MODE = sys.argv[1:]
sys.argv = ["x"]
ns = {}
exec(open("tools/pol_grid.py").read().split("rows = []")[0], ns)
ref = ns["ref"]
rows = []
GRID = [(h, f, ms, ls) for h, f, ms in itertools.product([0.03, 0.1, 0.3], [0.5, 1.0], [0.7, 1.0]) for ls in [1.0]]
if MODE[:1] == ["losses"]:
    GRID = [(h, f, ms, ls) for (h, f) in [(0.1, 1.0), (0.03, 1.0)] for ms in [0.7, 1.0] for ls in [0.45, 0.6, 0.75]]
for h, frac, ms, ls in GRID:
    cross = frac * np.sqrt(h)
    name = f"dec_h{h}_x{cross:.3f}_m{ms}" + ("" if ls == 1.0 else f"_l{ls}")
    path = f"out/trials/{name}/esprit_fitted.json"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["tools/unison_trial.sh", name, f"--pol-h-coupling={h}", f"--pol-cross={cross}",
                        f"--mobility-scale={ms}", f"--loss-scale={ls}"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    m = pd.read_json(path).set_index("midi").reindex(ref.index)
    bias = lambda col, sel: float(np.nanmedian(np.log(m[col][sel] / ref[col][sel])))
    mid = (ref.index >= 60)
    r = dict(h=h, cross=round(cross, 3), mobility=ms, losses=ls,
             prompt_bias=bias("t60_prompt", ref.index >= 0), after_bias=bias("t60_after", ref.index >= 0),
             prompt_bias_hi=bias("t60_prompt", mid), after_bias_hi=bias("t60_after", mid),
             **{k: float(v) for k, v in ns["score"](pd.read_json(path)).items() if k in ("all_db", "single_db", "score")})
    r["obj"] = r["score"] + abs(r["prompt_bias_hi"]) + abs(r["after_bias_hi"])
    rows.append(r)
df = pd.DataFrame(rows).sort_values("obj")
pd.set_option("display.width", 220)
print(df.round(3).to_string(index=False))
df.to_csv("out/trials/decay_grid.csv", index=False)
