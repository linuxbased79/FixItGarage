#!/usr/bin/env bash
# Start the FixItGarage Android emulator (GUI) on Debian 13.
# Uses KVM when available. Prefer running as your normal desktop user.
set -euo pipefail

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
# Fall back to root SDK if this machine was set up that way
if [[ ! -d "$ANDROID_HOME/emulator" && -d /root/Android/Sdk/emulator ]]; then
  export ANDROID_HOME=/root/Android/Sdk
fi
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export PATH="$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"

AVD_NAME="${AVD_NAME:-FixItGarage_API34}"

if ! command -v emulator >/dev/null; then
  echo "Android emulator not found under $ANDROID_HOME" >&2
  exit 1
fi

if ! emulator -list-avds | grep -qx "$AVD_NAME"; then
  echo "AVD '$AVD_NAME' missing. Create with:" >&2
  echo "  avdmanager create avd -n $AVD_NAME -k 'system-images;android-34;google_apis;x86_64' -d pixel_6 --force" >&2
  exit 1
fi

# Prefer Wayland on KDE/Plasma/GNOME Wayland sessions; override if needed:
#   QT_QPA_PLATFORM=xcb ./start-emulator.sh
if [[ -z "${QT_QPA_PLATFORM:-}" ]]; then
  if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
    export QT_QPA_PLATFORM=wayland
  else
    export QT_QPA_PLATFORM=xcb
  fi
fi

# Ensure AVD config is visible to this user
if [[ ! -f "$HOME/.android/avd/${AVD_NAME}.ini" && -f /root/.android/avd/${AVD_NAME}.ini ]]; then
  mkdir -p "$HOME/.android/avd"
  # Symlink root AVD for the desktop user when needed
  if [[ ! -e "$HOME/.android/avd/${AVD_NAME}.avd" ]]; then
    ln -s "/root/.android/avd/${AVD_NAME}.avd" "$HOME/.android/avd/${AVD_NAME}.avd" 2>/dev/null || true
    ln -s "/root/.android/avd/${AVD_NAME}.ini" "$HOME/.android/avd/${AVD_NAME}.ini" 2>/dev/null || true
  fi
fi

echo "Starting AVD: $AVD_NAME (GPU auto, KVM if available)"
echo "Stop with: adb emu kill   or close the emulator window"
exec emulator -avd "$AVD_NAME" \
  -gpu auto \
  -no-snapshot-save \
  -netdelay none \
  -netspeed full \
  "$@"
