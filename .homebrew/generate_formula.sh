#!/bin/bash

set -eo pipefail

REPO_OWNER="jpochyla"
REPO_NAME="psst"

# Get the latest release
RELEASE_INFO_JSON=$(curl -sL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest")
: "${RELEASE_INFO_JSON:?Error: Could not fetch latest release info.}"

# Find the latest Psst.dmg, get its version, URL, and SHA256
DATA=$(echo "$RELEASE_INFO_JSON" | jq -r '
  if .assets then
    .assets[]
    | select(.name | test("Psst-.*\\.dmg$"))
    | .name + " " + .browser_download_url + " " + .digest
  else
    empty
  end
' | sort -V | tail -n 1)

# Check if we got data
: "${DATA:?Error: Could not find a matching Psst.dmg asset.}"

# Extract variables from the data
DMG_NAME=$(echo "$DATA" | awk '{print $1}')
DMG_URL=$(echo "$DATA" | awk '{print $2}')
SHA256=$(echo "$DATA" | awk '{print $3}' | sed 's/sha256://')

VERSION=$(echo "$DMG_NAME" | sed -n 's/^Psst-\(.*\)\.dmg$/\1/p')
: "${VERSION:?Error: Could not extract version from DMG name.}"

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
    url "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest"
    strategy :github_latest
    regex(%r{href=.*?/Psst-([^/]+?)\.dmg}i)
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
