#!/usr/bin/env bash
# Install FixItGarage APK on a running emulator (or device) and launch it.
set -euo pipefail

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
if [[ ! -d "$ANDROID_HOME/platform-tools" && -d /root/Android/Sdk/platform-tools ]]; then
  export ANDROID_HOME=/root/Android/Sdk
fi
export PATH="$ANDROID_HOME/platform-tools:$PATH"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Emulator is x86_64; phones are usually arm64
APK_X64="$ROOT/dist/FixItGarage-0.2.18-x86_64.apk"
APK_ARM="$ROOT/dist/FixItGarage-0.2.18-arm64.apk"
APK="${1:-}"

if [[ -z "$APK" ]]; then
  ABI=$(adb shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r' || true)
  if [[ "$ABI" == x86_64* || "$ABI" == x86* ]]; then
    APK="$APK_X64"
  else
    APK="$APK_ARM"
  fi
fi

if [[ ! -f "$APK" ]]; then
  echo "APK not found: $APK" >&2
  echo "Build with: cd $ROOT && x build -p fixitgarage-ui --platform android --arch x64 --format apk --release" >&2
  exit 1
fi

echo "Waiting for device..."
adb wait-for-device
# Wait for boot if needed
for i in $(seq 1 60); do
  [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]] && break
  sleep 2
done

echo "Installing $APK"
adb install -r "$APK"
echo "Launching FixItGarage..."
adb shell am start -a android.intent.action.MAIN \
  -n org.fixitgarage.app/android.app.NativeActivity
echo "Done. Package: org.fixitgarage.app"
