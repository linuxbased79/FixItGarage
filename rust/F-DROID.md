# F-Droid packaging notes (FixItGarage)

## Binary
- Package ID: `org.fixitgarage.app`
- Built from `rust/fixitgarage-ui` with Slint + android-activity (no GMS).

## Suggested build (maintainer)
```bash
cd rust
rustup target add aarch64-linux-android
# NDK + xbuild as documented in fixitgarage-ui/README.md
x build -p fixitgarage-ui --platform android --arch arm64 --format apk --release
```

## AntiFeatures
- `NonFreeNet` only if user enables optional WebDAV upload to a third-party host (feature is opt-in).
- Camera permission used for optional issue/receipt photos.
- `SCHEDULE_EXACT_ALARM` / `USE_EXACT_ALARM` for optional date-based reminder wake-ups (local only).
- `RECEIVE_BOOT_COMPLETED` so `BootReceiver` can re-register local date alarms after reboot (no network).
- OCR helper may open an external browser/Lens URL; core parsing is on-device from pasted text.

## Boot-resilient packaging
After `x build ... --format apk`, run:

```bash
./scripts/package-apk-with-boot.sh dist/FixItGarage-0.2.15-arm64-raw.apk dist/FixItGarage-0.2.15-arm64.apk 0.2.15 2015
```

This injects `org.fixitgarage.app.BootReceiver` + `classes.dex` and sets `android:hasCode=true`.

## Source
https://github.com/linuxbased79/FixItGarage
