#!/usr/bin/env bash
# Rebuild an xbuild APK with BootReceiver (classes.dex) + full manifest so
# date-based reminder alarms are re-registered after device reboot.
#
# Usage:
#   package-apk-with-boot.sh <input.apk> <output.apk> [versionName]
set -euo pipefail

IN_APK="${1:?input apk}"
OUT_APK="${2:?output apk}"
VERSION_NAME="${3:-0.2.18}"
VERSION_CODE="${4:-2018}"

export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
if [[ ! -d "$ANDROID_HOME/platform-tools" && -d /root/Android/Sdk/platform-tools ]]; then
  export ANDROID_HOME=/root/Android/Sdk
fi
BT="$(ls -d "$ANDROID_HOME"/build-tools/*/ 2>/dev/null | sort -V | tail -1)"
AAPT2="${BT}aapt2"
D8="${BT}d8"
[[ -x "$D8" ]] || D8="$ANDROID_HOME/cmdline-tools/latest/bin/d8"
ZIPALIGN="${BT}zipalign"
APKSIGNER="${BT}apksigner"
ANDROID_JAR="$(ls -d "$ANDROID_HOME"/platforms/android-*/android.jar 2>/dev/null | sort -V | tail -1)"
JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk-amd64}"
JAVAC="${JAVA_HOME}/bin/javac"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JAVA_SRC="$ROOT/fixitgarage-ui/android/java"
MANIFEST_SRC="$ROOT/fixitgarage-ui/android/AndroidManifest.xml"
KEYSTORE="$(cd "$(dirname "$0")" && pwd)/debug.keystore"

if [[ ! -f "$IN_APK" ]]; then
  echo "Missing input APK: $IN_APK" >&2
  exit 1
fi
if [[ ! -f "$MANIFEST_SRC" ]]; then
  echo "Missing manifest: $MANIFEST_SRC" >&2
  exit 1
fi
if [[ ! -f "$ANDROID_JAR" ]]; then
  echo "Missing android.jar under $ANDROID_HOME/platforms" >&2
  exit 1
fi
if [[ ! -x "$JAVAC" ]]; then
  echo "javac not found at $JAVAC" >&2
  exit 1
fi
if [[ ! -f "$KEYSTORE" ]]; then
  keytool -genkeypair -v -keystore "$KEYSTORE" -storepass android -keypass android \
    -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
    -dname "CN=Android Debug,O=Android,C=US"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- Compile Java bridges (BootReceiver + ShareReceiveActivity) → classes.dex ---
mkdir -p "$WORK/classes"
mapfile -t JAVA_FILES < <(find "$JAVA_SRC" -name '*.java' | sort)
if [[ ${#JAVA_FILES[@]} -eq 0 ]]; then
  echo "No Java sources under $JAVA_SRC" >&2
  exit 1
fi
"$JAVAC" -source 8 -target 8 -bootclasspath "$ANDROID_JAR" \
  -d "$WORK/classes" \
  "${JAVA_FILES[@]}"
# d8 may be a shell script or binary
if [[ -x "$D8" ]]; then
  (cd "$WORK" && "$D8" --lib "$ANDROID_JAR" --min-api 26 --output "$WORK" \
    $(find classes -name '*.class'))
else
  echo "d8 not found" >&2
  exit 1
fi
if [[ ! -f "$WORK/classes.dex" ]]; then
  echo "d8 did not produce classes.dex" >&2
  exit 1
fi
echo "Java classes: ${JAVA_FILES[*]}"

# --- Manifest with version ---
sed -e "s/package=\"org.fixitgarage.app\"/package=\"org.fixitgarage.app\" android:versionCode=\"${VERSION_CODE}\" android:versionName=\"${VERSION_NAME}\"/" \
  "$MANIFEST_SRC" > "$WORK/AndroidManifest.xml"

# --- Link bare APK with aapt2 (binary manifest) ---
"$AAPT2" link -o "$WORK/base.apk" \
  --manifest "$WORK/AndroidManifest.xml" \
  -I "$ANDROID_JAR" \
  --min-sdk-version 26 \
  --target-sdk-version 34 \
  --version-code "$VERSION_CODE" \
  --version-name "$VERSION_NAME"

# --- Unpack and reassemble ---
mkdir -p "$WORK/in" "$WORK/out"
unzip -q "$IN_APK" -d "$WORK/in"
unzip -q "$WORK/base.apk" -d "$WORK/out"

# Keep libs from xbuild APK
if [[ -d "$WORK/in/lib" ]]; then
  cp -a "$WORK/in/lib" "$WORK/out/"
fi
# Optional: resources from base (resources.arsc)
cp -f "$WORK/classes.dex" "$WORK/out/classes.dex"

# Zip (store compressed for most files; libs compressed ok)
(cd "$WORK/out" && zip -q -r -9 "$WORK/unsigned.apk" .)

# zipalign then sign
"$ZIPALIGN" -f 4 "$WORK/unsigned.apk" "$WORK/aligned.apk"
"$APKSIGNER" sign \
  --ks "$KEYSTORE" \
  --ks-pass pass:android \
  --key-pass pass:android \
  --ks-key-alias androiddebugkey \
  --out "$OUT_APK" \
  "$WORK/aligned.apk"

echo "Packaged with BootReceiver: $OUT_APK"
"$APKSIGNER" verify --verbose "$OUT_APK" 2>&1 | head -5 || true
