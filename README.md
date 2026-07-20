# FixItGarage

**Open-source vehicle maintenance tracker for Android**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

FixItGarage helps you track **unlimited vehicles**, service history (DIY + shop), tires and rotations, filters/oil part numbers, brakes, battery, wipers, costs, reminders, photos, and notes — local-first, with optional cloud backup via share sheet or WebDAV.

Planned distribution: **F-Droid** and **Google Play**. Designed for **GrapheneOS** (no Google Play Services required for core features).

## Status

**0.2.25** — primary product is the **Rust + Slint** APK (`org.fixitgarage.app`).

| Area | Status |
|------|--------|
| Setup wizard (DIY / shop / both) | Done |
| Unlimited vehicles | Done |
| Last service + home dashboard | Done |
| Maintenance history (DIY + shop) + templates + notes | Done |
| Automatic MPG / L/100km | Done |
| Tire diagram, rotation, spare option, per-vehicle layouts | Done |
| Tire purchase + receipt text parse | Done |
| Monthly / yearly costs (services + tires) | Done |
| Dark mode | Done |
| CSV + JSON backup; Proton / Drive / Dropbox / OneDrive / WebDAV | Done |
| Donate → [website donate page](https://linuxbased79.github.io/FixItGarage/donate.html); Feedback → Issues | Done |
| Smart date/mileage reminders + **boot re-register** | Done |
| Units (imperial / metric) | Done |
| Languages (system + EN/ES/FR/DE/JA/KO/ZH) | Done |
| OpenDyslexic accessibility font | Done |
| Receipt OCR (in-app from photo) | Done (on-device ocrs + external app fallback) |
| Camera tread auto-measure | Done (coin-gauge CV + manual confirm) |
| VIN + NHTSA safety recall check | Done (US public APIs) |
| Selling my car (seller PDF packet) | Done |
| F-Droid / Play public listing | **Assets + recipes ready** — account submit (see STORE.md) |

## Features (product goals)

- Unlimited vehicles  
- Setup wizard: mostly DIY, mostly shop, or both  
- Last service quick view  
- Receipt scanning with OCR → date, mileage, gallons, cost, parts, labor  
- Automatic MPG tracking  
- Full maintenance history (shop + DIY oil changes)  
- Tire tracker: receipt scan, camera tread depth, rotation log with **graphical top-down car diagram**, before/after preview, patterns, mileage per tire  
- Wiper, battery, and brake trackers with reminders  
- Parts log: engine air filter, cabin filter, oil filter, oil type + part numbers  
- Oil level check reminders every 3 months  
- Photo log for issues, notes, smart date/mileage reminders  
- Monthly and yearly operational cost tracker  
- Export CSV  
- Optional cloud backup providers listed above  
- Full dark mode  
- Donate button opens the [project donate page](https://linuxbased79.github.io/FixItGarage/donate.html); Send Feedback opens [GitHub Issues](https://github.com/linuxbased79/FixItGarage/issues)  
- Project website: [linuxbased79.github.io/FixItGarage](https://linuxbased79.github.io/FixItGarage/)

## GrapheneOS & F-Droid notes

- **Local-first**: data on device; cloud is optional and user-initiated.  
- Core app does **not** depend on Google Play Services.  
- Receipt OCR: **on-device** (ocrs, no GMS) with external OCR app fallback.  
- Tread: **coin-gauge CV** estimate; always confirm before save.  
- No proprietary trackers.  
- Store submit: [`STORE.md`](STORE.md), [`PLAY.md`](PLAY.md), [`F-DROID.md`](F-DROID.md).  
- Accounts (Play, GitLab, donations, website): [`ACCOUNTS_SETUP.md`](ACCOUNTS_SETUP.md).

## Build

### Rust (primary UI + core) — recommended

Requirements: Rust toolchain, Android NDK/SDK, [xbuild](https://github.com/rust-mobile/xbuild) (`x`), OpenJDK.

```bash
cd rust
export ANDROID_HOME=~/Android/Sdk
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/<version>
./scripts/release-apks.sh
# dist/FixItGarage-<ver>-arm64.apk   → phones
# dist/FixItGarage-<ver>-x86_64.apk  → emulator
```

Install on emulator:

```bash
source ~/Documents/FixItGarage/android-env.sh   # if you use that helper
fig-emulator   # if needed
fig-install
```

| Crate | What |
|-------|------|
| **`fixitgarage-ui`** | **Mobile/desktop GUI (Slint)** — main product UI |
| `fixitgarage-core` | Domain logic |
| `fixitgarage-cli` | CLI helpers |

Desktop UI:

```bash
cd rust
cargo run -p fixitgarage-ui
```

### Kotlin / Compose shell (reference only)

Older scaffold under `app/` — not the primary ship path.

```bash
./gradlew :app:assembleDebug
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[GPL-3.0-only](LICENSE)

OpenDyslexic fonts under SIL OFL — see `rust/fixitgarage-ui/ui/fonts/`.
