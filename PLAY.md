# Google Play — submit FixItGarage

Primary binary: **Rust / Slint** APK `org.fixitgarage.app` (not the legacy Kotlin `app/` shell).

## Package identity

| Field | Value |
|-------|--------|
| Application ID | `org.fixitgarage.app` |
| License | GPL-3.0-only |
| Min SDK | 26 |
| Target SDK | 34 |
| Version (this release) | **0.2.20** (versionCode **2020**) |
| Privacy policy URL | https://raw.githubusercontent.com/linuxbased79/FixItGarage/main/PRIVACY.md |
| Support | https://github.com/linuxbased79/FixItGarage/issues |

## 1. Create upload keystore (once)

```bash
cd rust
./scripts/create-upload-keystore.sh ~/fixitgarage-upload.jks upload
export FIG_KEYSTORE=$HOME/fixitgarage-upload.jks
export FIG_KEYSTORE_PASS='…'
export FIG_KEY_ALIAS=upload
export FIG_KEY_PASS='…'
```

Never commit the keystore or passwords.

## 2. Build release APKs

```bash
cd rust
./fixitgarage-ui/models/download-models.sh   # if models missing
./scripts/release-apks.sh 0.2.20 2020
# dist/FixItGarage-0.2.20-arm64.apk   → phones (upload this)
# dist/FixItGarage-0.2.20-x86_64.apk  → emulator
```

Debug-signed builds (default) are for testing only. Set `FIG_KEYSTORE*` for Play upload.

## 3. Play Console checklist

1. Create app → **App name:** FixItGarage  
2. **App category:** Auto & Vehicles (or Tools)  
3. **Free** app; declare GPL-3.0 in Store listing / About  
4. **Privacy policy** URL (above)  
5. **Data safety form:**  
   - Data collected: none by developer servers  
   - Data may stay on device; user may share backups  
   - No ads, no account, no sale of data  
6. **Content rating** questionnaire (utility / tools)  
7. **Target audience:** 18+ / general (not primarily children)  
8. **Store listing**  
   - Short / full description: `metadata/en-US/*_description.txt`  
   - Screenshots: `metadata/en-US/images/phoneScreenshots/`  
   - Feature graphic: `metadata/en-US/images/featureGraphic/featureGraphic.png` (1024×500)  
   - Icon: `metadata/en-US/images/icon/icon.png` (512×512)  
9. **App access:** no login required  
10. **Permissions declaration:** Internet (optional), Camera (optional), Notifications, Exact alarms, Boot completed (local alarms only)  
11. Upload **arm64** APK (or App Bundle if you add AAB later) to **Internal testing** first  
12. Enable **Play App Signing** (recommended)

## 4. GrapheneOS / no GMS

Core features work without Google Play Services. OCR is on-device (ocrs). Cloud is share-sheet / WebDAV only.

## Pre-submit smoke test

1. Fresh install arm64 APK  
2. Wizard → vehicle → log service → date reminder  
3. Capture receipt → On-device OCR  
4. Tread photo → Measure tread (CV)  
5. Reboot → alarms re-register  
6. Export CSV + JSON backup  

## Status

Packaging, assets, and privacy URL are ready in-repo. Final upload requires your Play Console account.
