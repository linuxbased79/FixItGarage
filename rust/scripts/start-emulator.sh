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

# Android emulator's bundled Qt does NOT ship a Wayland plugin (only xcb).
# On KDE/GNOME Wayland this still works via XWayland when QT_QPA_PLATFORM=xcb.
# Override only if you know what you're doing: QT_QPA_PLATFORM=minimal fig-emulator
if [[ -z "${QT_QPA_PLATFORM:-}" ]]; then
  export QT_QPA_PLATFORM=xcb
fi

# Crash-report / metrics dirs must be writable by the desktop user
EMU_TMP="${TMPDIR:-/tmp}/android-${USER}"
mkdir -p "$EMU_TMP"
# If a root-owned dir was left behind from earlier runs, recreate as this user
if [[ ! -w "$EMU_TMP" ]]; then
  rm -rf "$EMU_TMP" 2>/dev/null || true
  mkdir -p "$EMU_TMP"
fi
export ANDROID_EMU_HOME="${ANDROID_EMU_HOME:-$HOME/.android}"
export ANDROID_EMULATOR_HOME="${ANDROID_EMULATOR_HOME:-$HOME/.android}"
# Quiet metrics prompt / collection for local dev
export ANDROID_EMU_DISABLE_METRICS_REPORTING=1

# Ensure AVD config is visible to this user
if [[ ! -f "$HOME/.android/avd/${AVD_NAME}.ini" && -f /root/.android/avd/${AVD_NAME}.ini ]]; then
  mkdir -p "$HOME/.android/avd"
  if [[ ! -e "$HOME/.android/avd/${AVD_NAME}.avd" ]]; then
    ln -s "/root/.android/avd/${AVD_NAME}.avd" "$HOME/.android/avd/${AVD_NAME}.avd" 2>/dev/null || true
    ln -s "/root/.android/avd/${AVD_NAME}.ini" "$HOME/.android/avd/${AVD_NAME}.ini" 2>/dev/null || true
  fi
fi

# Drop stale AVD locks from a previous crash
rm -f "$HOME/.android/avd/${AVD_NAME}.avd"/*.lock 2>/dev/null || true

# GPU: auto first; if the host driver is flaky, retry with software
GPU_MODE="${EMU_GPU:-auto}"

echo "Starting AVD: $AVD_NAME"
echo "  Qt platform: $QT_QPA_PLATFORM  |  GPU: $GPU_MODE  |  KVM if available"
echo "Stop with: adb emu kill   or close the emulator window"
echo

# shellcheck disable=SC2086
exec emulator -avd "$AVD_NAME" \
  -gpu "$GPU_MODE" \
  -no-metrics \
  -no-snapshot-save \
  -netdelay none \
  -netspeed full \
  "$@"
