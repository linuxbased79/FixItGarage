#!/usr/bin/env bash
# Download ocrs .rten models for on-device OCR (hash-pinned, no redirects).
set -euo pipefail
cd "$(dirname "$0")"
DET_SHA="f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca"
REC_SHA="e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e"
curl --fail --proto '=https' --tlsv1.2 --max-redirs 0 -o text-detection.rten \
  "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten"
curl --fail --proto '=https' --tlsv1.2 --max-redirs 0 -o text-recognition.rten \
  "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten"
echo "${DET_SHA}  text-detection.rten" | sha256sum -c -
echo "${REC_SHA}  text-recognition.rten" | sha256sum -c -
ls -lh text-detection.rten text-recognition.rten
echo "Done."
