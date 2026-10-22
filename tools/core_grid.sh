#!/usr/bin/env bash
# Grid over the two-layer hammer felt: top felt (hardness, p_offset) and
# core (hardness, depths bass:treble in mm); renders, splits and scores.
#   tools/core_grid.sh "<hardness>" "<p_offset>" "<core_hardness>" "<depth_bass:depth_treble>" [extra preset lines]
set -u
cd "$(dirname "$0")/.."
out=out/trials/core; mkdir -p $out
jobs=0; dirs=""
for h in $1; do for p in $2; do for c in $3; do for d in $4; do
  n="h${h}_p${p}_c${c}_d${d/:/-}"; dirs="$dirs $out/$n"
  [ -f "$out/$n/C8v13.wav" ] && continue
  (
    printf 'hardness = %s\np_offset = %s\np_bass = 0.0\nhardness_slope_bass = 0.02\nhysteresis = 0.6\ncore_hardness = %s\ncore_depth_bass = %s\ncore_depth_treble = %s\n%b[room]\nenabled = false\n' "$h" "$p" "$c" "${d%%:*}" "${d##*:}" "${5:-}" > "$out/$n.toml"
    target/release/unsampled-piano-render --preset "$out/$n.toml" --tail 2 midi out/compare/notes.mid "$out/$n.wav" > /dev/null 2>&1
    python3 -I tools/split_notes.py seq "$out/$n.wav" out/compare/notes.txt "$out/$n" && rm "$out/$n.wav"
  ) &
  jobs=$((jobs + 1)); [ $((jobs % 12)) -eq 0 ] && wait
done; done; done; done
wait
python3 -I tools/hammer_score.py out/compare/refs.json $dirs 2>/dev/null | sort -n | head -10
