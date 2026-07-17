//! On-device OCR using pure-Rust [ocrs] (no Google Play Services).
//! Models: text-detection.rten + text-recognition.rten (~12 MB total).

use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const DET_NAME: &str = "text-detection.rten";
const REC_NAME: &str = "text-recognition.rten";
const DET_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const REC_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

static ENGINE: Mutex<Option<OcrEngine>> = Mutex::new(None);

/// Directory for OCR model files (app files / data dir).
pub fn models_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Ok(p) = crate::platform::android_files_dir_public() {
            return p.join("models");
        }
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fixitgarage")
        .join("models")
}

/// Ensure detection + recognition models exist on disk (extract / download).
pub fn ensure_models() -> Result<PathBuf, String> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("models dir: {e}"))?;
    let det = dir.join(DET_NAME);
    let rec = dir.join(REC_NAME);
    if det.is_file() && rec.is_file() && file_ok(&det, 100_000) && file_ok(&rec, 100_000) {
        return Ok(dir);
    }
    // Prefer bundled copy next to source / release assets
    try_copy_bundled(&dir)?;
    if det.is_file() && rec.is_file() {
        return Ok(dir);
    }
    // Last resort: download once (Graphene/stock with network)
    download_model(DET_URL, &det)?;
    download_model(REC_URL, &rec)?;
    Ok(dir)
}

fn file_ok(path: &Path, min: u64) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() >= min)
        .unwrap_or(false)
}

fn try_copy_bundled(dest_dir: &Path) -> Result<(), String> {
    // 1) Dev / release tree: fixitgarage-ui/models
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"),
        PathBuf::from("models"),
        PathBuf::from("fixitgarage-ui/models"),
        // Packaged next to state
        dest_dir.to_path_buf(),
    ];
    for src_root in &candidates {
        let sdet = src_root.join(DET_NAME);
        let srec = src_root.join(REC_NAME);
        if sdet.is_file() && srec.is_file() {
            let ddet = dest_dir.join(DET_NAME);
            let drec = dest_dir.join(REC_NAME);
            if sdet != ddet {
                let _ = std::fs::copy(&sdet, &ddet);
            }
            if srec != drec {
                let _ = std::fs::copy(&srec, &drec);
            }
            if file_ok(&ddet, 100_000) && file_ok(&drec, 100_000) {
                return Ok(());
            }
        }
    }
    // 2) Android assets extracted by packaging helper
    #[cfg(target_os = "android")]
    {
        if let Err(e) = crate::platform::extract_asset_to_file(
            &format!("models/{DET_NAME}"),
            &dest_dir.join(DET_NAME),
        ) {
            eprintln!("extract det model: {e}");
        }
        if let Err(e) = crate::platform::extract_asset_to_file(
            &format!("models/{REC_NAME}"),
            &dest_dir.join(REC_NAME),
        ) {
            eprintln!("extract rec model: {e}");
        }
    }
    Ok(())
}

fn download_model(url: &str, dest: &Path) -> Result<(), String> {
    if file_ok(dest, 100_000) {
        return Ok(());
    }
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .map_err(|e| format!("download {url}: {e}"))?;
    let mut reader = resp.into_reader();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut buf).map_err(|e| format!("read model: {e}"))?;
    if buf.len() < 100_000 {
        return Err(format!("model too small ({} bytes) from {url}", buf.len()));
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(dest, buf).map_err(|e| format!("write model: {e}"))?;
    Ok(())
}

fn get_engine() -> Result<(), String> {
    let mut guard = ENGINE
        .lock()
        .map_err(|_| "OCR engine lock poisoned".to_string())?;
    if guard.is_some() {
        return Ok(());
    }
    let dir = ensure_models()?;
    let det = Model::load_file(dir.join(DET_NAME)).map_err(|e| format!("load detection model: {e}"))?;
    let rec =
        Model::load_file(dir.join(REC_NAME)).map_err(|e| format!("load recognition model: {e}"))?;
    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(det),
        recognition_model: Some(rec),
        ..Default::default()
    })
    .map_err(|e| format!("OcrEngine::new: {e}"))?;
    *guard = Some(engine);
    Ok(())
}

/// Run on-device OCR on a JPEG/PNG image path. Returns plain text (newline-separated lines).
pub fn ocr_image_path(path: &str) -> Result<String, String> {
    let path = Path::new(path);
    if !path.is_file() {
        return Err(format!("image not found: {}", path.display()));
    }
    get_engine()?;
    let img = image::open(path)
        .map_err(|e| format!("open image: {e}"))?
        .into_rgb8();
    // Downscale very large photos for speed (receipts stay readable at ~1600px)
    let img = downscale_if_needed(img, 1600);
    let (w, h) = img.dimensions();
    let src = ImageSource::from_bytes(img.as_raw(), (w, h))
        .map_err(|e| format!("ImageSource: {e}"))?;

    let guard = ENGINE
        .lock()
        .map_err(|_| "OCR engine lock poisoned".to_string())?;
    let engine = guard.as_ref().ok_or("OCR engine not loaded")?;
    let input = engine
        .prepare_input(src)
        .map_err(|e| format!("prepare_input: {e}"))?;
    let text = engine
        .get_text(&input)
        .map_err(|e| format!("get_text: {e}"))?;
    let cleaned = text
        .lines()
        .map(str::trim)
        .filter(|l| l.len() > 1)
        .collect::<Vec<_>>()
        .join("\n");
    if cleaned.trim().is_empty() {
        return Err("No text detected in image. Try brighter light / flatter photo.".into());
    }
    Ok(cleaned)
}

fn downscale_if_needed(img: image::RgbImage, max_side: u32) -> image::RgbImage {
    let (w, h) = img.dimensions();
    let m = w.max(h);
    if m <= max_side {
        return img;
    }
    let scale = max_side as f32 / m as f32;
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
}

/// Drop cached engine (e.g. after model reinstall). Optional.
#[allow(dead_code)]
pub fn reset_engine() {
    if let Ok(mut g) = ENGINE.lock() {
        *g = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_dir_is_absolute_or_relative() {
        let d = models_dir();
        assert!(!d.as_os_str().is_empty());
    }
}
