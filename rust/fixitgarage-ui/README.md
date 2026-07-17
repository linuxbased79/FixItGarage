# FixItGarage UI (Rust + Slint)

Touch-friendly **mobile-first** UI for FixItGarage, written in **Rust** with [Slint](https://slint.dev).

- Uses `fixitgarage-core` for MPG, tire rotation, costs, and CSV  
- Local-first JSON persistence (`~/.local/share/fixitgarage/state.json` on Linux)  
- Phone-sized window defaults (390×780) with bottom navigation  
- **Android-ready** via Slint’s `backend-android-activity-06` + `android_main`  
- License: **GPL-3.0** (compatible with Slint’s GPL option for FOSS apps)

## Screens

| Screen | Features |
|--------|----------|
| Setup wizard | DIY / shop / both |
| Home | Last service, vehicle count, quick actions |
| Vehicles | Unlimited vehicles form + list |
| Service | DIY/shop log, gallons for MPG, history |
| Tires | Top-down diagram, rotation patterns, apply |
| Costs | Month / year / all-time rollups |
| Settings | Mode, CSV export, donate, GitHub feedback |

## Desktop (Linux / Windows / macOS)

```bash
cd rust
cargo run -p fixitgarage-ui
# or release:
cargo run -p fixitgarage-ui --release
```

Requires a graphical session (Wayland or X11). Build-only (no display):

```bash
cargo build -p fixitgarage-ui --release
```

## Android (GrapheneOS / F-Droid oriented)

Slint apps ship as a native `cdylib` loaded by `NativeActivity`.

### 1. Toolchain

```bash
rustup target add aarch64-linux-android
# optional emulators:
rustup target add x86_64-linux-android

export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/<version>
```

### 2. Install xbuild (recommended by Slint docs)

```bash
cargo install --git https://github.com/rust-mobile/xbuild.git
```

### 3. Build / run

```bash
cd rust/fixitgarage-ui
x devices
x run --device <id>
# APK:
x build --platform android --arch arm64 --format apk --release
# → target/x/release/android/*.apk
```

Alternate (cargo-apk):

```bash
cargo install cargo-apk
cargo apk run --target aarch64-linux-android --lib
```

### Package id

Use application id **`org.fixitgarage.app`** when packaging for F-Droid (same as the Kotlin shell).

## Architecture

```
fixitgarage-ui (Slint UI + JSON store)
        │
        ▼
fixitgarage-core (pure domain logic)
```

The older **Kotlin Compose** app under `/app` remains as a reference shell. The Rust UI is the primary pure-Rust product path going forward.

## Notes

- Receipt OCR, camera tread depth, and cloud backup are still roadmap items.  
- On Android, “open URL” for feedback/donate can be extended via JNI if needed.  
- First Slint build is slow (native deps); later builds are incremental.
