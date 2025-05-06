#!/bin/bash

set -eo pipefail

REPO_OWNER="jpochyla"
REPO_NAME="psst"

LATEST_VERSION_TAG_NO_V=$(git ls-remote --tags "https://github.com/${REPO_OWNER}/${REPO_NAME}.git" |
	grep -Eo 'refs/tags/v[0-9]{4}\.[0-9]{2}\.[0-9]{2}-[a-f0-9]{7}$' |
	sed 's|refs/tags/v||' | sort -V | tail -n1)
: "${LATEST_VERSION_TAG_NO_V:?Error: No versioned tag found.}"

VERSION="$LATEST_VERSION_TAG_NO_V"
TAG_WITH_V="v${VERSION}"

RELEASE_INFO_JSON=$(curl -sL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/tags/${TAG_WITH_V}")

DMG_URL=$(echo "$RELEASE_INFO_JSON" | jq -r '.assets[] | select(.name=="Psst.dmg") | .browser_download_url')
: "${DMG_URL:?Error: Could not find Psst.dmg asset URL for tag ${TAG_WITH_V}.}"

CHECKSUMS_URL=$(echo "$RELEASE_INFO_JSON" | jq -r '.assets[] | select(.name=="checksums.txt") | .browser_download_url')
: "${CHECKSUMS_URL:?Error: Could not find checksums.txt asset URL for tag ${TAG_WITH_V}.}"

SHA256=$(curl -sL "$CHECKSUMS_URL" | grep '\./Psst.dmg/Psst.dmg$' | awk '{print $1}')
: "${SHA256:?Error: Could not find SHA256 for Psst.dmg in checksums.txt.}"

cat <<EOF
cask "psst" do
  version "${VERSION}"
  sha256 "${SHA256}"

  url "${DMG_URL}",
      verified: "github.com/${REPO_OWNER}/${REPO_NAME}/"
  name "Psst"
  desc "Fast and native Spotify client"
  homepage "https://github.com/${REPO_OWNER}/${REPO_NAME}/"

  livecheck do
    url "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases"
    strategy :github_latest
    regex(/^v?(\d{4}\.\d{2}\.\d{2}-[0-9a-f]{7})$/i)
  end

  app "Psst.app"

  depends_on macos: ">= :big_sur"

  zap trash: [
    "~/Library/Application Support/com.jpochyla.psst",
    "~/Library/Caches/com.jpochyla.psst",
    "~/Library/HTTPStorages/com.jpochyla.psst",
    "~/Library/Preferences/com.jpochyla.psst.plist",
    "~/Library/Saved Application State/com.jpochyla.psst.savedState",
  ]
end
EOF
