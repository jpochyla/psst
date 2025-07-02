#!/bin/bash

set -eo pipefail

REPO_OWNER="jpochyla"
REPO_NAME="psst"

cat <<EOF
cask "psst" do
  version :latest
  sha256 :no_check

  url "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest/download/Psst.dmg"
  name "Psst"
  desc "Fast and native Spotify client"
  homepage "https://github.com/${REPO_OWNER}/${REPO_NAME}/"

  depends_on macos: ">= :big_sur"

  app "Psst.app"

  zap trash: [
    "~/Library/Application Support/Psst",
    "~/Library/Caches/com.jpochyla.psst",
    "~/Library/Caches/Psst",
    "~/Library/HTTPStorages/com.jpochyla.psst",
    "~/Library/Preferences/com.jpochyla.psst.plist",
    "~/Library/Saved Application State/com.jpochyla.psst.savedState",
  ]
end
EOF
