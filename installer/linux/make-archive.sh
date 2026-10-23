#!/bin/sh
# The Linux archive: builds the standalone player and the plugins (profile
# dist) and gathers them with the presets, the menu entry, the icon and
# install.sh in target/linux/unsampled-piano-<version>-linux-<arch>/, then
# packs that folder as a .tar.gz next to it.
#   --no-build  use what is already built (packaging, e.g. the PKGBUILDs)
#   --no-tar    only the folder
# Extra arguments go to cargo build (e.g. --frozen). Prints the folder.
set -e
cd "$(dirname "$0")/../.."

build=true
tar=true
while [ $# -gt 0 ]; do
    case "$1" in
        --no-build) build=false ;;
        --no-tar) tar=false ;;
        *) break ;;
    esac
    shift
done

target="${CARGO_TARGET_DIR:-target}"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
name="unsampled-piano-$version-linux-$(uname -m)"
out="$target/linux/$name"

if $build; then
    cargo build --profile dist -p piano-play -p piano-plugin "$@"
fi

rm -rf "$out"
mkdir -p "$out/bin" "$out/plugins" "$out/share/applications" "$out/share/icons"
cp "$target/dist/unsampled-piano" "$out/bin/"
# The plugin library is both the CLAP file and the VST3 bundle's binary.
so="$target/dist/libpiano_plugin.so"
vst3="$out/plugins/Unsampled Piano.vst3/Contents/$(uname -m)-linux"
mkdir -p "$vst3"
cp "$so" "$out/plugins/unsampled-piano.clap"
cp "$so" "$vst3/Unsampled Piano.so"
cp -R presets "$out/"
cp installer/linux/unsampled-piano.desktop "$out/share/applications/"
cp assets/icon.svg "$out/share/icons/unsampled-piano.svg"
cp assets/icon-256.png "$out/share/icons/unsampled-piano.png"
cp installer/linux/install.sh LICENSE README.md "$out/"
chmod 755 "$out/install.sh"

if $tar; then
    tar -C "$target/linux" -czf "$target/linux/$name.tar.gz" "$name"
fi
echo "$out"
