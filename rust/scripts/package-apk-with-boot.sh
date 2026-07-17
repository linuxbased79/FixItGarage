#!/usr/bin/env bash
# Rebuild an xbuild APK with Java bridges (BootReceiver + ShareReceiveActivity),
# OCR models, and a valid resources.arsc (stored uncompressed — required for API 30+).
#
# Usage:
#   package-apk-with-boot.sh <input.apk> <output.apk> [versionName] [versionCode]
#
# Optional env for Play/production signing:
#   FIG_KEYSTORE=/path/to/upload.jks
#   FIG_KEYSTORE_PASS=...
#   FIG_KEY_ALIAS=upload
#   FIG_KEY_PASS=...
set -euo pipefail

IN_APK="${1:?input apk}"
OUT_APK="${2:?output apk}"
VERSION_NAME="${3:-0.2.24}"
VERSION_CODE="${4:-2024}"

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
DEFAULT_DEBUG_KS="$(cd "$(dirname "$0")" && pwd)/debug.keystore"

# Signing: production keystore via env, else debug
KEYSTORE="${FIG_KEYSTORE:-$DEFAULT_DEBUG_KS}"
KS_PASS="${FIG_KEYSTORE_PASS:-android}"
KEY_ALIAS="${FIG_KEY_ALIAS:-androiddebugkey}"
KEY_PASS="${FIG_KEY_PASS:-android}"

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
  if [[ "$KEYSTORE" == "$DEFAULT_DEBUG_KS" ]]; then
    keytool -genkeypair -v -keystore "$KEYSTORE" -storepass android -keypass android \
      -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
      -dname "CN=Android Debug,O=Android,C=US"
  else
    echo "Missing keystore: $KEYSTORE" >&2
    exit 1
  fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- Compile Java bridges → classes.dex ---
mkdir -p "$WORK/classes"
mapfile -t JAVA_FILES < <(find "$JAVA_SRC" -name '*.java' | sort)
if [[ ${#JAVA_FILES[@]} -eq 0 ]]; then
  echo "No Java sources under $JAVA_SRC" >&2
  exit 1
fi
"$JAVAC" -source 8 -target 8 -bootclasspath "$ANDROID_JAR" \
  -d "$WORK/classes" \
  "${JAVA_FILES[@]}"
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

# --- Compile Android resources (data extraction rules, etc.) ---
RES_DIR="$ROOT/fixitgarage-ui/android/res"
LINK_EXTRAS=()
if [[ -d "$RES_DIR" ]]; then
  mkdir -p "$WORK/compiled"
  while IFS= read -r -d '' resf; do
    "$AAPT2" compile -o "$WORK/compiled/" "$resf"
  done < <(find "$RES_DIR" -type f -print0)
  mapfile -t FLATS < <(find "$WORK/compiled" -name '*.flat' | sort)
  if [[ ${#FLATS[@]} -gt 0 ]]; then
    LINK_EXTRAS+=("${FLATS[@]}")
    echo "Compiled ${#FLATS[@]} resource file(s)"
  fi
fi

# --- Link APK with aapt2 (binary manifest + resources.arsc) ---
"$AAPT2" link -o "$WORK/base.apk" \
  --manifest "$WORK/AndroidManifest.xml" \
  -I "$ANDROID_JAR" \
  --min-sdk-version 26 \
  --target-sdk-version 34 \
  --version-code "$VERSION_CODE" \
  --version-name "$VERSION_NAME" \
  "${LINK_EXTRAS[@]+"${LINK_EXTRAS[@]}"}"

# --- Unpack and reassemble ---
mkdir -p "$WORK/in" "$WORK/out"
unzip -q "$IN_APK" -d "$WORK/in"
unzip -q "$WORK/base.apk" -d "$WORK/out"

if [[ -d "$WORK/in/lib" ]]; then
  cp -a "$WORK/in/lib" "$WORK/out/"
fi
cp -f "$WORK/classes.dex" "$WORK/out/classes.dex"

# On-device OCR models
MODELS_DIR="$ROOT/fixitgarage-ui/models"
if [[ -f "$MODELS_DIR/text-detection.rten" && -f "$MODELS_DIR/text-recognition.rten" ]]; then
  mkdir -p "$WORK/out/assets/models"
  cp -f "$MODELS_DIR/text-detection.rten" "$MODELS_DIR/text-recognition.rten" "$WORK/out/assets/models/"
  echo "Bundled OCR models into APK assets (~$(du -sh "$MODELS_DIR" | awk '{print $1}'))"
else
  echo "WARNING: OCR models missing under $MODELS_DIR — on-device OCR will download on first use" >&2
fi

# Zip correctly for Android R+:
# - resources.arsc MUST be stored (no compression) and 4-byte aligned
# - native libs preferably stored uncompressed for mmap (zipalign -p)
rm -f "$WORK/unsigned.apk"
(
  cd "$WORK/out"
  # Stored entries first
  if [[ -f resources.arsc ]]; then
    zip -q -X -0 "$WORK/unsigned.apk" resources.arsc
  fi
  # Native libs stored uncompressed (page-align friendly)
  if [[ -d lib ]]; then
    find lib -type f -name '*.so' -print0 | xargs -0 -r zip -q -X -0 "$WORK/unsigned.apk"
  fi
  # Everything else compressed (exclude already-added)
  zip -q -X -r -9 "$WORK/unsigned.apk" . \
    -x 'resources.arsc' \
    -x 'lib/*/*' \
    -x 'lib/*'
  # If lib dirs empty of so handled above, still add any other lib files compressed
  if [[ -d lib ]]; then
    find lib -type f ! -name '*.so' -print0 | xargs -0 -r zip -q -X -9 "$WORK/unsigned.apk"
  fi
)

# zipalign: -p page-aligns .so; -f force
"$ZIPALIGN" -f -p 4 "$WORK/unsigned.apk" "$WORK/aligned.apk"

"$APKSIGNER" sign \
  --ks "$KEYSTORE" \
  --ks-pass "pass:${KS_PASS}" \
  --key-pass "pass:${KEY_PASS}" \
  --ks-key-alias "$KEY_ALIAS" \
  --v1-signing-enabled false \
  --v2-signing-enabled true \
  --v3-signing-enabled true \
  --out "$OUT_APK" \
  "$WORK/aligned.apk"

# Verify alignment of resources.arsc
if ! "$ZIPALIGN" -c -p 4 "$OUT_APK" 2>/dev/null; then
  echo "WARNING: zipalign check reported issues" >&2
fi
# resources.arsc must show Stored
if unzip -lv "$OUT_APK" | grep -E 'resources\.arsc' | grep -q 'Defl'; then
  echo "ERROR: resources.arsc is compressed — install will fail on API 30+" >&2
  exit 1
fi

echo "Packaged: $OUT_APK (keystore=$(basename "$KEYSTORE") alias=$KEY_ALIAS)"
"$APKSIGNER" verify --verbose "$OUT_APK" 2>&1 | head -8 || true
unzip -lv "$OUT_APK" | grep -E 'resources\.arsc|classes\.dex|lib/' | head -10
