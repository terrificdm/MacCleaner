#!/bin/bash
# Builds and signs MacCleaner.app, then installs it to
# /Applications/MacCleaner.app. Run scripts/setup-signing.sh once first.
set -euo pipefail
cd "$(dirname "$0")/.."

APP_SRC="src-tauri/target/release/bundle/macos/MacCleaner.app"
DEST="/Applications/MacCleaner.app"

./scripts/build-sign.sh

# The executable is named "maccleaner". SIGTERM lets it exit normally.
if pgrep -x maccleaner >/dev/null; then
  pkill -TERM -x maccleaner || true
  for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x maccleaner >/dev/null || break; sleep 0.5; done
fi
rm -rf "$DEST.new"
ditto "$APP_SRC" "$DEST.new"
rm -rf "$DEST"
mv "$DEST.new" "$DEST"
echo "Installed $DEST"
codesign -dv "$DEST" 2>&1 | grep -E "Identifier|Authority|TeamIdentifier"
