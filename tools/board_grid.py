#!/usr/bin/env python3
"""Grid search of the physical high-frequency mechanisms (bridge mass, board
loss growth, felt hysteresis, contact width, mic height) against spectral
tilt, fluctuation, brightness and dynamics of the reference."""
import itertools, subprocess, sys, warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore")
sys.argv = ["x"]
ns = {}
exec(open("tools/spectral_stats.py").read().split("rows = {}")[0], ns)
stats = ns["stats"]
ref_all = pd.read_csv("out/salamander/eval_phase4.csv")
ref = ref_all[ref_all.source == "reference"]
rtilt, rflu = stats(ref)


def curves(df):
    n = df.drop_duplicates(["midi", "layer"])
    c = n.pivot_table(index="midi", columns="layer", values="centroid_hz")
    c = np.log(c.div(c[8], axis=0)).median()
    pk = n.pivot_table(index="midi", columns="layer", values="peak_dbfs")
    return c, pk.sub(pk[16], axis=0).median()


rc, rp = curves(ref)
grid = list(itertools.product([0.07, 0.2], [np.inf, 3000.0, 1000.0], [1.0, 3.0], [1.0, 2.0], [1.5, 3.0]))
rows = []
for bm, lc, hy, cw, mh in grid:
    name = f"bm{bm}_lc{lc}_hy{hy}_cw{cw}_mh{mh}"
    path = f"out/trials/board/{name}.csv"
    try:
        open(path).close()
    except FileNotFoundError:
        subprocess.run(["target/release/piano-eval", "data/salamander/Samples", "--model-only", "--out", path,
                        f"--bridge-mass={bm}", f"--board-loss-corner={lc if np.isfinite(lc) else 1e12}",
                        f"--hysteresis={hy}", f"--contact-width={cw}", f"--mic-height={mh}"],
                       check=True, stderr=subprocess.DEVNULL)
    m = pd.read_csv(path)
    tilt, flu = stats(m)
    mc, mp = curves(m)
    r = dict(bridge_mass=bm, loss_corner=lc, hysteresis=hy, contact_width=cw, mic_h=mh,
             tilt_err=float((tilt - rtilt).abs().mean()), fluct=flu,
             centroid_rms=float(np.sqrt(((mc - rc) ** 2).mean())), dyn_rms=float(np.sqrt(((mp - rp) ** 2).mean())))
    r["score"] = r["tilt_err"] / 3 + abs(flu - rflu) + r["centroid_rms"] / 0.04 + r["dyn_rms"] / 2
    rows.append(r)
df = pd.DataFrame(rows).sort_values("score")
df.to_csv("out/trials/board_grid.csv", index=False)
pd.set_option("display.width", 200)
print(f"reference fluct {rflu:.1f} dB")
print(df.head(10).round(3).to_string(index=False))
