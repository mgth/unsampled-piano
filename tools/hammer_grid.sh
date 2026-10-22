#!/usr/bin/env bash
# Grid over the hammer laws: renders the comparison sequence for each
# combination (preset keys hardness / hardness_slope / p_offset /
# hysteresis), splits it and scores it with tools/hammer_score.py.
#   tools/hammer_grid.sh "<hardness list>" "<slope list>" "<p_offset list>" "<hysteresis list>" ["<vmin:vmax list>"] ["<bass slope list>"]
set -u
cd "$(dirname "$0")/.."
out=out/trials/hammer2${PB:+_pb$PB}
mkdir -p "$out"
cargo build --release -q -p piano-render
jobs=0
V=${5:-"0.3:5.5"}
SB=${6:-""}
for h in $1; do for s in $2; do for p in $3; do for y in $4; do for vv in $V; do for sb in ${SB:-same}; do
  vmin=${vv%%:*}; vmax=${vv##*:}
  bass=$s; [ "$sb" != same ] && bass=$sb
  n="h${h}_s${s}_p${p}_y${y}"; [ "$vv" != "0.3:5.5" ] && n="${n}_v${vmin}-${vmax}"; [ "$sb" != same ] && n="${n}_sb${sb}"
  [ -f "$out/$n/C8v13.wav" ] && continue
  (
    printf 'hardness = %s\nhardness_slope = %s\nhardness_slope_bass = %s\np_offset = %s\np_bass = %s\nhysteresis = %s\n[room]\nenabled = false\n[velocity]\nvmin = %s\nvmax = %s\n' "$h" "$s" "$bass" "$p" "${PB:-0}" "$y" "$vmin" "$vmax" > "$out/$n.toml"
    target/release/unsampled-piano-render --preset "$out/$n.toml" --tail 2 midi out/compare/notes.mid "$out/$n.wav" > /dev/null 2>&1
    python3 -I tools/split_notes.py seq "$out/$n.wav" out/compare/notes.txt "$out/$n" && rm "$out/$n.wav"
  ) &
  jobs=$((jobs + 1)); [ $((jobs % 12)) -eq 0 ] && wait
done; done; done; done; done; done
wait
dirs=$(for h in $1; do for s in $2; do for p in $3; do for y in $4; do for vv in $V; do for sb in ${SB:-same}; do
  n="$out/h${h}_s${s}_p${p}_y${y}"; [ "$vv" != "0.3:5.5" ] && n="${n}_v${vv%%:*}-${vv##*:}"; [ "$sb" != same ] && n="${n}_sb${sb}"; echo "$n"; done; done; done; done; done; done)
python3 -I tools/hammer_score.py out/compare/refs.json $dirs 2>/dev/null | sort -n | head -12
