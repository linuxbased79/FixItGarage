# Google Play listing checklist (FixItGarage)

Primary product binary: **Rust / Slint** APK (`org.fixitgarage.app`), not the legacy Kotlin shell under `app/` (kept as reference).

## Package identity
| Field | Value |
|-------|--------|
| Application ID | `org.fixitgarage.app` |
| License | GPL-3.0-only |
| Min SDK | 26 |
| Target SDK | 34 (packaged manifest) |
| Signing | Use your **upload key** / Play App Signing (do not ship the repo debug keystore) |

## Build release APKs
```bash
cd rust
./scripts/release-apks.sh          # builds x86_64 + arm64 with BootReceiver
# outputs: dist/FixItGarage-<ver>-{x86_64,arm64}.apk
```

Upload **arm64** (and optionally a universal/multi-APK set) to Play Console. Emulator builds are x86_64.

## Store listing (suggested)
- **Title:** FixItGarage  
- **Short description:** Local-first vehicle maintenance tracker (no account required).  
- **Full description:** See `metadata/en-US/full_description.txt`  
- **Category:** Auto & Vehicles / Tools  
- **Contact / support:** GitHub Issues  
  https://github.com/linuxbased79/FixItGarage/issues  

## Privacy
- Local-first; no required accounts  
- Optional WebDAV upload and share-sheet cloud backup (user-initiated)  
- Camera optional (issue / receipt / tread assist photos)  
- See `PRIVACY.md`  

## Permissions to declare in Play form
- Internet (optional WebDAV / open URLs)  
- Camera (optional)  
- Notifications / exact alarms (reminders)  
- Boot completed (re-register local alarms only)  

## GrapheneOS / no GMS
Core features work without Google Play Services. Cloud “sync” is share-intent / WebDAV, not proprietary GMS SDKs.

## Not yet automated
- Play Console upload / internal testing track  
- Feature graphic / screenshots set  
- Content rating questionnaire (fill in Console)  

## Pre-submit smoke test
1. Fresh install arm64 APK  
2. Wizard → add vehicle → log service → set a **date** reminder tomorrow  
3. Reboot device → confirm alarm still fires (or is present in AlarmManager dumpsys)  
4. Toggle OpenDyslexic + a non-English language pack  
5. Export CSV + create JSON backup  
