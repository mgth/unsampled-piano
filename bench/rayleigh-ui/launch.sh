#!/usr/bin/env bash
# Explicit test mode. Keep the user's ordinary configuration and defaults intact.
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repo"
cargo build --release --offline -p piano-play --features experimental-rayleigh
bundle="$repo/out/rayleigh-ui/player_actual/bundle.toml"
if [[ ! -f "$bundle" ]]; then
    OPENBLAS_NUM_THREADS=1 python bench/rayleigh-ui/export.py
fi
mkdir -p "$repo/out/rayleigh-ui/bin"
cp target/release/unsampled-piano "$repo/out/rayleigh-ui/bin/unsampled-piano-test.new"
mv "$repo/out/rayleigh-ui/bin/unsampled-piano-test.new" "$repo/out/rayleigh-ui/bin/unsampled-piano-test"
test_config=$(mktemp -d /tmp/piano-rayleigh-ui-XXXXXX)
mkdir -p "$test_config/unsampled-piano"
printf 'restore = false\nblackbox = false\n' > "$test_config/unsampled-piano/play.toml"
exec env XDG_CONFIG_HOME="$test_config" "$repo/out/rayleigh-ui/bin/unsampled-piano-test" \
    --ui --experimental-rayleigh "$bundle" --quality full --threads 2 --buffer 512 \
    --name 'Piano TEST — Rayleigh intégral + contacts' --presets "$repo/presets" "$@"
