#!/usr/bin/env bash
# Build FixItGarage arm64 APK via xbuild (Slint / NativeActivity).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_HOME/ndk/27.2.12479018}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_NDK_ROOT}"
export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk-amd64}"
export PATH="$HOME/.cargo/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$JAVA_HOME/bin:$PATH"

if [[ ! -d "$ANDROID_NDK_ROOT" ]]; then
  echo "NDK not found at $ANDROID_NDK_ROOT" >&2
  echo "Install with: sdkmanager --install 'ndk;27.2.12479018'" >&2
  exit 1
fi

if ! command -v x >/dev/null; then
  echo "xbuild (x) not found. Install: cargo install --git https://github.com/rust-mobile/xbuild.git" >&2
  exit 1
fi

rustup target add aarch64-linux-android

echo "Building FixItGarage APK (arm64-v8a)..."
x build -p fixitgarage-ui --platform android --arch arm64 --format apk --release

mkdir -p dist
cp -f target/x/release/android/fixitgarage-ui.apk dist/FixItGarage-0.1.0-arm64.apk
echo "APK: $ROOT/dist/FixItGarage-0.1.0-arm64.apk"
ls -lh dist/FixItGarage-0.1.0-arm64.apk
