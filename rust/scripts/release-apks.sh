#!/usr/bin/env bash
# Build release APKs (x86_64 + arm64) with BootReceiver packaging.
# Usage: ./scripts/release-apks.sh [versionName] [versionCode]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION_NAME="${1:-}"
if [[ -z "$VERSION_NAME" ]]; then
  VERSION_NAME="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
fi
# versionCode: 2_0_16 → 2016 style from semver 0.2.16
VERSION_CODE="${2:-}"
if [[ -z "$VERSION_CODE" ]]; then
  # 0.2.16 → 2016 ; 1.0.0 → 10000
  VERSION_CODE="$(python3 - <<PY
v="$VERSION_NAME".split(".")
maj,min,pat=(int(v[0]),int(v[1]) if len(v)>1 else 0,int(v[2]) if len(v)>2 else 0)
print(maj*10000+min*100+pat)
PY
)"
fi

export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
if [[ ! -d "$ANDROID_HOME/platform-tools" && -d /root/Android/Sdk/platform-tools ]]; then
  export ANDROID_HOME=/root/Android/Sdk
fi
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | tail -1)}"
export PATH="$HOME/.cargo/bin:/root/.cargo/bin:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$PATH"

echo "Release $VERSION_NAME (versionCode=$VERSION_CODE)"
echo "ANDROID_HOME=$ANDROID_HOME"
echo "ANDROID_NDK_ROOT=$ANDROID_NDK_ROOT"

mkdir -p dist
PKG="$ROOT/scripts/package-apk-with-boot.sh"
chmod +x "$PKG" || true

build_one() {
  local arch="$1"   # x64 | arm64
  local label="$2"  # x86_64 | arm64
  echo "=== Building $label ($arch) ==="
  x build -p fixitgarage-ui --platform android --arch "$arch" --format apk --release
  local raw="dist/FixItGarage-${VERSION_NAME}-${label}-raw.apk"
  local out="dist/FixItGarage-${VERSION_NAME}-${label}.apk"
  cp -f target/x/release/android/fixitgarage-ui.apk "$raw"
  "$PKG" "$raw" "$out" "$VERSION_NAME" "$VERSION_CODE"
  ls -lh "$out"
}

build_one x64 x86_64
build_one arm64 arm64

echo "Done."
ls -lh dist/FixItGarage-${VERSION_NAME}-*.apk
