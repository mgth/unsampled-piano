#!/usr/bin/env bash
# Renders assets/icon.svg into the platform icons, committed next to it:
# assets/icon-256.png (the player's window), assets/icon.ico (Windows: the
# executable and the installer) and assets/icon.icns (the macOS bundle).
# Needs rsvg-convert, ImageMagick and Python with Pillow.
set -euo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for s in 16 24 32 48 64 128 256 512 1024; do
    rsvg-convert -w $s -h $s assets/icon.svg -o "$tmp/$s.png"
done
cp "$tmp/256.png" assets/icon-256.png
magick "$tmp"/{16,24,32,48,64,128,256}.png assets/icon.ico
python3 -I - "$tmp" <<'PY'
import sys
from PIL import Image
d = sys.argv[1]
sizes = [16, 32, 64, 128, 256, 512]
Image.open(f"{d}/1024.png").save("assets/icon.icns", append_images=[Image.open(f"{d}/{s}.png") for s in sizes])
PY
echo "assets/icon.ico, assets/icon.icns"
