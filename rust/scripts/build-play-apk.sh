#!/usr/bin/env bash
# Build a Play Console upload APK signed with your upload keystore.
# Usage: ./scripts/build-play-apk.sh [versionName] [versionCode]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION_NAME="${1:-0.2.33}"
VERSION_CODE="${2:-2033}"
KEYSTORE="${FIG_KEYSTORE:-$HOME/fixitgarage-upload.jks}"
ALIAS="${FIG_KEY_ALIAS:-upload}"

if [[ ! -f "$KEYSTORE" ]]; then
  echo "Missing keystore: $KEYSTORE" >&2
  exit 1
fi

echo "Keystore: $KEYSTORE"
echo "Alias:    $ALIAS"
echo "Version:  $VERSION_NAME (code $VERSION_CODE)"
echo
if [[ -z "${FIG_KEYSTORE_PASS:-}" ]]; then
  read -r -s -p "Keystore password: " FIG_KEYSTORE_PASS
  echo
fi
export FIG_KEYSTORE="$KEYSTORE"
export FIG_KEYSTORE_PASS
export FIG_KEY_ALIAS="$ALIAS"
export FIG_KEY_PASS="${FIG_KEY_PASS:-$FIG_KEYSTORE_PASS}"

# Rust toolchain (user install) + xbuild (may live under /root/.cargo/bin on this machine)
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
if [[ ! -d "$ANDROID_HOME/platform-tools" && -d /root/Android/Sdk/platform-tools ]]; then
  export ANDROID_HOME=/root/Android/Sdk
fi
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | tail -1)}"
export PATH="$HOME/.cargo/bin:/root/.cargo/bin:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$PATH"

# Ensure default toolchain + Android targets exist (fixes "no default is configured")
if command -v rustup >/dev/null 2>&1; then
  rustup default stable >/dev/null 2>&1 || true
  rustup target add aarch64-linux-android >/dev/null 2>&1 || true
fi
if ! command -v x >/dev/null 2>&1; then
  echo "xbuild (x) not found. Install with: cargo install xbuild" >&2
  exit 1
fi
if ! command -v rustc >/dev/null 2>&1; then
  echo "rustc not found. Open a new terminal or run: source \$HOME/.cargo/env" >&2
  exit 1
fi
echo "Using rustc: $(rustc --version)"
echo "Using x:     $(command -v x)"

# Prefer arm64 for phones (Play)
echo "=== Building arm64 release ==="
x build -p fixitgarage-ui --platform android --arch arm64 --format apk --release
mkdir -p dist
RAW="dist/FixItGarage-${VERSION_NAME}-arm64-raw.apk"
OUT="dist/FixItGarage-${VERSION_NAME}-arm64-play.apk"
cp -f target/x/release/android/fixitgarage-ui.apk "$RAW"
./scripts/package-apk-with-boot.sh "$RAW" "$OUT" "$VERSION_NAME" "$VERSION_CODE"

# Copy to Downloads for easy find
cp -f "$OUT" "$HOME/Downloads/FixItGarage-${VERSION_NAME}-arm64-play.apk" 2>/dev/null || \
  cp -f "$OUT" "/home/christopher/Downloads/FixItGarage-${VERSION_NAME}-arm64-play.apk"

echo
echo "Play upload APK ready:"
ls -lh "$OUT"
echo "Also: ~/Downloads/FixItGarage-${VERSION_NAME}-arm64-play.apk"
echo
echo "In Play Console: Test and release → Internal testing → Create new release → Upload this file."
