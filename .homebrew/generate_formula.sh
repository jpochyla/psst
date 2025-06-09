#!/bin/bash

set -eo pipefail

REPO_OWNER="jpochyla"
REPO_NAME="psst"

TAG_WITH_V="rolling" # Target the static rolling release tag

RELEASE_INFO_JSON=$(curl -sL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/tags/${TAG_WITH_V}")
: "${RELEASE_INFO_JSON:?Error: Could not fetch release info for tag ${TAG_WITH_V}.}"

# Extract the version from the release name, e.g., "Continuous release (2023.10.26-abcdefg)"
VERSION=$(echo "$RELEASE_INFO_JSON" | jq -r '.name' | sed -n 's/^Continuous release (\(.*\))$/\1/p')
: "${VERSION:?Error: Could not extract version from release name for tag ${TAG_WITH_V}. Release name format might have changed.}"

DMG_ASSET_JSON=$(echo "$RELEASE_INFO_JSON" | jq -r '.assets[] | select(.name=="Psst.dmg")')
: "${DMG_ASSET_JSON:?Error: Could not find Psst.dmg asset for tag ${TAG_WITH_V}.}"

DMG_URL=$(echo "$DMG_ASSET_JSON" | jq -r '.browser_download_url')
: "${DMG_URL:?Error: Could not find Psst.dmg asset URL for tag ${TAG_WITH_V}.}"

SHA256=$(echo "$DMG_ASSET_JSON" | jq -r '.digest' | sed 's/sha256://')
: "${SHA256:?Error: Could not find SHA256 for Psst.dmg in release info.}"

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
    # For a rolling release, check the name of the 'rolling' tag's release page title
    url "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/tag/rolling"
    strategy :page_match
    # Extracts version like "2023.10.26-abcdefg" from release title "Continuous release (2023.10.26-abcdefg)"
    regex(/Continuous release\s+\(([^)]+)\)/i)
  end

  app "Psst.app"

  depends_on macos: ">= :big_sur"

  zap trash: [
    "~/Library/Application Support/Psst",
    "~/Library/Caches/Psst",
    "~/Library/Caches/com.jpochyla.psst",
    "~/Library/HTTPStorages/com.jpochyla.psst",
    "~/Library/Preferences/com.jpochyla.psst.plist",
    "~/Library/Saved Application State/com.jpochyla.psst.savedState",
  ]
end
EOF
