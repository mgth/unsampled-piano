#!/usr/bin/env bash
# Render the Salamander notes (loud layers) with model overrides, analyse them
# with ESPRIT exactly like the recordings. Usage: tools/unison_trial.sh NAME [piano-eval overrides...]
set -euo pipefail
name=$1; shift
dir=out/trials/$name
mkdir -p "$dir"
target/release/piano-eval data/salamander/Samples --model-only --layers 8,12,16 --seconds 10 \
    --write-wav "$dir/wav" --out "$dir/eval.csv" "$@" 2>/dev/null
target/release/piano-survey "$dir/wav" --out "$dir/survey.csv" --esprit "$dir/components.csv" 2>/dev/null
python3 -I tools/esprit_summary.py "$dir/components.csv" 12 > /dev/null
echo "$dir/esprit_fitted.json"
