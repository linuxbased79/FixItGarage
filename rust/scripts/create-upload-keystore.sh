#!/usr/bin/env bash
# Create a Play Console *upload* keystore (NOT committed). Keep this file safe.
#
# Usage:
#   ./scripts/create-upload-keystore.sh [out.jks] [alias]
#
# Then sign release APKs:
#   export FIG_KEYSTORE=/path/to/fixitgarage-upload.jks
#   export FIG_KEYSTORE_PASS='...'
#   export FIG_KEY_ALIAS=upload
#   export FIG_KEY_PASS='...'
#   ./scripts/release-apks.sh
set -euo pipefail
OUT="${1:-$HOME/fixitgarage-upload.jks}"
ALIAS="${2:-upload}"
if [[ -f "$OUT" ]]; then
  echo "Already exists: $OUT" >&2
  exit 1
fi
echo "Creating upload keystore at $OUT (alias=$ALIAS)"
echo "You will be prompted for a keystore password — store it in a password manager."
keytool -genkeypair -v \
  -keystore "$OUT" \
  -alias "$ALIAS" \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000 \
  -dname "CN=FixItGarage Upload,O=linuxbased79,C=US"
echo
echo "Next:"
echo "  export FIG_KEYSTORE=$OUT"
echo "  export FIG_KEYSTORE_PASS='(your password)'"
echo "  export FIG_KEY_ALIAS=$ALIAS"
echo "  export FIG_KEY_PASS='(your password)'"
echo "  cd $(dirname "$0")/.. && ./scripts/release-apks.sh"
echo
echo "In Play Console: use Play App Signing; upload APKs signed with this key."
echo "Do NOT commit $OUT or passwords to git."
