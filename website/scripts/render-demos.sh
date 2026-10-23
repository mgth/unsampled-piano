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
# <wav> [seconds]: peak level (dBFS) and integrated loudness (LUFS).
peak() {
  ffmpeg -hide_banner -nostats -i "$1" ${2:+-t "$2"} -af volumedetect -f null - 2>&1 \
    | sed -n 's/.*max_volume: \(-\?[0-9.]*\) dB/\1/p'
}
lufs() {
  ffmpeg -hide_banner -nostats -i "$1" ${2:+-t "$2"} -af ebur128 -f null - 2>&1 \
    | sed -n 's/^ *I: *\(-\?[0-9.]*\) LUFS/\1/p' | tail -1
}
# <wav> <mp3> [seconds] [gain dB]: optional cut with a short fade, the gain
# (by default: peak-normalised to -1 dBFS), then VBR MP3 (~130 kb/s).
encode() {
  local cut=() fade="" gain="${4:-}"
  if [[ -n "${3:-}" ]]; then
    cut=(-t "$3")
    fade=",afade=t=out:st=$(python3 -c "print($3 - 1.5)"):d=1.5"
  fi
  [[ -n "$gain" ]] || gain=$(python3 -c "print(-1 - ($(peak "$1")))")
  ffmpeg -hide_banner -loglevel error -y -i "$1" "${cut[@]}" \
    -af "volume=${gain}dB$fade" -ar 44100 "$WORK/enc.wav"
  lame --quiet -V 5 --tt "$(basename "$2" .mp3)" --ta "Unsampled Piano" ${CREDIT:+--tc "$CREDIT"} \
    "$WORK/enc.wav" "$2"
  python3 -I scripts/demo_peaks.py "$WORK/enc.wav" "$(basename "$2" .mp3)" "$PEAKS"
}

render "$WORK/clair.mid" "$MICS/concert.toml" "$WORK/clair_concert.wav" 2
render "$WORK/clair.mid" "$MICS/close.toml" "$WORK/clair_close.wav" 2
render "$WORK/pedal.mid" "$MICS/concert.toml" "$WORK/pedal.wav" 2
render "$WORK/dynamics.mid" "$MICS/concert.toml" "$WORK/dynamics.wav" 2

# Adaptations of Bernd Krüger's MIDI file: credit and licence in the tags.
CREDIT="MIDI: Bernd Krueger, piano-midi.de, CC BY-SA 3.0 DE; this audio: same licence"
encode "$WORK/clair_concert.wav" "$OUT/clair.mp3"
# The microphone pair at one loudness, as loud as the louder peak allows:
# switching compares the sound, not the level.
MIC_CUT=23
read -r g_concert g_close < <(python3 -c "
p, l = [$(peak "$WORK/clair_concert.wav" $MIC_CUT), $(peak "$WORK/clair_close.wav" $MIC_CUT)], \
       [$(lufs "$WORK/clair_concert.wav" $MIC_CUT), $(lufs "$WORK/clair_close.wav" $MIC_CUT)]
target = min(-1 - pk + lu for pk, lu in zip(p, l))
print(*(round(target - lu, 2) for lu in l))")
encode "$WORK/clair_concert.wav" "$OUT/mics-concert.mp3" $MIC_CUT "$g_concert"
encode "$WORK/clair_close.wav" "$OUT/mics-close.mp3" $MIC_CUT "$g_close"
CREDIT=
encode "$WORK/pedal.wav" "$OUT/pedal.mp3"
encode "$WORK/dynamics.wav" "$OUT/dynamics.mp3"
ls -l "$OUT"
