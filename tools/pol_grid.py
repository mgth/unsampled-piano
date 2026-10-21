#!/usr/bin/env python3
"""Grid search of the horizontal-polarisation parameters against the ESPRIT
statistics of the reference (k ≤ 12), with emphasis on single-string notes
whose aftersound can only come from polarisation."""
import itertools, subprocess, warnings
import numpy as np, pandas as pd

warnings.filterwarnings("ignore")
ref = pd.read_json("out/salamander/k12/esprit_fitted.json").set_index("midi")


def score(m):
    m = m.set_index("midi").reindex(ref.index)
    e = {}
    for tag, sel in (("all", ref.index >= 0), ("single", ref.index <= 27)):
        e[f"{tag}_prompt"] = np.nanmedian(np.abs(np.log(m.t60_prompt[sel] / ref.t60_prompt[sel])))
        e[f"{tag}_after"] = np.nanmedian(np.abs(np.log(m.t60_after[sel] / ref.t60_after[sel])))
        e[f"{tag}_db"] = np.nanmedian(np.abs(m.after_db[sel] - ref.after_db[sel]))
        e[f"{tag}_found"] = float(np.nanmedian(np.abs(m.after_found[sel].fillna(0) - ref.after_found[sel])))
    e["spread"] = np.nanmedian(np.abs(m.spread - ref.spread))
    pen = lambda x: 3.0 if np.isnan(x) else x
    e["score"] = sum(pen(e[f"{t}_prompt"]) + pen(e[f"{t}_after"]) + pen(e[f"{t}_db"]) / 10 + e[f"{t}_found"]
                     for t in ("all", "single")) + e["spread"] / 2
    return e


rows = []
for tilt, rad, cross, hc in itertools.product([0.05, 0.15, 0.3], [0.1, 0.3], [0.1, 0.5, 1.0], [0.1, 0.3]):
    name = f"pol_t{tilt}_r{rad}_x{cross}_h{hc}"
    path = f"out/trials/{name}/esprit_fitted.json"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["tools/unison_trial.sh", name, f"--pol-tilt={tilt}", f"--pol-radiation={rad}",
                        f"--pol-cross={cross}", f"--pol-h-coupling={hc}"], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    rows.append(dict(tilt=tilt, radiation=rad, cross=cross, h_coupling=hc, **score(pd.read_json(path))))
for name in ("pol_off_k12",):
    subprocess.run(["tools/unison_trial.sh", name, "--pol-enabled=false"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    rows.append(dict(tilt=0, radiation=0, cross=0, h_coupling=0, **score(pd.read_json(f"out/trials/{name}/esprit_fitted.json"))))
df = pd.DataFrame(rows).sort_values("score")
df.to_csv("out/trials/pol_grid.csv", index=False)
pd.set_option("display.width", 250)
print(df.head(8).round(3).to_string(index=False))
print(df[df.tilt == 0].round(3).to_string(index=False))
