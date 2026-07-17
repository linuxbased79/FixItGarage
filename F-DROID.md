# F-Droid — submit FixItGarage

Canonical notes also live in [`rust/F-DROID.md`](rust/F-DROID.md).

## Package

| Field | Value |
|-------|--------|
| App ID | `org.fixitgarage.app` |
| License | GPL-3.0-only |
| Source | https://github.com/linuxbased79/FixItGarage |
| Metadata draft | [`metadata/org.fixitgarage.app.yml`](metadata/org.fixitgarage.app.yml) |
| Listing text | [`metadata/en-US/`](metadata/en-US/) |
| Screenshots | `metadata/en-US/images/phoneScreenshots/` |

## Maintainer build recipe

On an F-Droid build server (NDK + SDK + Java + Rust):

```bash
git clone https://github.com/linuxbased79/FixItGarage.git
cd FixItGarage && git checkout v0.2.20
cd rust
./fixitgarage-ui/models/download-models.sh
./scripts/release-apks.sh 0.2.20 2020
# → dist/FixItGarage-0.2.20-arm64.apk
```

Packaging injects BootReceiver + ShareReceiveActivity, OCR models, and correctly stores `resources.arsc` (API 30+ install requirement).

## Submit to fdroiddata

1. Fork https://gitlab.com/fdroid/fdroiddata  
2. Copy `metadata/org.fixitgarage.app.yml` into the fork’s `metadata/`  
3. Ensure `commit: v0.2.20` matches a **signed/published tag** on GitHub  
4. Copy or link `metadata/en-US/*` listing files as required by current fdroiddata layout  
5. Locally: `fdroid build org.fixitgarage.app` (or rely on CI after MR)  
6. Open a merge request with build logs  

## AntiFeatures

- Optional **NonFreeNet** only if documenting user-chosen third-party WebDAV hosts.  
- No proprietary GMS/ML Kit dependency.

## Status

Metadata, screenshots, and reproducible build recipe are in this repo. Merge into **fdroiddata** requires GitLab + F-Droid review (account action).
