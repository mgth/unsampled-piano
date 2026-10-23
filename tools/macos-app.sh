#!/usr/bin/env bash
# The standalone player as a macOS application bundle, universal (Intel and
# Apple silicon), ad-hoc signed: target/macos/Unsampled Piano.app, with the
# presets in its resources. Run on a Mac (Xcode command-line tools) with
# both Rust targets: rustup target add x86_64-apple-darwin aarch64-apple-darwin
set -euo pipefail
cd "$(dirname "$0")/.."

target=${CARGO_TARGET_DIR:-target}
app="$target/macos/Unsampled Piano.app"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

for triple in x86_64-apple-darwin aarch64-apple-darwin; do
    cargo build --profile dist -p piano-play --target "$triple"
done

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/unsampled-piano" \
    "$target/x86_64-apple-darwin/dist/unsampled-piano" \
    "$target/aarch64-apple-darwin/dist/unsampled-piano"
sed "s/@VERSION@/$version/" crates/piano-play/macos/Info.plist > "$app/Contents/Info.plist"
cp -R presets "$app/Contents/Resources/"
cp LICENSE "$app/Contents/Resources/"
# Apple silicon runs only signed code: an ad-hoc signature, no identity.
codesign --force --sign - "$app"
echo "$app"
