#!/bin/bash

set -eo pipefail

REPO_OWNER="jpochyla"
REPO_NAME="psst"

TAG_WITH_V="rolling" # Target the static rolling release tag

RELEASE_INFO_JSON=$(curl -sL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/tags/${TAG_WITH_V}")
: "${RELEASE_INFO_JSON:?Error: Could not fetch release info for tag ${TAG_WITH_V}.}"

# Extract the version from the release name, e.g., "Psst (rolling release - 2023.10.26-abcdefg)"
VERSION=$(echo "$RELEASE_INFO_JSON" | jq -r '.name' | sed -n 's/^Psst (rolling release - \(.*\))$/\1/p')
: "${VERSION:?Error: Could not extract version from release name for tag ${TAG_WITH_V}. Release name format might have changed.}"

DMG_URL=$(echo "$RELEASE_INFO_JSON" | jq -r '.assets[] | select(.name=="Psst.dmg") | .browser_download_url')
: "${DMG_URL:?Error: Could not find Psst.dmg asset URL for tag ${TAG_WITH_V}.}"

CHECKSUMS_URL=$(echo "$RELEASE_INFO_JSON" | jq -r '.assets[] | select(.name=="checksums.txt") | .browser_download_url')
: "${CHECKSUMS_URL:?Error: Could not find checksums.txt asset URL for tag ${TAG_WITH_V}.}"

# The checksums.txt contains paths like './Psst.dmg/Psst.dmg'
SHA256=$(curl -sL "$CHECKSUMS_URL" | grep -E '(\./)?Psst\.dmg/Psst\.dmg$' | awk '{print $1}')
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
    # For a rolling release, check the name of the 'rolling' tag's release page title
    url "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/tag/rolling"
    strategy :page_match
    # Extracts version like "2023.10.26-abcdefg" from release title "Psst (rolling release - 2023.10.26-abcdefg)"
    regex(/Psst\s+\(rolling release\s+-\s+([^)]+)\)/i)
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
