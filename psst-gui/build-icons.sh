#!/bin/bash
set -e

# Generate PNG icons from SVG
for size in 16 32 64 128 256 512; do
    rsvg-convert -w $size -h $size assets/logo.svg -o icons/logo_${size}.png
done

# Generate ICO for Windows
magick icons/logo_*.png icons/psst.ico

# Generate ICNS for macOS
mkdir -p icons/psst.iconset
for size in 16 32 64 128 256 512; do
    cp icons/logo_${size}.png icons/psst.iconset/icon_${size}x${size}.png
    cp icons/logo_${size}.png icons/psst.iconset/icon_$((size/2))x$((size/2))@2x.png
done
iconutil -c icns icons/psst.iconset -o icons/psst.icns
rm -r icons/psst.iconset
