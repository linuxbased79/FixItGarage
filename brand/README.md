# FixItGarage brand avatars & icons

Green top-down car + cyan wrench/gear on charcoal — matches the store listing icon and the in-app tire diagram.

## Quick pick

| Use | File |
|-----|------|
| **GitHub / social profile** | `github-avatar.png` (1024², padded square) |
| **Circular avatar** | `avatar-circle.png` |
| **App / store master** | `app-icon-1024.png` (same art as `docs/assets/icon.png`) |
| **Browser favicon** | `favicon.ico` |
| **Alt style** (side-view car) | `avatar-sideview-circle.png` |

Full size ladder (32–1024, favicons, Android adaptive layers) lives in:

`docs/assets/brand/`

Website already links `docs/assets/favicon.ico` and `docs/assets/avatar.png` (circle 512).

## Set as GitHub avatar

1. Open https://github.com/settings/profile  
2. Upload `brand/github-avatar.png` or `brand/avatar-circle.png`

## Android note

The adaptive launcher in the Kotlin sample still uses a simple vector wrench. The polished PNG foreground is at:

`docs/assets/brand/android-adaptive-foreground.png`

Wire it into the Rust/xbuild packaging when you next touch launcher resources.
