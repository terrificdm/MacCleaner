#!/bin/bash
# Builds MacCleaner.app (Apple Silicon) and signs it with the stable local
# identity. Used by build-install.sh and package-dmg.sh.
# Run scripts/setup-signing.sh once first.
set -euo pipefail
cd "$(dirname "$0")/.."

NAME="MacCleaner Local Signing"
KC="$HOME/Library/Keychains/maccleaner-signing.keychain-db"
SVC="maccleaner-signing-keychain"
APP_SRC="src-tauri/target/release/bundle/macos/MacCleaner.app"

[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

PASS=$(security find-generic-password -a "$USER" -s "$SVC" -w)
security unlock-keychain -p "$PASS" "$KC"

# Build, then sign ourselves (Tauri would otherwise ad-hoc sign).
npx tauri build --bundles app

codesign --force --deep --timestamp=none \
  --keychain "$KC" --sign "$NAME" "$APP_SRC"
codesign --verify --strict --verbose=2 "$APP_SRC"
