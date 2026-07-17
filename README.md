# FixItGarage

**Open-source vehicle maintenance tracker for Android**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

FixItGarage helps you track **unlimited vehicles**, service history (DIY + shop), tires and rotations, filters/oil part numbers, brakes, battery, wipers, costs, reminders, photos, and notes — with optional cloud backup later.

Planned distribution: **F-Droid** and **Google Play**. Designed to run well on **GrapheneOS** (local-first, no Google Play Services required for core features).

## Status

**0.1.0-alpha** — project scaffold with working local data layer and UI shells for the full feature set.

| Area | Status |
|------|--------|
| Setup wizard (DIY / shop / both) | Implemented |
| Unlimited vehicles | Implemented (Room) |
| Last service on home | Implemented |
| Maintenance history (DIY + shop) | Implemented |
| Automatic MPG helper | Core logic + unit test |
| Tire top-down diagram + rotation preview | Implemented (UI) |
| Monthly / yearly costs | Implemented from service data |
| Dark mode (system / light / dark) | Implemented |
| CSV export | Implemented for service records |
| Donate + Send Feedback → GitHub Issues | Implemented |
| Receipt OCR | UI scaffold (on-device OCR next) |
| Camera tread depth | Planned |
| Cloud sync (Proton / GDrive / Dropbox / OneDrive / ownCloud / Nextcloud) | Planned (optional) |
| Parts / brakes / battery / wipers / photos / notes / reminders | Data model + UI placeholders |

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
- Donate button; Send Feedback opens [GitHub Issues](https://github.com/linuxbased79/FixItGarage/issues)

## GrapheneOS & F-Droid notes

- **Local-first**: Room database on device; cloud is optional.  
- Core app does **not** depend on Google Play Services.  
- OCR and camera use AndroidX CameraX; F-Droid builds will prefer free on-device OCR.  
- No proprietary trackers or closed binary blobs in the free flavor.

## Build

### Android (Kotlin / Compose)

Requirements: Android Studio Ladybug+ or JDK 17, Android SDK 35.

```bash
./gradlew :app:assembleDebug
```

Install the debug APK from `app/build/outputs/apk/debug/`.

### Rust (core logic + CLI)

Domain logic also lives in pure Rust under [`rust/`](rust/) — builds with only `rustup` (no Android SDK).

```bash
cd rust
cargo test
cargo build --release
./target/release/fixitgarage mpg 10000:10 10300:10 10580:10
./target/release/fixitgarage rotate forward_cross
```

See [rust/README.md](rust/README.md).

## License

[GNU General Public License v3.0](LICENSE) — free software: you can redistribute and/or modify it under the GPL-3.0.

## Donate

Support development if you find FixItGarage useful. Prefer open-source-friendly options (to be linked here: Liberapay / Open Collective / etc.).

## Feedback

Bugs and feature requests: [github.com/linuxbased79/FixItGarage/issues](https://github.com/linuxbased79/FixItGarage/issues)

## Privacy

By default, data stays on your device. See [PRIVACY.md](PRIVACY.md).
