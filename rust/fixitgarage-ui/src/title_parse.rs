//! Parse vehicle title / registration OCR text into form fields.

use crate::recalls::normalize_vin;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default)]
pub struct TitleFields {
    pub vin: Option<String>,
    pub year: Option<u16>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub name_hint: Option<String>,
}

/// Extract VIN, year, make, model from free-form OCR of a title or registration.
pub fn parse_title_text(text: &str) -> TitleFields {
    let upper = text.to_ascii_uppercase();
    let mut out = TitleFields::default();

    // VIN: 17 chars, no I/O/Q
    out.vin = find_vin(&upper);

    // Year near VIN or standalone 19xx/20xx typical for model year
    out.year = find_year(&upper);

    // Make from known list
    out.make = find_make(&upper);

    // Model: try after make token, or common patterns
    if let Some(ref make) = out.make {
        out.model = find_model_after_make(&upper, make);
    }
    if out.model.is_none() {
        out.model = find_model_generic(&upper, out.make.as_deref());
    }

    // Nickname hint from year + make + model
    let mut parts = Vec::new();
    if let Some(y) = out.year {
        parts.push(y.to_string());
    }
    if let Some(ref m) = out.make {
        parts.push(m.clone());
    }
    if let Some(ref m) = out.model {
        parts.push(m.clone());
    }
    if !parts.is_empty() {
        out.name_hint = Some(parts.join(" "));
    }

    out
}

fn find_vin(upper: &str) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();

    // Prefer tokens next to "VIN" label
    static NEAR: OnceLock<Regex> = OnceLock::new();
    let near = NEAR.get_or_init(|| {
        Regex::new(r"VIN[:\s#]*([A-HJ-NPR-Z0-9 \-]{17,23})").expect("vin near")
    });
    if let Some(c) = near.captures(upper) {
        let raw: String = c
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or("")
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect();
        if let Ok(v) = normalize_vin(&raw) {
            candidates.push(v);
        }
    }

    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[A-HJ-NPR-Z0-9]{17}").expect("vin re"));
    for m in re.find_iter(upper) {
        if let Ok(v) = normalize_vin(m.as_str()) {
            candidates.push(v);
        }
    }

    // Compact OCR (spaces removed between lines)
    let compact: String = upper
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let chars: Vec<char> = compact.chars().collect();
    if chars.len() >= 17 {
        for i in 0..=chars.len() - 17 {
            let s: String = chars[i..i + 17].iter().collect();
            if let Ok(v) = normalize_vin(&s) {
                candidates.push(v);
            }
        }
    }

    // Prefer check-digit valid VINs; skip ones that look like words
    candidates.retain(|v| {
        !v.contains("NUMBER")
            && !v.contains("VEHICLE")
            && !v.starts_with("IDENTIF")
            && !v.contains("TITLE")
    });
    if let Some(v) = candidates.iter().find(|v| vin_check_digit_ok(v)) {
        return Some(v.clone());
    }
    candidates.into_iter().next()
}

/// ISO 3779 VIN check digit (position 9).
fn vin_check_digit_ok(vin: &str) -> bool {
    if vin.len() != 17 {
        return false;
    }
    let weights: [u32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];
    let mut sum = 0u32;
    for (i, c) in vin.chars().enumerate() {
        let val = match c {
            '0'..='9' => c.to_digit(10).unwrap_or(0),
            'A' | 'J' => 1,
            'B' | 'K' | 'S' => 2,
            'C' | 'L' | 'T' => 3,
            'D' | 'M' | 'U' => 4,
            'E' | 'N' | 'V' => 5,
            'F' | 'W' => 6,
            'G' | 'P' | 'X' => 7,
            'H' | 'Y' => 8,
            'R' | 'Z' => 9,
            _ => return false,
        };
        sum += val * weights[i];
    }
    let check = sum % 11;
    let expected = if check == 10 {
        'X'
    } else {
        char::from_digit(check, 10).unwrap_or('?')
    };
    vin.chars().nth(8) == Some(expected)
}

fn find_year(upper: &str) -> Option<u16> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\b(19[8-9]\d|20[0-3]\d)\b").expect("year re"));
    // Prefer years near "YEAR" / "MODEL YEAR" / "YR"
    let lines: Vec<&str> = upper.lines().collect();
    for line in &lines {
        if line.contains("YEAR") || line.contains(" YR") || line.contains("MODEL Y") {
            if let Some(m) = re.find(line) {
                if let Ok(y) = m.as_str().parse::<u16>() {
                    if (1980..=2035).contains(&y) {
                        return Some(y);
                    }
                }
            }
        }
    }
    // Fallback: first plausible year in text
    for m in re.find_iter(upper) {
        if let Ok(y) = m.as_str().parse::<u16>() {
            if (1980..=2035).contains(&y) {
                return Some(y);
            }
        }
    }
    None
}

const MAKES: &[&str] = &[
    "TOYOTA",
    "HONDA",
    "FORD",
    "CHEVROLET",
    "CHEVY",
    "GMC",
    "NISSAN",
    "HYUNDAI",
    "KIA",
    "SUBARU",
    "MAZDA",
    "BMW",
    "MERCEDES",
    "MERCEDES-BENZ",
    "AUDI",
    "VOLKSWAGEN",
    "VW",
    "JEEP",
    "RAM",
    "DODGE",
    "CHRYSLER",
    "LEXUS",
    "ACURA",
    "INFINITI",
    "VOLVO",
    "TESLA",
    "PORSCHE",
    "LAND ROVER",
    "JAGUAR",
    "MITSUBISHI",
    "BUICK",
    "CADILLAC",
    "LINCOLN",
    "MINI",
    "FIAT",
    "GENESIS",
    "RIVIAN",
    "LUCID",
];

fn find_make(upper: &str) -> Option<String> {
    // Longer names first
    let mut sorted: Vec<&str> = MAKES.to_vec();
    sorted.sort_by_key(|m| std::cmp::Reverse(m.len()));
    for m in sorted {
        if upper.contains(m) {
            // Normalize display
            if m == "CHEVY" {
                return Some("Chevrolet".into());
            }
            if m == "VW" {
                return Some("Volkswagen".into());
            }
            if m == "MERCEDES" || m == "MERCEDES-BENZ" {
                return Some("Mercedes-Benz".into());
            }
            // Title case
            let mut c = m.chars();
            let s = match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str().to_ascii_lowercase().as_str(),
                None => continue,
            };
            // Multi-word
            if m.contains(' ') || m.contains('-') {
                return Some(
                    m.split(|c: char| c == ' ' || c == '-')
                        .map(|w| {
                            let mut c = w.chars();
                            match c.next() {
                                Some(f) => {
                                    f.to_uppercase().collect::<String>()
                                        + c.as_str().to_ascii_lowercase().as_str()
                                }
                                None => String::new(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(if m.contains('-') { "-" } else { " " }),
                );
            }
            return Some(s);
        }
    }
    None
}

fn find_model_after_make(upper: &str, make: &str) -> Option<String> {
    let make_u = make.to_ascii_uppercase();
    let make_u = if make_u == "CHEVROLET" {
        // also try CHEVY
        "CHEVROLET"
    } else {
        make_u.as_str()
    };
    let idx = upper.find(make_u).or_else(|| {
        if make.eq_ignore_ascii_case("Chevrolet") {
            upper.find("CHEVY")
        } else {
            None
        }
    })?;
    let after = &upper[idx + make_u.len()..];
    // Next 1–3 tokens that look like model (letters/numbers)
    let mut tokens = Vec::new();
    for tok in after.split(|c: char| !c.is_ascii_alphanumeric() && c != '-') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) {
            // year — stop
            break;
        }
        if matches!(
            t,
            "VIN"
                | "VEHICLE"
                | "IDENTIFICATION"
                | "NUMBER"
                | "TITLE"
                | "OF"
                | "THE"
                | "STATE"
                | "OWNER"
                | "MAKE"
                | "MODEL"
                | "YEAR"
                | "BODY"
                | "TYPE"
        ) {
            if !tokens.is_empty() {
                break;
            }
            continue;
        }
        if t.len() >= 1 && t.len() <= 20 {
            tokens.push(t);
        }
        if tokens.len() >= 2 {
            break;
        }
    }
    if tokens.is_empty() {
        return None;
    }
    // Title case
    let model = tokens
        .iter()
        .map(|t| {
            let mut c = t.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str().to_ascii_lowercase().as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Some(model)
}

fn find_model_generic(upper: &str, make: Option<&str>) -> Option<String> {
    // Common patterns: MODEL: FORTE  or  MODEL FORTE
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"MODEL[:\s]+([A-Z0-9][A-Z0-9 \-]{1,24})").expect("model re")
    });
    if let Some(c) = re.captures(upper) {
        let mut m = c.get(1)?.as_str().trim().to_string();
        // cut at year
        if let Some(pos) = m.find(|c: char| c.is_ascii_digit()) {
            if m[pos..].len() >= 4 {
                m = m[..pos].trim().to_string();
            }
        }
        if m.len() >= 2 {
            if let Some(make) = make {
                let mu = make.to_ascii_uppercase();
                if m.starts_with(&mu) {
                    m = m[mu.len()..].trim().to_string();
                }
            }
            let title = m
                .split_whitespace()
                .take(2)
                .map(|t| {
                    let mut c = t.chars();
                    match c.next() {
                        Some(f) => {
                            f.to_uppercase().collect::<String>()
                                + c.as_str().to_ascii_lowercase().as_str()
                        }
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            if title.len() >= 2 {
                return Some(title);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_title() {
        let t = r#"
            STATE OF TEXAS CERTIFICATE OF TITLE
            YEAR 2024 MAKE KIA MODEL FORTE
            VEHICLE IDENTIFICATION NUMBER
            3KPF54ADXRE777701
        "#;
        let f = parse_title_text(t);
        assert_eq!(f.vin.as_deref(), Some("3KPF54ADXRE777701"));
        assert_eq!(f.year, Some(2024));
        assert_eq!(f.make.as_deref(), Some("Kia"));
        assert!(f.model.as_ref().map(|m| m.to_ascii_uppercase().contains("FORTE")).unwrap_or(false));
    }
}
