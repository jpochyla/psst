#!/bin/bash
# Script to generate Homebrew formula information for Psst

# Get latest release info
VERSION=$(curl -s "https://api.github.com/repos/jpochyla/psst/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
DOWNLOAD_URL="https://github.com/jpochyla/psst/releases/download/v${VERSION}/Psst-${VERSION}.dmg"
SHA256=$(curl -sL "https://github.com/jpochyla/psst/releases/download/v${VERSION}/Psst-${VERSION}.dmg.sha256" | awk '{print $1}')

echo "Psst Homebrew Formula Information"
echo "================================="
echo "Version: ${VERSION}"
echo "Download URL: ${DOWNLOAD_URL}"
echo "SHA256: ${SHA256}"
echo
echo "Homebrew Cask Formula:"
echo "----------------------"

cat << EOF
cask "psst" do
  version "${VERSION}"
  sha256 "${SHA256}"

  url "https://github.com/jpochyla/psst/releases/download/v#{version}/Psst-#{version}.dmg"
  name "Psst"
  desc "Fast and multi-platform Spotify client"
  homepage "https://github.com/jpochyla/psst"

  app "Psst.app"

  depends_on macos: ">= 11.0"

  zap trash: [
    "~/Library/Application Support/com.jpochyla.psst",
    "~/Library/Caches/com.jpochyla.psst",
    "~/Library/Preferences/com.jpochyla.psst.plist"
  ]
end
EOF