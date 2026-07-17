#!/usr/bin/env bash
# One-time setup: Android emulator + API 34 x86_64 image on Debian 13.
set -euo pipefail

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk-amd64}"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$JAVA_HOME/bin:$PATH"

if ! command -v sdkmanager >/dev/null; then
  echo "sdkmanager not found. Install Android cmdline-tools first." >&2
  exit 1
fi

echo "Installing emulator + system image (this can take a while)..."
yes | sdkmanager --licenses >/dev/null 2>&1 || true
sdkmanager --install \
  "emulator" \
  "platform-tools" \
  "platforms;android-34" \
  "system-images;android-34;google_apis;x86_64"

AVD_NAME=FixItGarage_API34
echo no | avdmanager create avd \
  -n "$AVD_NAME" \
  -k "system-images;android-34;google_apis;x86_64" \
  -d pixel_6 \
  --force

CFG="$HOME/.android/avd/${AVD_NAME}.avd/config.ini"
{
  echo "hw.keyboard=yes"
  echo "hw.ramSize=3072"
  echo "hw.gpu.enabled=yes"
  echo "hw.gpu.mode=auto"
} >> "$CFG"

# Allow KVM for this user if group exists
if getent group kvm >/dev/null && [[ -n "${SUDO_USER:-}" || "$(id -u)" -ne 0 ]]; then
  U="${SUDO_USER:-$USER}"
  if ! id -nG "$U" | grep -qw kvm; then
    echo "Tip: add yourself to kvm group for acceleration:"
    echo "  sudo usermod -aG kvm $U && newgrp kvm"
  fi
fi

echo
echo "AVD ready: $AVD_NAME"
echo "Start GUI:  $PWD/scripts/start-emulator.sh  (from rust/)"
echo "Install app: $PWD/scripts/install-on-emulator.sh"
