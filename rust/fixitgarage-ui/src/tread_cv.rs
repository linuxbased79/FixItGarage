//! On-device tread depth assist from a photo of a coin in the tread groove.
//!
//! Method (no GMS):
//! 1. Detect the largest roughly circular blob (US penny / coin).
//! 2. Scale: coin diameter ≈ 19.05 mm (US one-cent).
//! 3. Sample intensity along radial spokes; estimate how deeply the coin
//!    sits in the groove → approximate tread depth in mm.
//!
//! Always treat as assist — user should confirm before saving.

use image::{GrayImage, Luma, RgbImage};
use imageproc::filter::gaussian_blur_f32;
use imageproc::map::map_colors;
use std::path::Path;

/// Result of coin-gauge computer vision.
#[derive(Debug, Clone)]
pub struct TreadEstimate {
    /// Estimated tread depth in millimetres.
    pub depth_mm: f64,
    /// 0.0–1.0 rough confidence.
    pub confidence: f64,
    #[allow(dead_code)]
    pub coin_found: bool,
    #[allow(dead_code)]
    pub coin_diameter_px: f64,
    pub note: String,
}

/// US penny diameter (mm) — primary reference for North America coin gauge.
const PENNY_DIAMETER_MM: f64 = 19.05;
/// Legal / wear reference often cited with coin gauge (~2/32").
#[allow(dead_code)]
const WEAR_LIMIT_MM: f64 = 1.6;

/// Estimate tread depth from an image path (JPEG/PNG).
pub fn estimate_tread_from_image(path: &str) -> Result<TreadEstimate, String> {
    let path = Path::new(path);
    if !path.is_file() {
        return Err(format!("image not found: {}", path.display()));
    }
    let rgb = image::open(path)
        .map_err(|e| format!("open image: {e}"))?
        .into_rgb8();
    estimate_tread_rgb(&rgb)
}

/// Core estimator (testable with synthetic images).
pub fn estimate_tread_rgb(rgb: &RgbImage) -> Result<TreadEstimate, String> {
    let (w, h) = rgb.dimensions();
    if w < 64 || h < 64 {
        return Err("Image too small for tread CV.".into());
    }
    // Work on a manageable size
    let rgb = downscale(rgb, 960);
    let gray = to_gray(&rgb);
    let blur = gaussian_blur_f32(&gray, 1.5);

    let coin = detect_coin(&blur).ok_or_else(|| {
        "No coin found. Place a US penny (or similar coin) in the tread groove, fill the frame, and retry.".to_string()
    })?;

    let scale_mm_per_px = PENNY_DIAMETER_MM / coin.diameter_px.max(1.0);
    let depth_px = estimate_groove_depth_px(&blur, &coin);
    let mut depth_mm = depth_px * scale_mm_per_px;

    // Clamp to physical range for passenger tires
    depth_mm = depth_mm.clamp(0.3, 16.0);

    // Confidence: stronger circle score + mid-range depth
    let conf = (coin.score * 0.7 + if (1.0..10.0).contains(&depth_mm) {
        0.3
    } else {
        0.1
    })
    .clamp(0.05, 0.95);

    let note = format!(
        "Coin Ø≈{:.0}px → {:.2} mm/px. Estimated tread {:.1} mm (verify with gauge).",
        coin.diameter_px, scale_mm_per_px, depth_mm
    );

    Ok(TreadEstimate {
        depth_mm,
        confidence: conf,
        coin_found: true,
        coin_diameter_px: coin.diameter_px,
        note,
    })
}

#[derive(Clone, Debug)]
struct Coin {
    cx: f64,
    cy: f64,
    diameter_px: f64,
    score: f64,
}

fn to_gray(rgb: &RgbImage) -> GrayImage {
    map_colors(rgb, |p| {
        let y = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8;
        Luma([y])
    })
}

fn downscale(rgb: &RgbImage, max_side: u32) -> RgbImage {
    let (w, h) = rgb.dimensions();
    let m = w.max(h);
    if m <= max_side {
        return rgb.clone();
    }
    let scale = max_side as f32 / m as f32;
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    image::imageops::resize(rgb, nw, nh, image::imageops::FilterType::Triangle)
}

/// Detect largest circular object via multi-radius edge ring scoring.
fn detect_coin(gray: &GrayImage) -> Option<Coin> {
    let (w, h) = gray.dimensions();
    let min_r = (w.min(h) as f64 * 0.06).max(12.0);
    let max_r = (w.min(h) as f64 * 0.42).min(w.min(h) as f64 / 2.0 - 2.0);
    if max_r <= min_r + 4.0 {
        return None;
    }

    // Gradient magnitude (simple Sobel-ish)
    let mag = gradient_mag(gray);

    let mut best: Option<Coin> = None;
    let step_r = ((max_r - min_r) / 18.0).max(2.0);
    let step_xy = ((w.min(h) as f64) / 40.0).max(4.0) as u32;

    let mut r = min_r;
    while r <= max_r {
        let mut y = (r as u32) + 2;
        while y + (r as u32) + 2 < h {
            let mut x = (r as u32) + 2;
            while x + (r as u32) + 2 < w {
                let score = ring_score(&mag, x as f64, y as f64, r);
                if score > 0.15 {
                    let cand = Coin {
                        cx: x as f64,
                        cy: y as f64,
                        diameter_px: r * 2.0,
                        score,
                    };
                    if best.as_ref().map(|b| cand.score > b.score).unwrap_or(true) {
                        best = Some(cand);
                    }
                }
                x += step_xy;
            }
            y += step_xy;
        }
        r += step_r;
    }
    // Require a minimum score so noise doesn't invent coins
    best.filter(|c| c.score >= 0.22)
}

fn gradient_mag(gray: &GrayImage) -> GrayImage {
    let (w, h) = gray.dimensions();
    let mut out = GrayImage::new(w, h);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let gx = gray.get_pixel(x + 1, y)[0] as i16 - gray.get_pixel(x - 1, y)[0] as i16;
            let gy = gray.get_pixel(x, y + 1)[0] as i16 - gray.get_pixel(x, y - 1)[0] as i16;
            let m = ((gx as i32 * gx as i32 + gy as i32 * gy as i32) as f32)
                .sqrt()
                .min(255.0) as u8;
            out.put_pixel(x, y, Luma([m]));
        }
    }
    out
}

/// Average gradient on a circle ring — high for real coin edges.
fn ring_score(mag: &GrayImage, cx: f64, cy: f64, r: f64) -> f64 {
    let (w, h) = mag.dimensions();
    let n = 36;
    let mut sum = 0.0f64;
    let mut count = 0.0f64;
    for i in 0..n {
        let a = (i as f64) * std::f64::consts::TAU / (n as f64);
        let x = (cx + r * a.cos()).round() as i64;
        let y = (cy + r * a.sin()).round() as i64;
        if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
            sum += mag.get_pixel(x as u32, y as u32)[0] as f64;
            count += 1.0;
        }
    }
    if count < (n as f64) * 0.7 {
        return 0.0;
    }
    // Normalize ~0..1 (edges often 30–120)
    (sum / count / 90.0).clamp(0.0, 1.5)
}

/// Estimate groove depth in pixels near the coin (dark channel under/around coin).
fn estimate_groove_depth_px(gray: &GrayImage, coin: &Coin) -> f64 {
    let (w, h) = gray.dimensions();
    let r = coin.diameter_px / 2.0;
    // Sample spokes from center outward; find first bright rubber wall after dark groove
    let n_angles = 16;
    let mut depths = Vec::new();
    for i in 0..n_angles {
        let a = (i as f64) * std::f64::consts::TAU / (n_angles as f64);
        // Start slightly inside coin edge
        let mut prev = sample(gray, coin.cx, coin.cy, w, h);
        let mut in_dark = false;
        let mut dark_start = 0.0f64;
        let mut max_run = 0.0f64;
        let steps = (r * 1.8) as i32;
        for s in 0..steps {
            let dist = s as f64;
            let x = coin.cx + dist * a.cos();
            let y = coin.cy + dist * a.sin();
            let v = sample(gray, x, y, w, h);
            let dark = v < 90.0;
            if dark && !in_dark {
                in_dark = true;
                dark_start = dist;
            } else if !dark && in_dark {
                let run = dist - dark_start;
                if run > max_run && dist > r * 0.4 {
                    max_run = run;
                }
                in_dark = false;
            }
            // Transition coin body → groove (bright metal → dark rubber gap)
            if dist > r * 0.7 && dist < r * 1.35 && prev > 110.0 && v < 80.0 {
                // groove entry
            }
            prev = v;
        }
        if max_run > 1.0 {
            depths.push(max_run);
        }
    }
    if depths.is_empty() {
        // Fallback: use local contrast near coin rim as weak depth proxy
        return (r * 0.12).clamp(2.0, r * 0.4);
    }
    depths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Median dark-run width ≈ groove opening; map conservatively to depth
    let med = depths[depths.len() / 2];
    // Groove width is not equal to depth; scale heuristically (~0.45)
    (med * 0.45).clamp(2.0, r * 0.55)
}

fn sample(gray: &GrayImage, x: f64, y: f64, w: u32, h: u32) -> f64 {
    let xi = x.round() as i64;
    let yi = y.round() as i64;
    if xi < 0 || yi < 0 || xi as u32 >= w || yi as u32 >= h {
        return 128.0;
    }
    gray.get_pixel(xi as u32, yi as u32)[0] as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn synthetic_coin_image_yields_estimate() {
        // Gray background + bright circle (coin)
        let mut img = RgbImage::from_pixel(400, 400, Rgb([80, 80, 80]));
        let cx = 200i32;
        let cy = 200i32;
        let r = 70i32;
        for y in 0..400 {
            for x in 0..400 {
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                let d2 = dx * dx + dy * dy;
                if d2 <= r * r {
                    // metallic coin
                    img.put_pixel(x, y, Rgb([200, 190, 160]));
                }
                // dark groove ring just outside coin (partial)
                if d2 > r * r && d2 < (r + 12) * (r + 12) && dx > 0 {
                    img.put_pixel(x, y, Rgb([30, 30, 30]));
                }
            }
        }
        let est = estimate_tread_rgb(&img).expect("should find coin");
        assert!(est.coin_found);
        assert!(est.depth_mm > 0.3 && est.depth_mm < 16.0, "depth={}", est.depth_mm);
        assert!(est.coin_diameter_px > 50.0, "diam={}", est.coin_diameter_px);
    }

    #[test]
    fn blank_image_fails() {
        let img = RgbImage::from_pixel(200, 200, Rgb([128, 128, 128]));
        assert!(estimate_tread_rgb(&img).is_err());
    }
}
