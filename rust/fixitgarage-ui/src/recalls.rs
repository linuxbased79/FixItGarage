//! NHTSA recall lookup (US, free public APIs — no API key, no GMS).
//!
//! Flow:
//! 1. Decode VIN via vPIC → make / model / year (and fill vehicle if empty)
//! 2. Query recalls by make/model/year
//! 3. Optionally open NHTSA consumer VIN page for open-recall status on that VIN

use serde::Deserialize;
use std::io::Read;
use std::time::Duration;

const VIN_DECODE: &str = "https://vpic.nhtsa.dot.gov/api/vehicles/DecodeVinValues";
const RECALLS_BY_VEHICLE: &str = "https://api.nhtsa.gov/recalls/recallsByVehicle";
/// Consumer site VIN search (opens in browser for official open-recall status).
pub const NHTSA_VIN_RECALLS_WEB: &str = "https://www.nhtsa.gov/recalls?vin=";

#[derive(Debug, Clone)]
pub struct DecodedVin {
    pub make: String,
    pub model: String,
    pub year: Option<u16>,
    pub clean: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RecallItem {
    pub campaign: String,
    pub component: String,
    pub summary: String,
    pub consequence: String,
    pub remedy: String,
    pub report_date: String,
    pub manufacturer: String,
}

#[derive(Debug, Clone)]
pub struct RecallCheckResult {
    pub decoded: DecodedVin,
    pub recalls: Vec<RecallItem>,
    #[allow(dead_code)]
    pub note: String,
}

/// Normalize VIN: uppercase, strip spaces/dashes; validate length 17.
pub fn normalize_vin(raw: &str) -> Result<String, String> {
    let v: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if v.len() != 17 {
        return Err(format!(
            "VIN must be 17 characters (got {}). Check for typos.",
            v.len()
        ));
    }
    // VIN never uses I, O, Q
    if v.chars().any(|c| matches!(c, 'I' | 'O' | 'Q')) {
        return Err("VIN cannot contain I, O, or Q.".into());
    }
    Ok(v)
}

pub fn nhtsa_vin_web_url(vin: &str) -> String {
    format!("{NHTSA_VIN_RECALLS_WEB}{vin}")
}

/// Full check: decode VIN + list recalls for that YMM.
pub fn check_recalls_for_vin(vin_raw: &str) -> Result<RecallCheckResult, String> {
    let vin = normalize_vin(vin_raw)?;
    let decoded = decode_vin(&vin)?;
    if decoded.make.is_empty() || decoded.model.is_empty() {
        return Err(format!(
            "Could not decode make/model from VIN. NHTSA said: {}",
            decoded.message
        ));
    }
    let year = decoded
        .year
        .ok_or_else(|| "Could not decode model year from VIN.".to_string())?;
    // Never fail the whole check if the campaigns list is empty or the API returns 400.
    let (recalls, list_note) = match fetch_recalls(&decoded.make, &decoded.model, year) {
        Ok(r) => (r, String::new()),
        Err(e) => (
            Vec::new(),
            format!(" Campaign list unavailable ({e}). Use Open NHTSA VIN page for official status."),
        ),
    };
    let note = format!(
        "NHTSA data for {} {} {} (from VIN). {} campaign(s). Open-recall status for this exact VIN: nhtsa.gov/recalls.{}",
        year,
        decoded.make,
        decoded.model,
        recalls.len(),
        list_note
    );
    Ok(RecallCheckResult {
        decoded,
        recalls,
        note,
    })
}

/// Check by year/make/model without VIN (fallback).
pub fn check_recalls_ymm(make: &str, model: &str, year: u16) -> Result<Vec<RecallItem>, String> {
    if make.trim().is_empty() || model.trim().is_empty() || year < 1960 {
        return Err("Need make, model, and year.".into());
    }
    fetch_recalls(make, model, year)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .user_agent("FixItGarage/0.2.38 (Android; NHTSA recall check)")
        .build()
}

fn decode_vin(vin: &str) -> Result<DecodedVin, String> {
    let url = format!("{VIN_DECODE}/{vin}?format=json");
    let resp = agent()
        .get(&url)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| {
            // Surface common Android / offline failures clearly.
            let s = e.to_string();
            if s.contains("timed out") || s.contains("Timeout") {
                format!("VIN decode timed out — check Wi‑Fi/mobile data and try again.")
            } else if s.contains("failed to lookup")
                || s.contains("Dns")
                || s.contains("Name or service")
                || s.contains("Network is unreachable")
            {
                format!("No network for VIN decode — enable internet and try again. ({s})")
            } else {
                format!("VIN decode network: {s}")
            }
        })?;
    let body: VinDecodeResponse = serde_json::from_reader(resp.into_reader())
        .map_err(|e| format!("VIN decode JSON: {e}"))?;
    let row = body
        .results
        .into_iter()
        .next()
        .ok_or_else(|| "Empty VIN decode response.".to_string())?;

    let err = row.error_code.unwrap_or_default();
    // ErrorCode can be "0" or "0,..." — first number 0 is success
    let clean = err
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("1")
        == "0";

    let make = row.make.unwrap_or_default().trim().to_string();
    let model = row.model.unwrap_or_default().trim().to_string();
    let year = row
        .model_year
        .as_deref()
        .and_then(|y| y.trim().parse().ok());
    let message = row
        .error_text
        .unwrap_or_else(|| body.message.unwrap_or_default());

    Ok(DecodedVin {
        make,
        model,
        year,
        clean,
        message,
    })
}

fn fetch_recalls(make: &str, model: &str, year: u16) -> Result<Vec<RecallItem>, String> {
    // NHTSA is picky about encoding; try a few make/model casings.
    let make_u = make.trim().to_ascii_uppercase();
    let model_trim = model.trim();
    let mut model_variants = vec![
        model_trim.to_string(),
        model_trim.to_ascii_uppercase(),
        model_trim.to_ascii_lowercase(),
        model_trim
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str().to_ascii_lowercase().as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    ];
    // First word only (e.g. "Forte GT" → "Forte")
    if let Some(first) = model_trim.split_whitespace().next() {
        model_variants.push(first.to_string());
        model_variants.push(first.to_ascii_uppercase());
    }
    model_variants.sort();
    model_variants.dedup();

    let mut last_err = String::new();
    let mut saw_empty_ok = false;
    for model_try in &model_variants {
        if model_try.is_empty() {
            continue;
        }
        let url = format!(
            "{RECALLS_BY_VEHICLE}?make={}&model={}&modelYear={}",
            urlencoding_lite(&make_u),
            urlencoding_lite(model_try),
            year
        );
        match fetch_recalls_url(&url) {
            Ok(out) if !out.is_empty() => return Ok(out),
            Ok(_empty) => {
                saw_empty_ok = true;
            }
            Err(e) => last_err = e,
        }
    }
    // Empty list is success (no campaigns on file for this YMM).
    if saw_empty_ok || last_err.is_empty() {
        return Ok(Vec::new());
    }
    Err(last_err)
}

fn fetch_recalls_url(url: &str) -> Result<Vec<RecallItem>, String> {
    // NHTSA often returns HTTP 400 + {"Count":0,"Message":"Results returned successfully","results":[]}
    // when there are simply no campaigns for that year/make/model.
    let resp = match agent().get(url).set("Accept", "application/json").call() {
        Ok(r) => r,
        Err(ureq::Error::Status(code, resp)) if (400..500).contains(&code) => resp,
        Err(ureq::Error::Status(code, _)) => {
            return Err(format!("Recalls network: status code {code}"));
        }
        Err(other) => {
            let s = other.to_string();
            if s.contains("timed out") || s.contains("Timeout") {
                return Err("Recalls timed out — check internet and try again.".into());
            }
            return Err(format!("Recalls network: {s}"));
        }
    };

    let mut body_bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut body_bytes)
        .map_err(|e| format!("Recalls read: {e}"))?;
    // Empty or non-JSON on 4xx → treat as no campaigns
    if body_bytes.is_empty() {
        return Ok(Vec::new());
    }
    let body: RecallsResponse = match serde_json::from_slice(&body_bytes) {
        Ok(b) => b,
        Err(_) => {
            // If message looks like success with no results, OK empty
            let s = String::from_utf8_lossy(&body_bytes);
            if s.contains("Results returned successfully") || s.contains("\"Count\":0") {
                return Ok(Vec::new());
            }
            return Err(format!("Recalls JSON parse failed"));
        }
    };

    let mut out = Vec::new();
    for r in body.results.unwrap_or_default() {
        out.push(RecallItem {
            campaign: r.nhtsa_campaign_number.unwrap_or_default(),
            component: r.component.unwrap_or_default(),
            summary: r.summary.unwrap_or_default(),
            consequence: r.consequence.unwrap_or_default(),
            remedy: r.remedy.unwrap_or_default(),
            report_date: r.report_received_date.unwrap_or_default(),
            manufacturer: r.manufacturer.unwrap_or_default(),
        });
    }
    out.sort_by(|a, b| b.report_date.cmp(&a.report_date));
    Ok(out)
}

/// Minimal URL-encode for query values (make/model may have spaces).
fn urlencoding_lite(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct VinDecodeResponse {
    #[serde(rename = "Message")]
    message: Option<String>,
    #[serde(rename = "Results")]
    results: Vec<VinDecodeRow>,
}

#[derive(Debug, Deserialize)]
struct VinDecodeRow {
    #[serde(rename = "Make")]
    make: Option<String>,
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "ModelYear")]
    model_year: Option<String>,
    #[serde(rename = "ErrorCode")]
    error_code: Option<String>,
    #[serde(rename = "ErrorText")]
    error_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecallsResponse {
    #[serde(default)]
    results: Option<Vec<RecallRow>>,
}

#[derive(Debug, Deserialize)]
struct RecallRow {
    #[serde(rename = "NHTSACampaignNumber")]
    nhtsa_campaign_number: Option<String>,
    #[serde(rename = "Component")]
    component: Option<String>,
    #[serde(rename = "Summary")]
    summary: Option<String>,
    #[serde(rename = "Consequence")]
    consequence: Option<String>,
    #[serde(rename = "Remedy")]
    remedy: Option<String>,
    #[serde(rename = "ReportReceivedDate")]
    report_received_date: Option<String>,
    #[serde(rename = "Manufacturer")]
    manufacturer: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_vin() {
        assert_eq!(
            normalize_vin("1hgcm82633a004352").unwrap(),
            "1HGCM82633A004352"
        );
        assert!(normalize_vin("short").is_err());
        assert!(normalize_vin("1HGCM82633A00435I").is_err()); // I invalid
    }

    #[test]
    #[ignore = "network"]
    fn live_decode_and_recalls() {
        let r = check_recalls_for_vin("1HGCM82633A004352").expect("api");
        assert_eq!(r.decoded.make.to_ascii_uppercase(), "HONDA");
        assert!(!r.recalls.is_empty());
    }
}
