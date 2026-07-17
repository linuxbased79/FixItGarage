#!/usr/bin/env bash
# Download ocrs .rten models for on-device OCR.
set -euo pipefail
cd "$(dirname "$0")"
curl -L --fail -o text-detection.rten \
  "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten"
curl -L --fail -o text-recognition.rten \
  "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten"
ls -lh text-detection.rten text-recognition.rten
echo "Done."
