#!/bin/bash
set -e

# Temp folder
mkdir icons

# Generate PNG icons from SVG
for size in 16 32 48 64 128 256 512; do
	rsvg-convert -w $size -h $size assets/logo.svg -o icons/logo_${size}.png
	rsvg-convert -w $((size * 2)) -h $((size * 2)) assets/logo.svg -o icons/logo_${size}@2x.png
done

# Generate ICNS for macOS
mkdir -p icons/psst.iconset
for size in 16 32 48 64 128 256 512; do
	cp icons/logo_${size}.png icons/psst.iconset/icon_${size}x${size}.png
	cp icons/logo_${size}@2x.png icons/psst.iconset/icon_${size}x${size}@2x.png
done
iconutil -c icns icons/psst.iconset -o assets/logo.icns

# Cleanup
rm -r icons