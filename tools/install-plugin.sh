#!/bin/sh
# Builds the plugin and installs it for the current user:
#   ~/.clap/physical-piano.clap and ~/.vst3/Physical Piano.vst3
set -e
cd "$(dirname "$0")/.."
cargo build --release -p piano-plugin
so=target/release/libpiano_plugin.so
mkdir -p "$HOME/.clap" "$HOME/.vst3/Physical Piano.vst3/Contents/x86_64-linux"
# Debug info stays in target/ (profiling); the installed copy is stripped.
strip -o "$HOME/.clap/physical-piano.clap" "$so"
cp "$HOME/.clap/physical-piano.clap" "$HOME/.vst3/Physical Piano.vst3/Contents/x86_64-linux/Physical Piano.so"
echo "installed: $HOME/.clap/physical-piano.clap"
echo "           $HOME/.vst3/Physical Piano.vst3"
