# On-device OCR models (ocrs)

Download the pure-Rust OCR models (≈12 MB total, no Google Play Services):

```bash
./download-models.sh
```

Files:

| File | Purpose |
|------|---------|
| `text-detection.rten` | Word/region detector |
| `text-recognition.rten` | Line text recognition |

Source: [ocrs-models](https://ocrs-models.s3-accelerate.amazonaws.com/) (MIT/Apache-friendly ocrs project).

These are packaged into the Android APK as `assets/models/*` by `scripts/package-apk-with-boot.sh`.
