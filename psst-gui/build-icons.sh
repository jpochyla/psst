#!/bin/bash
set -euo pipefail

# Check for required tools
command -v rsvg-convert >/dev/null 2>&1 || {
	echo >&2 "rsvg-convert is required but not installed. Aborting."
	exit 1
}
command -v iconutil >/dev/null 2>&1 || {
	echo >&2 "iconutil is required but not installed. Aborting."
	exit 1
}
command -v pngquant >/dev/null 2>&1 || {
	echo >&2 "pngquant is required but not installed. Aborting."
	exit 1
}
command -v optipng >/dev/null 2>&1 || {
	echo >&2 "optipng is required but not installed. Aborting."
	exit 1
}

# Temp folder
ICON_DIR="icons"
mkdir -p "$ICON_DIR"

# Generate PNG icons from SVG
SIZES=(16 32 64 128 256 512)
for size in "${SIZES[@]}"; do
	rsvg-convert -w $size -h $size assets/logo.svg -o "$ICON_DIR/logo_${size}.png"

	# Apply lossy compression with pngquant
	pngquant --force --quality=60-80 "$ICON_DIR/logo_${size}.png" --output "$ICON_DIR/logo_${size}.png"

	# Further optimize with optipng
	optipng -quiet -o5 "$ICON_DIR/logo_${size}.png"

	# For smaller sizes, reduce color depth
	if [ $size -le 32 ]; then
		magick "$ICON_DIR/logo_${size}.png" -colors 256 PNG8:"$ICON_DIR/logo_${size}.png"
	fi
done

# Generate ICNS for macOS
ICONSET_DIR="$ICON_DIR/psst.iconset"
mkdir -p "$ICONSET_DIR"
for size in "${SIZES[@]}"; do
	cp "$ICON_DIR/logo_${size}.png" "$ICONSET_DIR/icon_${size}x${size}.png"
	if [ $size -ne 16 ] && [ $size -ne 32 ]; then
		cp "$ICON_DIR/logo_${size}.png" "$ICONSET_DIR/icon_$((size / 2))x$((size / 2))@2x.png"
	fi
done

# Create ICNS file
iconutil -c icns "$ICONSET_DIR" -o assets/logo.icns

# Cleanup
rm -r "$ICON_DIR"

echo "Icon generation complete. ICNS file size: $(du -h assets/logo.icns | cut -f1)"
