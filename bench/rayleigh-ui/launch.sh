#!/usr/bin/env bash
# Explicit test mode. Keep the user's ordinary configuration and defaults intact.
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repo"
cargo build --release --offline -p piano-play -p piano-render -p piano-analysis --features piano-play/experimental-rayleigh,piano-render/experimental-rayleigh --bins --examples
bundle="$repo/out/upstream-4a00f00/player_actual/bundle.toml"
compiled_fingerprint=$(target/release/examples/rayleigh_live --fingerprint)
if ! python - "$bundle" "$compiled_fingerprint" <<'PY_CHECK'
import sys,tomllib
from pathlib import Path
p=Path(sys.argv[1])
assert p.is_file() and tomllib.loads(p.read_text())['physics_fingerprint']==sys.argv[2]
PY_CHECK
then
    OPENBLAS_NUM_THREADS=1 python bench/upstream-4a00f00/prepare.py
fi
mkdir -p "$repo/out/rayleigh-ui/bin"
cp target/release/unsampled-piano "$repo/out/rayleigh-ui/bin/unsampled-piano-test.new"
mv "$repo/out/rayleigh-ui/bin/unsampled-piano-test.new" "$repo/out/rayleigh-ui/bin/unsampled-piano-test"
test_config=$(mktemp -d /tmp/piano-rayleigh-ui-XXXXXX)
mkdir -p "$test_config/unsampled-piano"
printf 'restore = false\nblackbox = false\n' > "$test_config/unsampled-piano/play.toml"
exec env XDG_CONFIG_HOME="$test_config" "$repo/out/rayleigh-ui/bin/unsampled-piano-test" \
    --ui --experimental-rayleigh "$bundle" --quality full --threads 2 --buffer 512 \
    --name 'Piano TEST — upstream + Rayleigh intégral' --presets "$repo/presets" "$@"
