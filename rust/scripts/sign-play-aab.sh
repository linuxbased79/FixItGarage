#!/usr/bin/env bash
# Sign an existing Play AAB with the upload keystore.
# Usage:
#   ./scripts/sign-play-aab.sh [path-to.aab]
# Default: dist/FixItGarage-0.2.35-play.aab (or latest in dist/)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

KEYSTORE="${FIG_KEYSTORE:-$HOME/fixitgarage-upload.jks}"
ALIAS="${FIG_KEY_ALIAS:-upload}"
AAB="${1:-}"

if [[ -z "$AAB" ]]; then
  if [[ -f dist/FixItGarage-0.2.35-play.aab ]]; then
    AAB="dist/FixItGarage-0.2.35-play.aab"
  else
    AAB="$(ls -t dist/FixItGarage-*-play.aab 2>/dev/null | head -1 || true)"
  fi
fi

if [[ -z "$AAB" || ! -f "$AAB" ]]; then
  echo "No AAB found. Pass path: ./scripts/sign-play-aab.sh path/to/file.aab" >&2
  exit 1
fi
if [[ ! -f "$KEYSTORE" ]]; then
  echo "Missing keystore: $KEYSTORE" >&2
  exit 1
fi

JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk-amd64}"
JARSIGNER="$JAVA_HOME/bin/jarsigner"
[[ -x "$JARSIGNER" ]] || JARSIGNER="$(command -v jarsigner)"

echo "AAB:      $AAB"
echo "Keystore: $KEYSTORE"
echo "Alias:    $ALIAS"
echo

if [[ -z "${FIG_KEYSTORE_PASS:-}" ]]; then
  read -r -s -p "Keystore password: " FIG_KEYSTORE_PASS
  echo
fi
KEY_PASS="${FIG_KEY_PASS:-$FIG_KEYSTORE_PASS}"

# Work on a copy so a bad password cannot leave a half-signed mess
SIGNED="${AAB%.aab}-signed.aab"
cp -f "$AAB" "$SIGNED"

echo "Signing…"
if ! "$JARSIGNER" -sigalg SHA256withRSA -digestalg SHA-256 \
  -keystore "$KEYSTORE" \
  -storepass "$FIG_KEYSTORE_PASS" \
  -keypass "$KEY_PASS" \
  "$SIGNED" "$ALIAS"
then
  rm -f "$SIGNED"
  echo "Signing FAILED (wrong password or keystore issue)." >&2
  exit 1
fi

echo "Verifying…"
if ! "$JARSIGNER" -verify -certs "$SIGNED" 2>&1 | tee /tmp/fig-aab-verify.txt | grep -q "jar verified"; then
  # jarsigner sometimes says "jar verified." with period
  if ! grep -qiE 'jar verified|s = signature was verified' /tmp/fig-aab-verify.txt; then
    echo "Verify failed:" >&2
    cat /tmp/fig-aab-verify.txt >&2
    exit 1
  fi
fi

# Replace original + copy to Downloads
mv -f "$SIGNED" "$AAB"
BASENAME="$(basename "$AAB")"
cp -f "$AAB" "$HOME/Downloads/$BASENAME"
chown "$(stat -c '%U:%G' "$HOME" 2>/dev/null || echo christopher:christopher)" \
  "$HOME/Downloads/$BASENAME" 2>/dev/null || true

echo
echo "Signed OK:"
ls -lh "$AAB"
ls -lh "$HOME/Downloads/$BASENAME"
echo
echo "Upload THIS file in Play Console:"
echo "  ~/Downloads/$BASENAME"
"$JARSIGNER" -verify -verbose -certs "$AAB" 2>&1 | grep -E 'CN=|jar verified|signed by' | head -10
