# FixItGarage — Rust workspace

Pure **Rust** domain logic and CLI for FixItGarage (GPL-3.0).

The Android UI remains Kotlin/Compose under `../app`. This crate holds
shared algorithms so they can be tested without the Android SDK and later
exposed to Android via UniFFI/JNI if desired.

## Crates

| Crate | Role |
|-------|------|
| `fixitgarage-core` | MPG, tire rotation, CSV export, cost rollups, reminders |
| `fixitgarage-cli` | `fixitgarage` binary for local tools |

## Build & test

```bash
cd rust
cargo build --release
cargo test
```

## CLI examples

```bash
# Average MPG from fill-ups (odometer:gallons)
cargo run -p fixitgarage-cli -- mpg 10000:10 10300:10 10580:10

# Tire rotation preview
cargo run -p fixitgarage-cli -- rotate forward_cross --fl A --fr B --rl C --rr D

# Sample costs + CSV
cargo run -p fixitgarage-cli -- demo
```

## Why Rust here?

- Fast, memory-safe core logic  
- Unit tests without Android Studio  
- Future option: bind the same core into the Android app  
- Builds on GrapheneOS-friendly developer machines with only `rustup`

Android APK packaging still uses Gradle (`../gradlew`). This does **not** replace the mobile app binary yet.
