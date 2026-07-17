# Store release status (0.2.20)

## Done in this repository

| Item | Location |
|------|----------|
| Installable APK packaging (resources.arsc stored, libs page-aligned) | `rust/scripts/package-apk-with-boot.sh` |
| BootReceiver + share-target OCR + on-device models | APK packaging |
| Upload keystore helper | `rust/scripts/create-upload-keystore.sh` |
| Privacy policy (Play URL-ready) | `PRIVACY.md` → raw.githubusercontent.com/…/PRIVACY.md |
| F-Droid metadata draft | `metadata/org.fixitgarage.app.yml` |
| Play listing text | `metadata/en-US/*` |
| Phone screenshots | `metadata/en-US/images/phoneScreenshots/` |
| Feature graphic 1024×500 | `metadata/en-US/images/featureGraphic/` |
| High-res icon 512×512 | `metadata/en-US/images/icon/` |
| Play / F-Droid guides | `PLAY.md`, `F-DROID.md` |
| GitHub release APKs | tag `v0.2.20` (when published) |

## What only you can finish (accounts)

### Google Play (≈30–60 min once account exists)

1. [Play Console](https://play.google.com/console) → create app  
2. Create upload keystore (`./rust/scripts/create-upload-keystore.sh`)  
3. `FIG_KEYSTORE=… ./rust/scripts/release-apks.sh 0.2.20 2020`  
4. Upload `dist/FixItGarage-0.2.20-arm64.apk` to Internal testing  
5. Paste privacy URL, descriptions, screenshots, feature graphic from `metadata/`  
6. Complete Data safety + content rating → promote to production when ready  

### F-Droid

1. Fork [fdroiddata](https://gitlab.com/fdroid/fdroiddata)  
2. Add `metadata/org.fixitgarage.app.yml` (from this repo)  
3. Open MR pointing at tag **v0.2.20**  
4. Respond to review / fix reproducible build notes if asked  

## Prebuilt APKs (debug-signed, for sideload / Graphene)

After release build:

- `rust/dist/FixItGarage-0.2.20-arm64.apk`  
- `rust/dist/FixItGarage-0.2.20-x86_64.apk`  
- Also copied to `~/Downloads/` when built in this environment  

**Do not** upload debug-signed APKs to Play production — use your upload keystore.
