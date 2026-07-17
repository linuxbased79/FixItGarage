# F-Droid packaging notes (FixItGarage)

## Binary
- **Package ID:** `org.fixitgarage.app`
- **Source module:** `rust/fixitgarage-ui` (Slint + android-activity, no GMS)
- **License:** GPL-3.0-only
- **Draft metadata:** `metadata/org.fixitgarage.app.yml` + `metadata/en-US/*`

## One-shot release build (recommended)
From the `rust/` directory (NDK + SDK + `x` / xbuild required):

```bash
export ANDROID_HOME=~/Android/Sdk   # or /root/Android/Sdk
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/<version>
./scripts/release-apks.sh
# → dist/FixItGarage-<ver>-arm64.apk   (phones)
# → dist/FixItGarage-<ver>-x86_64.apk  (emulator)
```

Each APK is post-processed with `package-apk-with-boot.sh` to include:

- `org.fixitgarage.app.BootReceiver` (`classes.dex`)
- `android:hasCode="true"`
- `RECEIVE_BOOT_COMPLETED` + alarm permissions

### Manual two-step (arm64 only)
```bash
cd rust
rustup target add aarch64-linux-android
x build -p fixitgarage-ui --platform android --arch arm64 --format apk --release
cp target/x/release/android/fixitgarage-ui.apk dist/FixItGarage-VER-arm64-raw.apk
./scripts/package-apk-with-boot.sh \
  dist/FixItGarage-VER-arm64-raw.apk \
  dist/FixItGarage-VER-arm64.apk \
  VER VERSION_CODE
```

## AntiFeatures
- `NonFreeNet` only if the user enables optional WebDAV upload to a third-party host (opt-in).
- Camera permission for optional issue/receipt/tread assist photos.
- `SCHEDULE_EXACT_ALARM` / `USE_EXACT_ALARM` for local date-based reminder wakes.
- `RECEIVE_BOOT_COMPLETED` so BootReceiver can re-register **local** date alarms after reboot (no network).
- OCR helper may open an external browser/Lens URL; core receipt parse is on-device from pasted text.

## Privacy
Local-first; no required account. See root `PRIVACY.md`.

## fdroiddata submission sketch
1. Fork [fdroiddata](https://gitlab.com/fdroid/fdroiddata)
2. Copy `metadata/org.fixitgarage.app.yml` and fill `commit:` with a signed git tag
3. Copy/adapt `metadata/en-US/*` listing text
4. Run `fdroid build org.fixitgarage.app` in a server environment with NDK
5. Open merge request with build logs

## Source
https://github.com/linuxbased79/FixItGarage  
Issues: https://github.com/linuxbased79/FixItGarage/issues

## Deep OCR (0.2.18+)
- `ShareReceiveActivity` is a share target for `text/plain` and `image/*` (OCR apps → FixItGarage).
- Capture uses MediaStore; “Send photo to OCR” uses `ACTION_SEND` image/* to Text Fairy / Lens / chooser.
- No bundled Tesseract/ML Kit (keeps APK size + GrapheneOS-friendly; free OCR via F-Droid Text Fairy).
