#!/bin/bash
# Builds and signs MacCleaner.app, then packages it as
# release/MacCleaner_<version>_aarch64.dmg for installing on other Macs.
# The dmg contains the app, an /Applications shortcut and both READMEs.
set -euo pipefail
cd "$(dirname "$0")/.."

APP_SRC="src-tauri/target/release/bundle/macos/MacCleaner.app"
VERSION=$(python3 -c 'import json;print(json.load(open("src-tauri/tauri.conf.json"))["version"])')
OUT="release/MacCleaner_${VERSION}_aarch64.dmg"

if [ "${1:-}" != "--no-build" ]; then
  ./scripts/build-sign.sh
fi
[ -d "$APP_SRC" ] || { echo "Missing $APP_SRC; run without --no-build"; exit 1; }

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
ditto "$APP_SRC" "$STAGE/MacCleaner.app"
ln -s /Applications "$STAGE/Applications"
cp README.md README.zh-CN.md "$STAGE/"

mkdir -p release
rm -f "$OUT"
hdiutil create -quiet -volname "MacCleaner $VERSION" -srcfolder "$STAGE" \
  -fs HFS+ -format UDZO -ov "$OUT"
hdiutil verify -quiet "$OUT"

echo "Created $OUT ($(du -h "$OUT" | cut -f1))"
shasum -a 256 "$OUT"
