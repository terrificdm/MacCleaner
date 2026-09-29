#!/bin/bash
# Creates a self-signed code-signing identity "MacCleaner Local Signing" in a
# dedicated keychain (~/Library/Keychains/maccleaner-signing.keychain-db).
# A stable signing identity keeps Full Disk Access / Automation grants valid
# across rebuilds. The keychain password is random and stored only in your
# login keychain; nothing is written to the repository. Safe to run again.
set -euo pipefail

NAME="MacCleaner Local Signing"
KC="$HOME/Library/Keychains/maccleaner-signing.keychain-db"
SVC="maccleaner-signing-keychain"
# macOS's own LibreSSL writes PKCS#12 files that `security import` accepts.
OPENSSL=/usr/bin/openssl

has_identity() {
  # No -v: a self-signed certificate is "not trusted" and -v would hide it.
  security find-identity -p codesigning "$KC" 2>/dev/null | grep -q "\"$NAME\""
}

if [ -f "$KC" ]; then
  if ! PASS=$(security find-generic-password -a "$USER" -s "$SVC" -w 2>/dev/null); then
    echo "Keychain $KC exists but its password is not in the login keychain."
    echo "Delete it (security delete-keychain \"$KC\") and run this script again."
    exit 1
  fi
  security unlock-keychain -p "$PASS" "$KC"
  if has_identity; then
    echo "Identity \"$NAME\" already exists in $KC"
    exit 0
  fi
else
  PASS=$($OPENSSL rand -hex 24)
  security add-generic-password -U -a "$USER" -s "$SVC" -w "$PASS" >/dev/null
  security create-keychain -p "$PASS" "$KC"
fi
security set-keychain-settings "$KC"            # no auto-lock timeout
security unlock-keychain -p "$PASS" "$KC"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cat > "$TMP/cfg" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
EOF
$OPENSSL req -x509 -newkey rsa:2048 -nodes -days 3650 -config "$TMP/cfg" \
  -keyout "$TMP/key.pem" -out "$TMP/cert.pem" 2>/dev/null
P12PASS=$($OPENSSL rand -hex 16)
$OPENSSL pkcs12 -export -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
  -name "$NAME" -out "$TMP/id.p12" -passout "pass:$P12PASS"

security import "$TMP/id.p12" -k "$KC" -P "$P12PASS" -T /usr/bin/codesign >/dev/null
security set-key-partition-list -S apple-tool:,apple: -s -k "$PASS" "$KC" >/dev/null

# Add to the search list (keep existing keychains).
EXISTING=$(security list-keychains -d user | sed 's/[" ]//g')
echo "$EXISTING" | grep -qx "$KC" || security list-keychains -d user -s $EXISTING "$KC"

has_identity || { echo "Identity was not created"; exit 1; }
echo "Created \"$NAME\" in $KC (self-signed certificates show as not trusted; codesign still uses them)."
