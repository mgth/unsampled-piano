#!/usr/bin/env python3
"""Calibrate the modal-board coupling (mobility scale, series stiffness,
bridge mass) on the ESPRIT decay statistics (bridge force output)."""
import itertools, subprocess, sys, warnings
import pandas as pd
warnings.filterwarnings("ignore")
sys.argv = ["x"]
ns = {}
exec(open("tools/pol_grid.py").read().split("rows = []")[0], ns)
rows = []
for ms, ks, bm in itertools.product([1.0, 1.4, 2.0], [1e12, 2e7, 5e6], [0.0, 0.07]):
    name = f"cpl_ms{ms}_ks{ks:.0e}_bm{bm}"
    path = f"out/trials/{name}/esprit_fitted.json"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["tools/unison_trial.sh", name, "--no-radiation", f"--mobility-scale={ms}",
                        f"--series-stiffness={ks}", f"--bridge-mass={bm}"], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    r = ns["score"](pd.read_json(path))
    rows.append(dict(mobility_scale=ms, series_k=ks, bridge_mass=bm, **{k: float(v) for k, v in r.items()}))
df = pd.DataFrame(rows).sort_values("score")
df.to_csv("out/trials/coupling_grid.csv", index=False)
pd.set_option("display.width", 250)
cols = ["mobility_scale", "series_k", "bridge_mass", "all_prompt", "all_after", "all_db", "single_after", "single_db", "spread", "score"]
print(df[cols].head(8).round(3).to_string(index=False))
