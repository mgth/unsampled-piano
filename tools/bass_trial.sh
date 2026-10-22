#!/usr/bin/env bash
# Renders the comparison sequence with a preset, surveys it, and prints the
# median T60 of the bass partials (keys 21–47, ff) next to the references.
#   tools/bass_trial.sh <name> <preset.toml>
set -eu
cd "$(dirname "$0")/.."
n=$1; p=$2; d=out/bass/$n
if [ ! -f "$d.survey.csv" ]; then
  target/release/unsampled-piano-render --preset "$p" --tail 2 midi out/compare/notes.mid "$d.wav" > /dev/null 2>&1
  python3 -I tools/split_notes.py seq "$d.wav" out/compare/notes.txt "$d" && rm "$d.wav"
  target/release/piano-survey "$d" --out "$d.survey.csv" > /dev/null 2>&1
fi
python3 -I - "$d.survey.csv" <<'PY'
import sys, pandas as pd, numpy as np
def per(p, lo, hi):
    d = pd.read_csv(p); d = d[(d.layer == 13) & (d.midi >= lo) & (d.midi <= hi)]
    return d.pivot_table(index="midi", columns="k", values="t60_early", aggfunc="median")
for lo, hi in [(21, 47), (48, 71)]:
    refs = [per(f"out/compare/{n}.survey.csv", lo, hi) for n in ("salamander", "iowa", "commercial")]
    m = per(sys.argv[1], lo, hi)
    def band(t, a, b): return np.nanmedian(t[[k for k in t.columns if a <= k <= b]].values)
    row = []
    for a, b in [(1, 2), (3, 6), (7, 12)]:
        r = np.nanmedian([band(t, a, b) for t in refs])
        row.append(f"k{a}–{b}: {band(m, a, b):5.1f} (ref. {r:4.1f})")
    print(f"keys {lo}–{hi}: " + "  ".join(row))
PY
