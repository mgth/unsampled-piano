#!/bin/sh
# Builds the plugin and installs it for the current user:
#   ~/.clap/unsampled-piano.clap and ~/.vst3/Unsampled Piano.vst3
set -e
cd "$(dirname "$0")/.."
cargo build --release -p piano-plugin
so=target/release/libpiano_plugin.so
mkdir -p "$HOME/.clap" "$HOME/.vst3/Unsampled Piano.vst3/Contents/x86_64-linux"
# Debug info stays in target/ (profiling); the installed copy is stripped.
strip -o "$HOME/.clap/unsampled-piano.clap" "$so"
cp "$HOME/.clap/unsampled-piano.clap" "$HOME/.vst3/Unsampled Piano.vst3/Contents/x86_64-linux/Unsampled Piano.so"
echo "installed: $HOME/.clap/unsampled-piano.clap"
echo "           $HOME/.vst3/Unsampled Piano.vst3"
