#!/usr/bin/env python3
"""Compare model and reference from a piano-eval CSV: velocity dynamics,
attack brightness, partial-level profiles, hammer contact.

usage: python3 -I tools/eval_compare.py out/salamander/eval_phase1.csv
"""
import sys
from pathlib import Path

import numpy as np
import pandas as pd
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

csv = Path(sys.argv[1])
d = pd.read_csv(csv)
stem = csv.stem
notes = d.drop_duplicates(["source", "midi", "layer"])
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def name(m):
    return f"{NAMES[m % 12]}{m // 12 - 1}"


def rel_to_layer(col, ref_layer, log):
    t = notes.pivot_table(index=["source", "midi"], columns="layer", values=col)
    t = (t.sub(t[ref_layer], axis=0) if log else t.div(t[ref_layer], axis=0))
    return t.groupby("source").median().T


peak = rel_to_layer("peak_dbfs", 16, True)
cent = rel_to_layer("centroid_hz", 8, False)

# Partial profiles: level relative to the strongest of k=1..12.
p = d[d.k <= 12].copy()
p["rel"] = p.level_db - p.groupby(["source", "midi", "layer"]).level_db.transform("max")
# Shape error: level differences on the partials found in both, up to a
# constant offset per (note, layer) — robust to a missing strongest partial.
w = p.pivot_table(index=["midi", "layer", "k"], columns="source", values="level_db").dropna()
diff = w.model - w.reference
w["err"] = (diff - diff.groupby(level=["midi", "layer"]).transform("median")).abs()
prof_err = w.groupby(["midi"]).err.median()
prof_err_layer = w.groupby(["layer"]).err.median()

fig, ax = plt.subplots(2, 3, figsize=(16, 8.5))
a = ax[0, 0]
for src, st in (("reference", "o-"), ("model", "s--")):
    a.plot(peak.index, peak[src], st, label=src)
a.set_xlabel("layer"); a.set_ylabel("peak / layer 16 (dB)"); a.set_title("Dynamics"); a.legend()
a = ax[0, 1]
for src, st in (("reference", "o-"), ("model", "s--")):
    a.plot(cent.index, cent[src], st, label=src)
a.set_xlabel("layer"); a.set_ylabel("centroid / layer 8"); a.set_title("Attack brightness (50 ms)"); a.legend()
a = ax[0, 2]
c = notes[notes.source == "model"]
for layer in (1, 8, 16):
    g = c[c.layer == layer]
    a.plot(g.midi, g.contact_s * 1e3, "o-", ms=3, label=f"layer {layer} (v={int(g.velocity.iloc[0])})")
a.set_xlabel("MIDI note"); a.set_ylabel("contact duration (ms)"); a.set_title("Hammer contact (model)"); a.legend()
for j, (midi, layer) in enumerate(((33, 12), (60, 12), (84, 12))):
    a = ax[1, j]
    for src, st in (("reference", "o-"), ("model", "s--")):
        g = p[(p.source == src) & (p.midi == midi) & (p.layer == layer)]
        a.plot(g.k, g.rel, st, label=src)
    a.set_xlabel("partial k"); a.set_ylabel("dB / max"); a.set_ylim(-60, 3)
    a.set_title(f"Profile {name(midi)}, layer {layer} (median error {prof_err.get(midi, np.nan):.1f} dB)")
    a.legend()
fig.tight_layout()
out_png = csv.with_name(stem + ".png")
fig.savefig(out_png, dpi=105)

L = [f"# Evaluation {stem}\n",
     "| layer | peak ref. | peak model | centroid ref. | centroid model | profile error (dB) |",
     "|---|---|---|---|---|---|"]
for layer in peak.index:
    L.append(f"| {layer} | {peak.reference[layer]:+.1f} | {peak.model[layer]:+.1f} | ×{cent.reference[layer]:.2f} | "
             f"×{cent.model[layer]:.2f} | {prof_err_layer.get(layer, np.nan):.1f} |")
L.append("\n| note | median profile error (dB, k ≤ 12) | contact layer 1 / 16 (ms) | max force layer 16 (N) |")
L.append("|---|---|---|---|")
for midi, e in prof_err.items():
    g = c[c.midi == midi]
    L.append(f"| {name(midi)} | {e:.1f} | {g[g.layer == 1].contact_s.iloc[0]*1e3:.2f} / {g[g.layer == 16].contact_s.iloc[0]*1e3:.2f} | "
             f"{g[g.layer == 16].max_force_n.iloc[0]:.0f} |")
L.append(f"\nOverall profile error (median): {w.err.median():.1f} dB.")
csv.with_name(stem + ".md").write_text("\n".join(L) + "\n")
print("\n".join(L[:19]))
print(L[-1])
