#!/usr/bin/env python3
"""Bridge conductance from ESPRIT components: with N strings of tension T and
length L, Weinreich's model gives Δσ = σ_prompt − σ_after ≈ N·(T/L)·G(f).
If the model holds, G = Δσ·L/(N·T) depends on frequency, not on the note.

usage: python3 -I tools/bridge_conductance.py out/salamander/components.csv
"""
import sys
from pathlib import Path
import numpy as np, pandas as pd
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

LN1000 = 6.907755
c = pd.read_csv(sys.argv[1])

def strings(m):  # generic grand layout, same as piano-core params
    return 1 if m <= 27 else (2 if m <= 44 else 3)

def length(m, len_c8=0.052, ratio=1.85, brk=48, len_a0=1.45):
    free = lambda x: len_c8 * ratio ** ((108 - x) / 12)
    return free(m) if m >= brk else free(brk) + (brk - m) / (brk - 21) * (len_a0 - free(brk))

T = 750.0
rows = []
for (f, midi, layer, k), g in c[c.layer >= 8].groupby(["file", "midi", "layer", "k"]):
    main = g[g["rank"] == 0].iloc[0]
    aft = g[(g["rank"] > 0) & (g.amp_db_rel >= -40) & (g.t60 > 1.5 * main.t60)].sort_values("amp_db_rel", ascending=False).head(1)
    if not len(aft):
        continue
    ds = LN1000 / main.t60 - LN1000 / aft.t60.iloc[0]
    n = strings(midi)
    if n < 2 or ds <= 0:
        continue
    rows.append(dict(midi=midi, k=k, freq=main.freq, ds=ds, G=ds * length(midi) / (n * T)))
d = pd.DataFrame(rows)
# Does G depend on the note at fixed frequency? Regress log G on log f and midi.
A = np.c_[np.ones(len(d)), np.log(d.freq), d.midi]
coef, *_ = np.linalg.lstsq(A, np.log(d.G), rcond=None)
A2 = np.c_[np.ones(len(d)), np.log(d.freq)]
coef2, *_ = np.linalg.lstsq(A2, np.log(d.G), rcond=None)
res1 = np.log(d.G) - A @ coef
res2 = np.log(d.G) - A2 @ coef2
res0 = np.log(d.ds) - np.c_[np.ones(len(d)), np.log(d.freq)] @ np.linalg.lstsq(np.c_[np.ones(len(d)), np.log(d.freq)], np.log(d.ds), rcond=None)[0]
print(f"{len(d)} partials (bichords/trichords, layers ≥ 8)")
print(f"log G = {coef2[0]:.3f} + {coef2[1]:.3f}·log f          rms {res2.std():.3f}")
print(f"log G = {coef[0]:.3f} + {coef[1]:.3f}·log f + {coef[2]:.4f}·midi   rms {res1.std():.3f}")
print(f"(for comparison, log Δσ vs log f alone: rms {res0.std():.3f})")
for f0 in (100, 300, 1000, 3000, 10000):
    print(f"  G({f0} Hz) ≈ {np.exp(coef2[0] + coef2[1]*np.log(f0)):.2e} s/kg")
fig, ax = plt.subplots(1, 2, figsize=(12, 4.5))
for a, col, lab in ((ax[0], "ds", "Δσ (1/s)"), (ax[1], "G", "G = Δσ·L/(N·T) (s/kg)")):
    sc = a.scatter(d.freq, d[col], c=d.midi, s=6, cmap="viridis", alpha=.6)
    a.set_xscale("log"); a.set_yscale("log"); a.set_xlabel("frequency (Hz)"); a.set_ylabel(lab)
ff = np.logspace(1.7, 4.2, 50)
ax[1].plot(ff, np.exp(coef2[0] + coef2[1] * np.log(ff)), "k--", label="fit")
ax[1].axhline(1.3e-3, color="r", ls=":", label="mean mobility, Ege 2013")
ax[1].legend()
fig.colorbar(sc, ax=ax[1], label="MIDI note")
fig.tight_layout()
fig.savefig(Path(sys.argv[1]).with_name("bridge_conductance.png"), dpi=105)
