#!/bin/bash
set -e

# Temp folder
mkdir icons

# Generate PNG icons from SVG
for size in 16 32 64 128 256 512; do
    rsvg-convert -w $size -h $size assets/logo.svg -o icons/logo_${size}.png
done

# Generate ICNS for macOS
mkdir -p icons/psst.iconset
for size in 16 32 64 128 256 512; do
    cp icons/logo_${size}.png icons/psst.iconset/icon_${size}x${size}.png
    cp icons/logo_${size}.png icons/psst.iconset/icon_$((size/2))x$((size/2))@2x.png
done
iconutil -c icns icons/psst.iconset -o assets/logo.icns

# Cleanup
rm -r icons