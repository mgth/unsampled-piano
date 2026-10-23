#!/bin/sh
# Installs Unsampled Piano from this folder (the Linux archive): the
# standalone player with its menu entry and icon, the CLAP and VST3 plugins,
# and the presets they share.
#
#   ./install.sh              for the current user: ~/.local, ~/.clap, ~/.vst3
#   sudo ./install.sh --system   for all users: /usr/local, /usr/lib/clap,
#                                /usr/lib/vst3
#   ./install.sh --uninstall  (with --system, or the same options as the
#                             installation) removes what it installed
#
# Packagers: --prefix /usr --clap-dir /usr/lib/clap --vst3-dir /usr/lib/vst3
# --destdir "$pkgdir". The presets you save go to ~/.config/unsampled-piano
# and are never touched.
set -e
here=$(cd "$(dirname "$0")" && pwd)

prefix="$HOME/.local"
clap="$HOME/.clap"
vst3="$HOME/.vst3"
destdir=""
uninstall=false

usage() {
    sed -n '2,/^set -e/p' "$0" | sed '$d; s/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
    case "$1" in
        --system) prefix=/usr/local clap=/usr/lib/clap vst3=/usr/lib/vst3 ;;
        --prefix) prefix="$2"; shift ;;
        --clap-dir) clap="$2"; shift ;;
        --vst3-dir) vst3="$2"; shift ;;
        --destdir) destdir="$2"; shift ;;
        --uninstall) uninstall=true ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
    shift
done

share="$prefix/share"
icons="$share/icons/hicolor"
files="$prefix/bin/unsampled-piano
$share/applications/unsampled-piano.desktop
$icons/scalable/apps/unsampled-piano.svg
$icons/256x256/apps/unsampled-piano.png
$share/unsampled-piano
$clap/unsampled-piano.clap
$vst3/Unsampled Piano.vst3"

# The menus and the icon theme pick the change up (installed system only).
refresh() {
    [ -z "$destdir" ] || return 0
    if command -v update-desktop-database >/dev/null; then update-desktop-database -q "$share/applications" || true; fi
    if command -v gtk-update-icon-cache >/dev/null; then gtk-update-icon-cache -q -t "$icons" 2>/dev/null || true; fi
}

if $uninstall; then
    echo "$files" | while IFS= read -r f; do
        if [ -e "$destdir$f" ]; then rm -rf "$destdir$f" && echo "removed: $f"; fi
    done
    refresh
    exit 0
fi

mkdir -p "$destdir$prefix/bin" "$destdir$share/applications" "$destdir$icons/scalable/apps" \
    "$destdir$icons/256x256/apps" "$destdir$share/unsampled-piano" "$destdir$clap" "$destdir$vst3"
install -m 755 "$here/bin/unsampled-piano" "$destdir$prefix/bin/unsampled-piano"
# The menu entry runs the installed copy whatever the PATH.
sed "s|^Exec=unsampled-piano|Exec=$prefix/bin/unsampled-piano|" "$here/share/applications/unsampled-piano.desktop" \
    > "$destdir$share/applications/unsampled-piano.desktop"
install -m 644 "$here/share/icons/unsampled-piano.svg" "$destdir$icons/scalable/apps/unsampled-piano.svg"
install -m 644 "$here/share/icons/unsampled-piano.png" "$destdir$icons/256x256/apps/unsampled-piano.png"
# Shipped presets: replaced on each installation.
rm -rf "$destdir$share/unsampled-piano/presets"
cp -R "$here/presets" "$destdir$share/unsampled-piano/presets"
install -m 755 "$here/plugins/unsampled-piano.clap" "$destdir$clap/unsampled-piano.clap"
rm -rf "$destdir$vst3/Unsampled Piano.vst3"
cp -R "$here/plugins/Unsampled Piano.vst3" "$destdir$vst3/"
refresh

echo "installed: $prefix/bin/unsampled-piano (menu: Unsampled Piano)"
echo "           $clap/unsampled-piano.clap"
echo "           $vst3/Unsampled Piano.vst3"
echo "           $share/unsampled-piano/presets"
