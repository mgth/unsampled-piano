#!/usr/bin/env python3
"""Summarise the ESPRIT components CSV of piano-survey.

Per partial: the strongest component at t=0 is the "prompt" sound; the
strongest component decaying ≥1.5× slower is the "aftersound" (Weinreich's
antisymmetric unison mode, which barely loads the bridge). Their decay rates
give intrinsic string losses (≈ σ_after) and bridge-induced losses
(Δσ = σ_prompt − σ_after, ∝ N·Re(Y_bridge)).

usage: python3 -I tools/esprit_summary.py out/salamander/components.csv
"""
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

csv = Path(sys.argv[1])
out = csv.parent
c = pd.read_csv(csv)
LN1000 = 6.907755
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def name(m):
    return f"{NAMES[m % 12]}{m // 12 - 1}"


rows = []
for (f, midi, layer, k), g in c.groupby(["file", "midi", "layer", "k"]):
    main = g[g["rank"] == 0].iloc[0]
    others = g[(g["rank"] > 0) & (g.amp_db_rel >= -40) & (g.t60 > 1.5 * main.t60)]
    aft = others.sort_values("amp_db_rel", ascending=False).head(1)
    strong = g[g.amp_db_rel >= -20]
    rows.append(dict(
        file=f, midi=midi, layer=layer, k=k, freq=main.freq,
        t60_prompt=main.t60,
        t60_after=aft.t60.iloc[0] if len(aft) else np.nan,
        after_db=aft.amp_db_rel.iloc[0] if len(aft) else np.nan,
        after_cents=aft.cents_rel_main.iloc[0] if len(aft) else np.nan,
        spread_cents=strong.cents_rel_main.max() - strong.cents_rel_main.min(),
        n20=len(strong),
    ))
p = pd.DataFrame(rows)
p["sig_p"] = LN1000 / p.t60_prompt
p["sig_a"] = LN1000 / p.t60_after
p["dsig"] = p.sig_p - p.sig_a

KMAX = int(sys.argv[2]) if len(sys.argv) > 2 else 6
loud = p[(p.layer >= 8) & (p.k <= KMAX)]
note = loud.groupby("midi").agg(
    t60_prompt_k1=("t60_prompt", lambda s: s[loud.loc[s.index, "k"] == 1].median()),
    t60_after_k1=("t60_after", lambda s: s[loud.loc[s.index, "k"] == 1].median()),
    t60_prompt=("t60_prompt", "median"),
    t60_after=("t60_after", "median"),
    after_db=("after_db", "median"),
    after_cents=("after_cents", "median"),
    after_found=("t60_after", lambda s: s.notna().mean()),
    spread=("spread_cents", "median"),
    n20=("n20", "median"),
).reset_index()

# Stability across velocity layers: relative IQR of aftersound T60 (k=1..3).
stab = (
    loud[loud.k <= 3].groupby(["midi", "k"]).t60_after
    .agg(lambda s: (s.quantile(.75) - s.quantile(.25)) / s.median() if s.notna().sum() >= 4 else np.nan)
    .groupby("midi").median()
)
note["after_t60_rel_iqr"] = note.midi.map(stab)

# Loss laws per note: σ = b1 + b2·f² fitted on aftersound (≈ string) and on Δσ (bridge).
def fit_law(d, col):
    d = d.dropna(subset=[col])
    d = d[(d[col] > 0) & np.isfinite(d[col])]
    g = d.groupby("k").agg(f=("freq", "median"), s=(col, "median"), n=(col, "size")).reset_index()
    g = g[g.n >= 3]
    if len(g) < 3:
        return np.nan, np.nan, np.nan
    A = np.c_[np.ones(len(g)), g.f ** 2]
    coef, *_ = np.linalg.lstsq(A, g.s, rcond=None)
    pred = A @ coef
    r2 = 1 - np.sum((g.s - pred) ** 2) / max(np.sum((g.s - g.s.mean()) ** 2), 1e-12)
    return coef[0], coef[1], r2

laws = []
for midi, d in p[p.layer >= 8].groupby("midi"):
    a = fit_law(d[d.k <= 12], "sig_a")
    b = fit_law(d[d.k <= 12], "dsig")
    laws.append(dict(midi=midi, b1_string=a[0], b2_string=a[1], r2_string=a[2],
                     b1_bridge=b[0], b2_bridge=b[1], r2_bridge=b[2]))
laws = pd.DataFrame(laws)
note = note.merge(laws, on="midi")

# ---------- plots ----------
fig, ax = plt.subplots(2, 2, figsize=(12, 8))
a = ax[0, 0]
a.plot(note.midi, note.t60_prompt_k1, "o-", ms=4, label="prompt (k=1)")
a.plot(note.midi, note.t60_after_k1, "s-", ms=4, label="aftersound (k=1)")
a.plot(note.midi, note.t60_prompt, "o:", ms=3, label="prompt (k=1..6)")
a.plot(note.midi, note.t60_after, "s:", ms=3, label="aftersound (k=1..6)")
a.set_yscale("log"); a.set_xlabel("MIDI note"); a.set_ylabel("T60 (s)"); a.set_title("Double decay (medians, layers ≥ 8)")
a.legend(fontsize=8)
a = ax[0, 1]
a.plot(note.midi, note.after_db, "o-", ms=4)
a.set_xlabel("MIDI note"); a.set_ylabel("dB / main component"); a.set_title("Aftersound level (at t=0)")
a = ax[1, 0]
a.plot(note.midi, note.spread, "o-", ms=4, label="spread of components ≥ −20 dB")
a.plot(note.midi, note.after_cents.abs(), "s-", ms=4, label="|distance| aftersound / main")
a.set_xlabel("MIDI note"); a.set_ylabel("cents"); a.set_title("Frequency spread in the unison (coupled modes)")
a.legend(fontsize=8)
a = ax[1, 1]
ok = loud.dropna(subset=["dsig"])
ok = ok[(ok.dsig > 0) & (ok.k <= 6)]
sc = a.scatter(ok.freq, ok.dsig, c=ok.midi, s=6, cmap="viridis", alpha=0.6)
a.set_xscale("log"); a.set_yscale("log")
a.set_xlabel("partial frequency (Hz)"); a.set_ylabel("Δσ = σ_prompt − σ_after (1/s)")
a.set_title("Bridge-induced losses (∝ N·Re Y)")
fig.colorbar(sc, ax=a, label="MIDI note")
fig.tight_layout()
fig.savefig(out / "esprit_summary.png", dpi=110)

(out / "esprit_fitted.json").write_text(note.to_json(orient="records", indent=1))

L = [f"# ESPRIT components — {out.name}\n",
     f"{c.file.nunique()} files, {len(p)} partials analysed, {len(c)} components.\n",
     f"Aftersound found for {p[p.layer >= 8].t60_after.notna().mean():.0%} of partials (layers ≥ 8). "
     f"Stability across velocity layers (relative IQR of the aftersound T60, k=1..3): median {note.after_t60_rel_iqr.median():.0%}.\n",
     "| note | T60 prompt k=1 | T60 after k=1 | T60 prompt k≤6 | T60 after k≤6 | after dB | after cents | spread cents | n ≥ −20 dB | rel. IQR | b1 string | b2 string | b1 bridge | b2 bridge |",
     "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
for _, r in note.iterrows():
    L.append(f"| {name(int(r.midi))} | {r.t60_prompt_k1:.1f} | {r.t60_after_k1:.1f} | {r.t60_prompt:.1f} | {r.t60_after:.1f} | "
             f"{r.after_db:.1f} | {r.after_cents:+.2f} | {r.spread:.2f} | {r.n20:.0f} | {r.after_t60_rel_iqr:.0%} | "
             f"{r.b1_string:.3f} | {r.b2_string:.1e} | {r.b1_bridge:.3f} | {r.b2_bridge:.1e} |")
(out / "esprit_summary.md").write_text("\n".join(L) + "\n")
print("\n".join(L))
