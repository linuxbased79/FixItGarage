# Privacy Policy — Motor Noter

**Last updated:** 2026-09-15  
**App ID:** `org.fixitgarage.app`  
**Contact:** [GitHub Issues](https://github.com/linuxbased79/FixItGarage/issues)

Motor Noter (package `org.fixitgarage.app`) is **local-first** open-source software (GPL-3.0). Core features work without an account and without Google Play Services (including on GrapheneOS).

## Data stored on your device

- Vehicles and maintenance history you enter  
- Tire, brake, battery, wiper, parts, costs, notes, and reminders  
- Optional photos (issues, receipts, tread assist)  
- App preferences (theme, units, language, accessibility font, wizard mode)  
- Optional WebDAV credentials you enter for backup (stored only on this device; **not** included in shared JSON backups)  
- Local alarm schedule for date reminders (`fig_alarms.json`)

Data is stored in the app’s private storage on your phone. Android Auto Backup of app data is **disabled**. We do **not** operate a Motor Noter cloud account or analytics backend.

## Network activity

Network use is **optional and user-initiated**:

| Action | What leaves the device |
|--------|-------------------------|
| Send Feedback | Opens GitHub Issues in a browser |
| Donate | Opens the project donate page (linuxbased79.github.io/FixItGarage/donate.html) |
| Share backup / CSV | System share sheet — destination app you choose (Proton Drive, Drive, Dropbox, OneDrive, etc.) |
| WebDAV / Nextcloud / ownCloud upload | Only when you configure an **https://** URL and tap upload (HTTP cleartext blocked) |
| On-device OCR model fallback download | Only if bundled models are missing; downloads are HTTPS, no redirects, SHA-256 pinned |
| NHTSA recall check | VIN/make/model/year sent to public US NHTSA APIs (user-initiated) |
| Open OCR helper / F-Droid / market links | Opens external apps or sites you choose |

We do **not** sell personal data. We do **not** include advertising or third-party tracking SDKs.

## Camera & notifications

- **Camera** is optional (issue photos, receipt OCR, title/VIN, tread assist). Captures are copied into app-private storage; the temporary public MediaStore row is deleted. Photos leave the device only if you share them.  
- **Notifications / exact alarms / boot completed** are used only for **local** maintenance reminders (re-registered after reboot). No remote push service.

## On-device OCR & computer vision

Receipt text recognition and tread coin-gauge estimation run **on your device** (pure Rust OCR models bundled in the app). No Google ML Kit / Play Services OCR is required.

## Children

The app is a general-purpose vehicle maintenance tool, not directed at children under 13.

## Changes

Material changes to this policy will be noted in the app changelog and this file in the source repository.

## Source

https://github.com/linuxbased79/FixItGarage  
Public copy of this policy:  
https://github.com/linuxbased79/FixItGarage/blob/main/PRIVACY.md  
Raw URL (for store forms):  
https://raw.githubusercontent.com/linuxbased79/FixItGarage/main/PRIVACY.md
