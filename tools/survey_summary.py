#!/usr/bin/env python3
"""Summarise a piano-survey CSV: inharmonicity law, stretch, loss laws,
velocity dynamics. Writes summary.md, fitted.json and PNG plots next to it.

usage: python3 -I tools/survey_summary.py out/salamander/survey.csv
"""
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from scipy.optimize import curve_fit, least_squares

csv = Path(sys.argv[1])
outdir = csv.parent
df = pd.read_csv(csv)
notes = df.drop_duplicates(["midi", "layer"])[
    ["file", "midi", "layer", "f0", "cents_et", "B", "n_partials", "k_max", "peak_dbfs", "centroid_hz"]
]

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def name(m):
    return f"{NAMES[m % 12]}{m // 12 - 1}"


# ---------- inharmonicity ----------
# Reliable B needs several partials; weight by partial count.
per_note = (
    notes[notes.n_partials >= 4]
    .groupby("midi")
    .agg(B_med=("B", "median"), B_q1=("B", lambda x: x.quantile(0.25)), B_q3=("B", lambda x: x.quantile(0.75)),
         cents=("cents_et", "median"), f0=("f0", "median"), np=("n_partials", "median"))
    .reset_index()
)
# Treble notes with < 4 partials: keep the best-documented layer.
treble = notes[(notes.n_partials < 4) & ~notes.midi.isin(per_note.midi)].sort_values("n_partials").groupby("midi").last()
for m, r in treble.iterrows():
    per_note.loc[len(per_note)] = dict(midi=m, B_med=r.B, B_q1=r.B, B_q3=r.B, cents=r.cents_et, f0=r.f0, np=r.n_partials)
per_note = per_note.sort_values("midi").reset_index(drop=True)


def rigaud(m, aB, bB, aT, bT):
    return np.exp(aB * m + bB) + np.exp(aT * m + bT)


RIGAUD_REF = (-0.0847, -5.82, 0.0926, -13.64)
m = per_note.midi.values.astype(float)
logB = np.log(per_note.B_med.values)
res = least_squares(lambda p: np.log(rigaud(m, *p)) - logB, RIGAUD_REF, loss="soft_l1", f_scale=0.3)
rig_fit = res.x
rig_rms = float(np.sqrt(np.mean((np.log(rigaud(m, *rig_fit)) - logB) ** 2)))

# Spread of B across velocity layers (should be ~0 for a linear string).
spread = notes[notes.n_partials >= 6].groupby("midi").B.agg(lambda x: x.std() / x.median()).median()

# ---------- stretch ----------
# f0 here is the fitted virtual f0; compare with ET A4=440.

# ---------- losses ----------
d = df.dropna(subset=["t60_early"]).copy()
d = d[(d.t60_early > 0) & (d.t60_early < 200)]
d["sigma_early"] = 6.9078 / d.t60_early
d["sigma_late"] = 6.9078 / d.t60_late
loud = d[d.layer >= 8]  # good SNR

loss_rows = []
for midi, g in loud.groupby("midi"):
    # Median over layers per partial, then σ = b1 + b2 f² by weighted LS.
    gk = g.groupby("k").agg(f=("freq", "median"), s=("sigma_early", "median"), n=("sigma_early", "size")).reset_index()
    gk = gk[gk.n >= 3]
    if len(gk) < 3:
        continue
    A = np.c_[np.ones(len(gk)), gk.f.values ** 2]
    coef, *_ = np.linalg.lstsq(A, gk.s.values, rcond=None)
    pred = A @ coef
    r2 = 1 - np.sum((gk.s - pred) ** 2) / max(np.sum((gk.s - gk.s.mean()) ** 2), 1e-12)
    loss_rows.append(dict(midi=midi, b1=coef[0], b2=coef[1], r2=r2, n=len(gk),
                          t60_k1=6.9078 / gk.s.values[0] if gk.k.values[0] == 1 else np.nan))
loss = pd.DataFrame(loss_rows)

# Smooth laws across the keyboard: log b1 linear in midi; b2 median.
good = loss[(loss.b1 > 0)]
pb1 = np.polyfit(good.midi, np.log(good.b1), 1)
b2_med = float(np.median(loss.b2[loss.b2 > 0])) if (loss.b2 > 0).any() else float("nan")

# Late / early ratio: "aftersound".
d["late_over_early"] = d.t60_late / d.t60_early
ratio = d[(d.layer >= 8) & d.t60_late.notna()].groupby("midi").late_over_early.median()

# ---------- velocity ----------
vel = notes.groupby("layer").agg(peak=("peak_dbfs", "median"), centroid=("centroid_hz", "median")).reset_index()
dyn_range = float(vel.peak.max() - vel.peak.min())
# Per note: centroid ratio between loudest and softest layers.
cr = notes.pivot_table(index="midi", columns="layer", values="centroid_hz")
centroid_ratio = (cr[16] / cr[1]).median()

# ---------- plots ----------
fig, ax = plt.subplots(2, 2, figsize=(12, 8))
mm = np.arange(21, 109)
a = ax[0, 0]
a.errorbar(per_note.midi, per_note.B_med, yerr=[per_note.B_med - per_note.B_q1, per_note.B_q3 - per_note.B_med],
           fmt="o", ms=4, label="Salamander (median over layers)")
a.plot(mm, rigaud(mm, *RIGAUD_REF), "--", label="Rigaud 2013 (mean of pianos)")
a.plot(mm, rigaud(mm, *rig_fit), "-", label="Salamander fit")
a.set_yscale("log"); a.set_xlabel("MIDI note"); a.set_ylabel("B"); a.set_title("Inharmonicity"); a.legend(fontsize=8)
a = ax[0, 1]
a.plot(per_note.midi, per_note.cents, "o-", ms=4)
a.axhline(0, color="gray", lw=0.5)
a.set_xlabel("MIDI note"); a.set_ylabel("cents / ET 440"); a.set_title("Tuning (fitted virtual f0)")
a = ax[1, 0]
for k in (1, 2, 3, 6):
    g = loud[loud.k == k].groupby("midi").t60_early.median()
    a.plot(g.index, g.values, "o-", ms=3, label=f"k={k}")
a.plot(mm, 6.9078 / (np.exp(np.polyval(pb1, mm)) + b2_med * (440 * 2 ** ((mm - 69) / 12)) ** 2), "k--",
       label="fitted law k=1")
a.set_yscale("log"); a.set_xlabel("MIDI note"); a.set_ylabel("early T60 (s)"); a.set_title("Decay (layers ≥ 8)")
a.legend(fontsize=8)
a = ax[1, 1]
a2 = a.twinx()
a.plot(vel.layer, vel.peak, "o-", label="peak dBFS")
a2.plot(vel.layer, vel.centroid, "s-", color="C1", label="attack centroid")
a.set_xlabel("velocity layer"); a.set_ylabel("peak (dBFS)"); a2.set_ylabel("centroid (Hz)")
a.set_title("Dynamics (median over notes)")
fig.tight_layout()
fig.savefig(outdir / "summary.png", dpi=110)

# ---------- outputs ----------
fitted = {
    "source": str(csv),
    "inharmonicity_rigaud": dict(zip(["aB", "bB", "aT", "bT"], map(float, rig_fit)), rms_log=rig_rms),
    "loss": {"b1_log_poly": list(map(float, pb1)), "b2": b2_med,
             "law": "sigma_k = exp(b1_log_poly[0]*midi + b1_log_poly[1]) + b2*f_k^2  (1/s, early decay)"},
    "per_note": per_note.to_dict(orient="records"),
    "loss_per_note": loss.to_dict(orient="records"),
}
(outdir / "fitted.json").write_text(json.dumps(fitted, indent=1))

L = []
L.append(f"# Survey {csv.parent.name}\n")
L.append(f"{notes.file.nunique()} files, {notes.midi.nunique()} notes, {notes.layer.nunique()} layers.\n")
L.append("## Inharmonicity\n")
L.append(f"Refitted Rigaud law: aB={rig_fit[0]:.4f}, bB={rig_fit[1]:.3f}, aT={rig_fit[2]:.4f}, bT={rig_fit[3]:.3f} "
         f"(rms error {rig_rms:.2f} in log B). Spread of B across velocity layers (median CV): {spread:.1%}.\n")
L.append("| note | f0 Hz | cents/ET | median B | B Rigaud ref. | partials |")
L.append("|---|---|---|---|---|---|")
for _, r in per_note.iterrows():
    L.append(f"| {name(int(r.midi))} | {r.f0:.2f} | {r.cents:+.1f} | {r.B_med:.2e} | {rigaud(r.midi, *RIGAUD_REF):.2e} | {int(r.np)} |")
L.append("\n## Losses (early decay, layers ≥ 8)\n")
L.append(f"σ_k ≈ b1(note) + b2·f_k² ; log b1 = {pb1[0]:.4f}·midi {pb1[1]:+.3f} ; median b2 = {b2_med:.2e}.\n")
L.append("| note | b1 (1/s) | b2 | R² | T60 k=1 (s) | late / early T60 (median) |")
L.append("|---|---|---|---|---|---|")
for _, r in loss.iterrows():
    L.append(f"| {name(int(r.midi))} | {r.b1:.3f} | {r.b2:.2e} | {r.r2:.2f} | {r.t60_k1:.1f} | {ratio.get(r.midi, np.nan):.2f} |")
L.append("\n## Velocity\n")
L.append(f"Peak difference between layers 1 and 16: {dyn_range:.1f} dB (median over notes). "
         f"Attack centroid ratio layer 16 / layer 1: ×{centroid_ratio:.2f}.\n")
L.append("| layer | peak dBFS | centroid Hz |")
L.append("|---|---|---|")
for _, r in vel.iterrows():
    L.append(f"| {int(r.layer)} | {r.peak:.1f} | {r.centroid:.0f} |")
(outdir / "summary.md").write_text("\n".join(L) + "\n")
print("\n".join(L[:6]))
print(f"loss: log b1 = {pb1[0]:.4f} m {pb1[1]:+.3f}, b2 = {b2_med:.2e}; dyn range {dyn_range:.1f} dB; centroid ×{centroid_ratio:.2f}")
