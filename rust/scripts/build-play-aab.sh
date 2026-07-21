#!/usr/bin/env bash
# Build a signed Play App Bundle (.aab) for Google Play Console.
# Usage: ./scripts/build-play-aab.sh [versionName] [versionCode]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION_NAME="${1:-0.2.36}"
VERSION_CODE="${2:-2036}"
KEYSTORE="${FIG_KEYSTORE:-$HOME/fixitgarage-upload.jks}"
ALIAS="${FIG_KEY_ALIAS:-upload}"

if [[ ! -f "$KEYSTORE" ]]; then
  echo "Missing keystore: $KEYSTORE" >&2
  exit 1
fi

echo "Keystore: $KEYSTORE"
echo "Alias:    $ALIAS"
echo "Version:  $VERSION_NAME (code $VERSION_CODE)  targetSdk 35"
echo
if [[ -z "${FIG_KEYSTORE_PASS:-}" ]]; then
  read -r -s -p "Keystore password: " FIG_KEYSTORE_PASS
  echo
fi
export FIG_KEYSTORE_PASS
export FIG_KEY_PASS="${FIG_KEY_PASS:-$FIG_KEYSTORE_PASS}"

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
export GRADLE_USER_HOME="${GRADLE_USER_HOME:-$HOME/.gradle}"
export PATH="/opt/gradle-8.7/bin:/opt/gradle-8.2/bin:$HOME/.cargo/bin:/root/.cargo/bin:$ANDROID_HOME/platform-tools:$PATH"

JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk-amd64}"
export JAVA_HOME
JARSIGNER="$JAVA_HOME/bin/jarsigner"
[[ -x "$JARSIGNER" ]] || JARSIGNER="$(command -v jarsigner)"

if command -v rustup >/dev/null 2>&1; then
  rustup default stable >/dev/null 2>&1 || true
  rustup target add aarch64-linux-android >/dev/null 2>&1 || true
fi

if ! command -v x >/dev/null 2>&1; then
  echo "xbuild (x) not found" >&2
  exit 1
fi
if ! command -v gradle >/dev/null 2>&1; then
  echo "gradle not found (need /opt/gradle-8.7)" >&2
  exit 1
fi

echo "Using rustc:  $(rustc --version)"
echo "Using x:      $(command -v x)"
echo "Using gradle: $(command -v gradle) ($(gradle --version 2>/dev/null | head -1))"
echo "ANDROID_HOME: $ANDROID_HOME"

# Ensure platform 35 exists and is readable
if [[ ! -f "$ANDROID_HOME/platforms/android-35/android.jar" ]]; then
  echo "Installing platforms;android-35 ..."
  yes | "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" "platforms;android-35" "build-tools;35.0.0" || true
fi

echo "=== 1/3 Build native library (xbuild APK) ==="
x build -p fixitgarage-ui --platform android --arch arm64 --format apk --release

SO_CANDIDATES=(
  "$ROOT/target/x/release/android/gradle/app/src/main/jniLibs/arm64-v8a/libfixitgarage_ui.so"
  "$ROOT/target/x/release/android/arm64/libfixitgarage_ui.so"
)
SO=""
for c in "${SO_CANDIDATES[@]}"; do
  if [[ -f "$c" ]]; then SO="$c"; break; fi
done
# Extract from APK if needed
if [[ -z "$SO" ]]; then
  APK="$ROOT/target/x/release/android/fixitgarage-ui.apk"
  [[ -f "$APK" ]] || { echo "APK not found after xbuild" >&2; exit 1; }
  TMP="$(mktemp -d)"
  unzip -q -o "$APK" "lib/arm64-v8a/libfixitgarage_ui.so" -d "$TMP"
  SO="$TMP/lib/arm64-v8a/libfixitgarage_ui.so"
  [[ -f "$SO" ]] || { echo "Native .so not in APK" >&2; exit 1; }
fi
echo "Native lib: $SO ($(du -h "$SO" | cut -f1))"

echo "=== 2/3 Assemble Gradle project for bundle (AGP 8.5 / SDK 35) ==="
GDIR="$ROOT/target/x/release/android/play-bundle"
rm -rf "$GDIR"
mkdir -p "$GDIR/app/src/main/jniLibs/arm64-v8a"
cp -f "$SO" "$GDIR/app/src/main/jniLibs/arm64-v8a/libfixitgarage_ui.so"

# Brand icons + other Android resources (launcher icon, adaptive icons, etc.)
RES_SRC="$ROOT/fixitgarage-ui/android/res"
if [[ -d "$RES_SRC" ]]; then
  mkdir -p "$GDIR/app/src/main/res"
  cp -a "$RES_SRC/." "$GDIR/app/src/main/res/"
  echo "Copied Android res (icons) from $RES_SRC"
else
  mkdir -p "$GDIR/app/src/main/res/values"
  echo "WARNING: no $RES_SRC — stock icon will be used" >&2
fi
mkdir -p "$GDIR/app/src/main/res/values"
if [[ ! -f "$GDIR/app/src/main/res/values/strings.xml" ]]; then
  cat > "$GDIR/app/src/main/res/values/strings.xml" << 'STR'
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">FixItGarage</string>
</resources>
STR
fi

# Manifest with brand launcher icons
cat > "$GDIR/app/src/main/AndroidManifest.xml" << 'MANI'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.CAMERA" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    <uses-permission android:name="android.permission.SCHEDULE_EXACT_ALARM" />
    <uses-permission android:name="android.permission.USE_EXACT_ALARM" />
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    <uses-permission android:name="android.permission.WAKE_LOCK" />
    <application
        android:label="@string/app_name"
        android:icon="@mipmap/ic_launcher"
        android:roundIcon="@mipmap/ic_launcher_round"
        android:hasCode="true"
        android:allowBackup="false"
        android:extractNativeLibs="true">
        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:launchMode="singleTop"
            android:configChanges="orientation|keyboardHidden|keyboard|screenSize|smallestScreenSize|locale|layoutDirection|fontScale|screenLayout|density|uiMode"
            android:windowSoftInputMode="adjustResize"
            android:hardwareAccelerated="true">
            <meta-data android:name="android.app.lib_name" android:value="fixitgarage_ui" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
MANI

echo "sdk.dir=$ANDROID_HOME" > "$GDIR/local.properties"

cat > "$GDIR/settings.gradle" << 'SET'
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = "FixItGarage"
include ':app'
SET

cat > "$GDIR/build.gradle" << 'ROOT'
plugins {
    id 'com.android.application' version '8.5.2' apply false
}
ROOT

cat > "$GDIR/gradle.properties" << 'PROP'
org.gradle.jvmargs=-Xmx2g -Dfile.encoding=UTF-8
android.useAndroidX=true
android.nonTransitiveRClass=true
android.suppressUnsupportedCompileSdk=35
PROP

cat > "$GDIR/app/build.gradle" << GAPP
plugins {
    id 'com.android.application'
}
android {
    namespace 'org.fixitgarage.app'
    compileSdk 35
    defaultConfig {
        applicationId 'org.fixitgarage.app'
        minSdk 26
        targetSdk 35
        versionCode ${VERSION_CODE}
        versionName '${VERSION_NAME}'
    }
    buildTypes {
        release {
            minifyEnabled false
        }
    }
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}
dependencies {
}
GAPP

echo "=== 3/3 bundleRelease + sign with upload key ==="
(
  cd "$GDIR"
  gradle :app:bundleRelease --no-daemon
)

RAW="$(find "$GDIR/app/build/outputs/bundle" -name '*.aab' | head -1)"
[[ -n "$RAW" && -f "$RAW" ]] || { echo "No AAB produced" >&2; exit 1; }

mkdir -p dist
OUT="dist/FixItGarage-${VERSION_NAME}-play.aab"
cp -f "$RAW" "$OUT"

echo "Signing AAB with upload keystore (alias=$ALIAS)…"
if ! "$JARSIGNER" -sigalg SHA256withRSA -digestalg SHA-256 \
  -keystore "$KEYSTORE" \
  -storepass "$FIG_KEYSTORE_PASS" \
  -keypass "$FIG_KEY_PASS" \
  "$OUT" "$ALIAS"
then
  echo "ERROR: jarsigner failed — AAB is NOT signed. Check keystore password." >&2
  exit 1
fi

VERIFY_OUT="$("$JARSIGNER" -verify -verbose -certs "$OUT" 2>&1 || true)"
if ! echo "$VERIFY_OUT" | grep -qiE 'jar verified|s = signature was verified'; then
  echo "ERROR: AAB signature verification failed:" >&2
  echo "$VERIFY_OUT" | tail -20 >&2
  exit 1
fi
echo "Signature OK"
echo "$VERIFY_OUT" | grep -E 'CN=|signed by' | head -5 || true

DEST_HOME="$HOME/Downloads/FixItGarage-${VERSION_NAME}-play.aab"
cp -f "$OUT" "$DEST_HOME" 2>/dev/null || cp -f "$OUT" "/home/christopher/Downloads/FixItGarage-${VERSION_NAME}-play.aab"

# Final guard: refuse to advertise unsigned file
if ! "$JARSIGNER" -verify "$DEST_HOME" 2>&1 | grep -qiE 'jar verified|s = signature'; then
  # jarsigner -verify alone prints to stderr; check exit or message
  if ! "$JARSIGNER" -verify "$OUT" >/dev/null 2>&1; then
    echo "ERROR: final signed AAB check failed" >&2
    exit 1
  fi
fi

echo
echo "Play upload AAB ready (targetSdk 35, SIGNED):"
ls -lh "$OUT"
ls -lh "$DEST_HOME" 2>/dev/null || ls -lh "/home/christopher/Downloads/FixItGarage-${VERSION_NAME}-play.aab"
echo
echo "Upload in Play Console:"
echo "  ~/Downloads/FixItGarage-${VERSION_NAME}-play.aab"
echo "  versionName=$VERSION_NAME  versionCode=$VERSION_CODE  targetSdk=35"
