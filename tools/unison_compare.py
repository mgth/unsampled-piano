#!/usr/bin/env python3
"""Per-note comparison of ESPRIT statistics, model trial vs reference.
usage: python3 -I tools/unison_compare.py out/trials/<name>/esprit_fitted.json out.png"""
import sys
import pandas as pd
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

ref = pd.read_json(sys.argv[3] if len(sys.argv) > 3 else "out/salamander/esprit_fitted.json")
mod = pd.read_json(sys.argv[1])
fig, ax = plt.subplots(1, 4, figsize=(18, 4))
for a, col, lab, log in ((ax[0], "t60_prompt", "T60 prompt (s, k≤12)", True), (ax[1], "t60_after", "T60 aftersound (s, k≤12)", True),
                         (ax[2], "after_db", "aftersound level (dB / main)", False), (ax[3], "spread", "component spread (cents)", False)):
    a.plot(ref.midi, ref[col], "o-", ms=4, label="Salamander")
    a.plot(mod.midi, mod[col], "s--", ms=4, label="model")
    if log:
        a.set_yscale("log")
    a.set_xlabel("MIDI note"); a.set_title(lab); a.legend(fontsize=8)
fig.tight_layout()
fig.savefig(sys.argv[2], dpi=100)
