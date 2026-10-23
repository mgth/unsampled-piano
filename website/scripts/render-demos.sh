#!/usr/bin/env bash
#
# Render the home page's audio demos with the engine itself and encode them
# to small MP3s in public/media/. Needs cargo, python3, ffmpeg and lame.
#
# Usage:  npm run demos
set -euo pipefail

cd "$(dirname "$0")/.."
REPO="$(cd .. && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
OUT=public/media
PEAKS=src/data/demo-peaks.json
mkdir -p "$OUT"

# RENDER=<path to unsampled-piano-render> skips the build.
if [[ -z "${RENDER:-}" ]]; then
  cargo build -q --release -p piano-render --manifest-path "$REPO/Cargo.toml"
  RENDER="$(cargo metadata -q --format-version 1 --no-deps --manifest-path "$REPO/Cargo.toml" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/release/unsampled-piano-render"
fi

python3 -I scripts/demo_midi.py "$WORK"
MICS="$REPO/presets/listening/Microphones"

render() { # <midi> <preset> <wav> [tail]
  "$RENDER" --preset "$2" --tail "${4:-3}" midi "$1" "$3"
}
# <wav> <mp3> [seconds]: optional cut with a short fade, peak-normalised to
# -1 dBFS, then VBR MP3 (~130 kb/s).
encode() {
  local cut=() fade=""
  if [[ -n "${3:-}" ]]; then
    cut=(-t "$3")
    fade=",afade=t=out:st=$(python3 -c "print($3 - 1.5)"):d=1.5"
  fi
  local peak
  peak=$(ffmpeg -hide_banner -nostats -i "$1" "${cut[@]}" -af volumedetect -f null - 2>&1 \
    | sed -n 's/.*max_volume: \(-\?[0-9.]*\) dB/\1/p')
  ffmpeg -hide_banner -loglevel error -y -i "$1" "${cut[@]}" \
    -af "volume=$(python3 -c "print(-1 - ($peak))")dB$fade" -ar 44100 "$WORK/enc.wav"
  lame --quiet -V 5 --tt "$(basename "$2" .mp3)" --ta "Unsampled Piano" "$WORK/enc.wav" "$2"
  python3 -I scripts/demo_peaks.py "$WORK/enc.wav" "$(basename "$2" .mp3)" "$PEAKS"
}

render "$WORK/prelude.mid" "$MICS/concert.toml" "$WORK/prelude_concert.wav" 2
render "$WORK/prelude.mid" "$MICS/close.toml" "$WORK/prelude_close.wav" 2
render "$WORK/pedal.mid" "$MICS/concert.toml" "$WORK/pedal.wav" 2
render "$WORK/dynamics.mid" "$MICS/concert.toml" "$WORK/dynamics.wav" 2

encode "$WORK/prelude_concert.wav" "$OUT/prelude.mp3"
encode "$WORK/prelude_concert.wav" "$OUT/mics-concert.mp3" 16
encode "$WORK/prelude_close.wav" "$OUT/mics-close.mp3" 16
encode "$WORK/pedal.wav" "$OUT/pedal.mp3"
encode "$WORK/dynamics.wav" "$OUT/dynamics.mp3"
ls -l "$OUT"
