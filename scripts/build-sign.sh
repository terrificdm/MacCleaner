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

# Source paths end up in the binary (panic locations). Replace the builder's
# home and project folders so a shipped app doesn't reveal the user name.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME/.cargo=/cargo --remap-path-prefix=$HOME/.rustup=/rustup --remap-path-prefix=$PWD=/maccleaner --remap-path-prefix=$HOME=/home"

# Build, then sign ourselves (Tauri would otherwise ad-hoc sign).
npx tauri build --bundles app

codesign --force --deep --timestamp=none \
  --keychain "$KC" --sign "$NAME" "$APP_SRC"
codesign --verify --strict --verbose=2 "$APP_SRC"
