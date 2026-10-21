#!/usr/bin/env bash
# End-to-end calibration report against a reference sample set whose files
# are named <note>v<layer>.{flac,wav} (e.g. Salamander).
#   tools/calibrate.sh <samples dir> <output dir> [preset.toml]
# Produces: survey + ESPRIT statistics of the reference, the same for the
# model, spectral/dynamics comparison and the bridge-conductance estimate.
set -euo pipefail
ref=$1; out=$2; preset=${3:-}
mkdir -p "$out"
cargo build --release -q --workspace
p=(); [ -n "$preset" ] && p=(--preset "$preset")

echo "1/4 reference survey (partials, B, decays, ESPRIT components)"
target/release/piano-survey "$ref" --out "$out/ref_survey.csv" --esprit "$out/ref_components.csv" 2>/dev/null
mkdir -p "$out/ref"
cp "$out/ref_survey.csv" "$out/ref/survey.csv"; cp "$out/ref_components.csv" "$out/ref/components.csv"
python3 -I tools/survey_summary.py "$out/ref/survey.csv" >/dev/null
python3 -I tools/esprit_summary.py "$out/ref/components.csv" 12 >/dev/null
python3 -I tools/bridge_conductance.py "$out/ref/components.csv" > "$out/ref/bridge_conductance.txt"

echo "2/4 model renders of the same notes and velocities"
target/release/piano-eval "$ref" "${p[@]}" --out "$out/eval.csv" --write-wav "$out/model_wav" --seconds 10 2>/dev/null

echo "3/4 model survey with the same analysers"
mkdir -p "$out/model"
target/release/piano-survey "$out/model_wav" --out "$out/model/survey.csv" --esprit "$out/model/components.csv" 2>/dev/null
python3 -I tools/esprit_summary.py "$out/model/components.csv" 12 >/dev/null

echo "4/4 comparisons"
python3 -I tools/eval_compare.py "$out/eval.csv" > "$out/eval_summary.txt"
python3 -I tools/spectral_stats.py model="$out/eval.csv" > "$out/spectral_stats.txt"
python3 -I tools/unison_compare.py "$out/model/esprit_fitted.json" "$out/unison_compare.png" "$out/ref/esprit_fitted.json" 2>/dev/null
echo "report in $out: eval.png, eval_summary.txt, spectral_stats.txt, unison_compare.png, ref/*.png"
