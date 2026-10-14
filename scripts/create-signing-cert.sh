#!/usr/bin/env bash
# Create a self-signed code signing certificate in the login keychain.
#
# scripts/build-local.sh signs with it when present. Unlike ad-hoc signing,
# a stable certificate lets macOS keep the Accessibility permission across
# rebuilds. Run once per machine.

set -euo pipefail

NAME="Tab Local Code Signing"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
# Built-in LibreSSL writes PKCS#12 files the macOS keychain can import
OPENSSL=/usr/bin/openssl

if security find-identity -p codesigning "$KEYCHAIN" | grep -qF "\"$NAME\""; then
  echo "\"$NAME\" already exists in the login keychain"
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cat > "$tmp/cert.conf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
EOF

"$OPENSSL" req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -config "$tmp/cert.conf" -keyout "$tmp/key.pem" -out "$tmp/cert.pem" 2>/dev/null

# The password only protects the temporary file during import
pass="$(uuidgen)"
"$OPENSSL" pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" \
  -name "$NAME" -passout "pass:$pass" -out "$tmp/cert.p12"

# -T lets codesign use the private key without a prompt
security import "$tmp/cert.p12" -k "$KEYCHAIN" -P "$pass" -T /usr/bin/codesign

echo "Created \"$NAME\" in the login keychain"
