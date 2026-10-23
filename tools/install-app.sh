#!/bin/sh
# Builds the standalone player and installs it with its menu entry, icon
# and presets, for the current user (PREFIX=~/.local) or system-wide
# (PREFIX=/usr/local, as root):
#   $PREFIX/bin/unsampled-piano
#   $PREFIX/share/applications/unsampled-piano.desktop
#   $PREFIX/share/icons/hicolor/{scalable,256x256}/apps/unsampled-piano.*
#   $PREFIX/share/unsampled-piano/presets (shipped presets, replaced on
#   each install; the presets you save go to ~/.config/unsampled-piano)
set -e
cd "$(dirname "$0")/.."
prefix="${PREFIX:-$HOME/.local}"
cargo build --release -p piano-play
bin="${CARGO_TARGET_DIR:-target}/release/unsampled-piano"
share="$prefix/share"
mkdir -p "$prefix/bin" "$share/applications" "$share/icons/hicolor/scalable/apps" "$share/icons/hicolor/256x256/apps"
# Debug info stays in the build (profiling); the installed copy is stripped.
strip -o "$prefix/bin/unsampled-piano" "$bin"
# The menu entry runs the installed copy whatever the PATH.
sed "s|^Exec=unsampled-piano|Exec=$prefix/bin/unsampled-piano|" installer/linux/unsampled-piano.desktop \
    > "$share/applications/unsampled-piano.desktop"
cp assets/icon.svg "$share/icons/hicolor/scalable/apps/unsampled-piano.svg"
cp assets/icon-256.png "$share/icons/hicolor/256x256/apps/unsampled-piano.png"
rm -rf "$share/unsampled-piano/presets"
mkdir -p "$share/unsampled-piano"
cp -r presets "$share/unsampled-piano/presets"
# The menus pick the new entry up (when the tool exists).
command -v update-desktop-database >/dev/null && update-desktop-database -q "$share/applications" || true
echo "installed: $prefix/bin/unsampled-piano"
echo "           $share/applications/unsampled-piano.desktop"
echo "           $share/unsampled-piano/presets"
