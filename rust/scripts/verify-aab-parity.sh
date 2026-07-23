#!/usr/bin/env bash
# Post-build gate for Play AABs (Lumo 0.2.39 crash consult).
# Fails if Java bridges or native lib are missing — the 0.2.39 Play regression.
#
# Usage:
#   ./scripts/verify-aab-parity.sh path/to/app.aab
#   ./scripts/verify-aab-parity.sh path/to/app.aab path/to/sideload.apk   # optional dex diff
set -euo pipefail

AAB="${1:?usage: verify-aab-parity.sh app.aab [sideload.apk]}"
SIDELOAD_APK="${2:-}"

if [[ ! -f "$AAB" ]]; then
  echo "ERROR: AAB not found: $AAB" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "=== Verify AAB: $AAB ==="

# AAB layout: base/dex/classes.dex , base/lib/<abi>/lib*.so
unzip -q -o "$AAB" -d "$TMP/aab"

DEX=""
if [[ -f "$TMP/aab/base/dex/classes.dex" ]]; then
  DEX="$TMP/aab/base/dex/classes.dex"
elif [[ -f "$TMP/aab/classes.dex" ]]; then
  DEX="$TMP/aab/classes.dex"
else
  # Search
  DEX="$(find "$TMP/aab" -name 'classes.dex' | head -1 || true)"
fi

if [[ -z "$DEX" || ! -f "$DEX" ]]; then
  echo "ERROR: no classes.dex inside AAB" >&2
  exit 1
fi

DEX_SIZE="$(wc -c < "$DEX" | tr -d ' ')"
echo "classes.dex size: $DEX_SIZE bytes"
if [[ "$DEX_SIZE" -lt 5000 ]]; then
  echo "ERROR: classes.dex is suspiciously small ($DEX_SIZE) — likely R-only (0.2.39 bug)" >&2
  exit 1
fi

# Exact class names as packaged (not Lumo's shortened ShareReceiver)
REQUIRED=(
  "StorageHelper"
  "BootReceiver"
  "ShareReceiveActivity"
)

FAIL=0
for cls in "${REQUIRED[@]}"; do
  if strings "$DEX" | grep -q "$cls"; then
    echo "  OK  $cls"
  else
    echo "  FAIL $cls missing from AAB dex" >&2
    FAIL=1
  fi
done

# Native lib (arm64 is required for phones)
if find "$TMP/aab" -path '*/lib/arm64-v8a/libfixitgarage_ui.so' | grep -q .; then
  SO="$(find "$TMP/aab" -path '*/lib/arm64-v8a/libfixitgarage_ui.so' | head -1)"
  echo "  OK  lib/arm64-v8a/libfixitgarage_ui.so ($(du -h "$SO" | cut -f1))"
else
  echo "  FAIL lib/arm64-v8a/libfixitgarage_ui.so missing" >&2
  FAIL=1
fi

# Brand icon asset present
if find "$TMP/aab" -iname '*ic_launcher*' | grep -q .; then
  echo "  OK  launcher icon resources present"
else
  echo "  FAIL no ic_launcher resources in AAB" >&2
  FAIL=1
fi

# Optional: compare required class presence with sideload APK
if [[ -n "$SIDELOAD_APK" && -f "$SIDELOAD_APK" ]]; then
  echo "=== Dex parity vs sideload APK ==="
  unzip -q -o "$SIDELOAD_APK" classes.dex -d "$TMP/apk" 2>/dev/null || true
  if [[ -f "$TMP/apk/classes.dex" ]]; then
    for cls in "${REQUIRED[@]}"; do
      IN_AAB=0
      IN_APK=0
      strings "$DEX" | grep -q "$cls" && IN_AAB=1
      strings "$TMP/apk/classes.dex" | grep -q "$cls" && IN_APK=1
      if [[ "$IN_AAB" -eq 1 && "$IN_APK" -eq 1 ]]; then
        echo "  OK  $cls in both"
      else
        echo "  FAIL $cls  aab=$IN_AAB apk=$IN_APK" >&2
        FAIL=1
      fi
    done
  else
    echo "  WARN could not extract classes.dex from sideload APK"
  fi
fi

if [[ "$FAIL" -ne 0 ]]; then
  echo "=== AAB parity check FAILED — do not upload to Play ===" >&2
  exit 1
fi

echo "=== AAB parity check PASSED ==="
exit 0
