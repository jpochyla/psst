#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
cargo build --locked --release --bin psst-gui
output_root="$project_root/dist"
mkdir -p "$output_root"
stage_root="$(mktemp -d "$output_root/package-XXXXXXXX")"
case "$stage_root" in "$output_root"/package-*) ;; *) exit 1 ;; esac
trap 'rm -rf -- "$stage_root"' EXIT
case "$(uname -s)" in
  Darwin)
    app_root="$stage_root/Xpotify.app/Contents"
    mkdir -p "$app_root/MacOS" "$app_root/Resources" "$stage_root/Xpotify.iconset"
    cp target/release/psst-gui "$app_root/MacOS/Xpotify"
    for size in 16 32 128 256 512; do
      sips -z "$size" "$size" psst-gui/assets/logo_512.png --out "$stage_root/Xpotify.iconset/icon_${size}x${size}.png" >/dev/null
      double=$((size * 2))
      sips -z "$double" "$double" psst-gui/assets/logo_512.png --out "$stage_root/Xpotify.iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns "$stage_root/Xpotify.iconset" -o "$app_root/Resources/Xpotify.icns"
    cat > "$app_root/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Xpotify</string>
<key>CFBundleIdentifier</key><string>com.xpotify.desktop</string>
<key>CFBundleExecutable</key><string>Xpotify</string>
<key>CFBundleIconFile</key><string>Xpotify.icns</string>
<key>CFBundleVersion</key><string>0.4.2</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
    cp LICENSE.md LICENSE-librespot.md LICENSE-Spotifast.md "$app_root/Resources/"
    ditto -c -k --sequesterRsrc --keepParent "$stage_root/Xpotify.app" "$output_root/Xpotify-macOS-$(uname -m).zip"
    ;;
  Linux)
    mkdir -p "$stage_root/Xpotify/bin" "$stage_root/Xpotify/share/applications" "$stage_root/Xpotify/share/icons/hicolor/256x256/apps"
    cp target/release/psst-gui "$stage_root/Xpotify/bin/xpotify"
    cp psst-gui/assets/logo_256.png "$stage_root/Xpotify/share/icons/hicolor/256x256/apps/xpotify.png"
    cp LICENSE.md LICENSE-librespot.md LICENSE-Spotifast.md SPLITIFY.md ROADMAP.md "$stage_root/Xpotify/"
    cat > "$stage_root/Xpotify/Start-Xpotify.sh" <<'LAUNCHER'
#!/usr/bin/env sh
set -eu
cd -- "$(dirname -- "$0")"
exec ./bin/xpotify
LAUNCHER
    chmod +x "$stage_root/Xpotify/Start-Xpotify.sh"
    cat > "$stage_root/Xpotify/share/applications/xpotify.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Xpotify
Comment=Spotify and Splitify
Exec=xpotify
Icon=xpotify
Terminal=false
Categories=AudioVideo;Audio;Player;
DESKTOP
    tar -czf "$output_root/Xpotify-Linux-$(uname -m).tar.gz" -C "$stage_root" Xpotify
    ;;
  *) echo 'Use Package-Native.ps1 on Windows.' >&2; exit 1 ;;
esac
