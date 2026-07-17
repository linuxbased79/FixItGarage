//! Receipt OCR assist: normalize noisy OCR text and parse common fields.
//! Camera / share pipeline feeds this same form (see platform OCR helpers).

use regex::Regex;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedReceipt {
    pub date: Option<String>,       // YYYY-MM-DD
    pub mileage: Option<u32>,
    pub gallons: Option<f64>,
    pub total_cost: Option<f64>,
    pub parts_cost: Option<f64>,
    pub labor_cost: Option<f64>,
    pub fuel_cost: Option<f64>,
    pub shop_name: Option<String>,
    /// Suggested service title from keywords (oil change, rotation, …).
    pub title: Option<String>,
}

/// Clean typical OCR noise before field extraction.
/// Soft fixes only — does not invent values.
pub fn normalize_ocr_text(text: &str) -> String {
    let mut s = text.replace('\u{00a0}', " ").replace('\u{fffd}', "");
    // Normalize newlines (OCR often uses CR or vertical bars as line breaks)
    s = s.replace("\r\n", "\n").replace('\r', "\n");
    s = s.replace('|', "\n");
    // Currency glued with space: "$ 42.50" / "€ 12,00"
    if let Ok(re) = Regex::new(r"([\$€£])\s+(\d)") {
        s = re.replace_all(&s, "$1$2").into_owned();
    }
    // "42 . 50" → "42.50" (digit space punct space digit)
    if let Ok(re) = Regex::new(r"(\d)\s+([.,])\s+(\d)") {
        s = re.replace_all(&s, "$1$2$3").into_owned();
    }
    // Collapse runs of spaces/tabs (keep newlines)
    if let Ok(re) = Regex::new(r"[^\S\n]+") {
        s = re.replace_all(&s, " ").into_owned();
    }
    // Common OCR word fixes (case-insensitive whole words)
    let word_fixes: &[(&str, &str)] = &[
        (r"(?i)\btotai\b", "Total"),
        (r"(?i)\btota1\b", "Total"),
        (r"(?i)\boii\b", "Oil"),
        (r"(?i)\boil\s+change\b", "oil change"),
        (r"(?i)\blab0r\b", "Labor"),
        (r"(?i)\blabour\b", "Labour"),
        (r"(?i)\bodosmeter\b", "Odometer"),
        (r"(?i)\bodometer\b", "Odometer"),
        (r"(?i)\bmi1es\b", "miles"),
        (r"(?i)\bga11ons\b", "gallons"),
        (r"(?i)\bgailons\b", "gallons"),
    ];
    for (pat, rep) in word_fixes {
        if let Ok(re) = Regex::new(pat) {
            s = re.replace_all(&s, *rep).into_owned();
        }
    }
    // Fix letter O/o as zero inside digit runs: "98O32" → "98032", "12.4O" → "12.40"
    if let Ok(re) = Regex::new(r"(\d)[Oo](\d)") {
        s = re.replace_all(&s, "${1}0${2}").into_owned();
    }
    if let Ok(re) = Regex::new(r"(\d)[Oo]\b") {
        s = re.replace_all(&s, "${1}0").into_owned();
    }
    // Fix l/I as 1 inside digit runs: "98l32" / "98I32"
    if let Ok(re) = Regex::new(r"(\d)[lI](\d)") {
        s = re.replace_all(&s, "${1}1${2}").into_owned();
    }
    s.trim().to_string()
}

/// Extract date / mileage / gallons / money from free-form receipt text.
pub fn parse_receipt_text(text: &str) -> ParsedReceipt {
    let text = normalize_ocr_text(text);
    let text = text.as_str();
    let mut out = ParsedReceipt::default();

    // Dates: 2024-07-16, 07/16/2024, 16/07/2024 (EU), 7/16/24
    if let Ok(re) = Regex::new(
        r"(?i)\b(20\d{2})[-/](\d{1,2})[-/](\d{1,2})\b|\b(\d{1,2})[/\-.](\d{1,2})[/\-.](20\d{2}|\d{2})\b",
    ) {
        if let Some(c) = re.captures(text) {
            if let (Some(y), Some(m), Some(d)) = (c.get(1), c.get(2), c.get(3)) {
                let year = y.as_str().parse::<u32>().unwrap_or(0);
                let month = m.as_str().parse::<u32>().unwrap_or(0);
                let day = d.as_str().parse::<u32>().unwrap_or(0);
                if (1..=12).contains(&month) && (1..=31).contains(&day) {
                    out.date = Some(format!("{year:04}-{month:02}-{day:02}"));
                }
            } else if let (Some(a), Some(b), Some(y)) = (c.get(4), c.get(5), c.get(6)) {
                let mut year = y.as_str().parse::<u32>().unwrap_or(0);
                if year < 100 {
                    year += 2000;
                }
                let n1 = a.as_str().parse::<u32>().unwrap_or(0);
                let n2 = b.as_str().parse::<u32>().unwrap_or(0);
                // Prefer DMY when first number > 12 (16/07/2024); else MDY (US)
                let (month, day) = if n1 > 12 && n2 <= 12 {
                    (n2, n1)
                } else if n2 > 12 && n1 <= 12 {
                    (n1, n2)
                } else {
                    (n1, n2) // ambiguous → MDY
                };
                if (1..=12).contains(&month) && (1..=31).contains(&day) {
                    out.date = Some(format!("{year:04}-{month:02}-{day:02}"));
                }
            }
        }
    }

    // Mileage / odometer — store as **miles** always.
    // Patterns: 98,432 mi · ODO 123456 · 158200 km · mileage: 98000
    if let Ok(re) = Regex::new(
        r"(?i)(?:odo(?:meter)?|mileage|kilomet(?:er|re)?s?|km|miles?|mi\.?)\s*[:=]?\s*([\d,]{3,7})\s*(km|mi|miles?)?\b|\b([\d,]{4,7})\s*(km|mi|miles)\b",
    ) {
        if let Some(c) = re.captures(text) {
            let raw = c
                .get(1)
                .or_else(|| c.get(3))
                .map(|m| m.as_str())
                .unwrap_or("");
            let unit = c
                .get(2)
                .or_else(|| c.get(4))
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            let digits: String = raw.chars().filter(|ch| ch.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                if (1_000..=3_000_000).contains(&n) {
                    let miles = if unit == "km" {
                        ((f64::from(n) / 1.609_344).round() as u32).max(1)
                    } else {
                        // bare "odo" number or mi → treat as miles (legacy NA receipts)
                        n
                    };
                    // If label was kilometre without unit group, detect from full match text
                    let miles = if unit.is_empty() {
                        let snip = c.get(0).map(|m| m.as_str().to_ascii_lowercase()).unwrap_or_default();
                        if snip.contains("km") || snip.contains("kilomet") {
                            ((f64::from(n) / 1.609_344).round() as u32).max(1)
                        } else {
                            miles
                        }
                    } else {
                        miles
                    };
                    out.mileage = Some(miles);
                }
            }
        }
    }

    // Fuel volume — store as **gallons** always.
    // 12.4 gal · 45.2 L · 45,2 l (EU comma) · 12 liters
    if let Ok(re) = Regex::new(
        r"(?i)\b(\d{1,3}(?:[.,]\d{1,3})?)\s*(?:gal|gallons?|l|liters?|litres?)\b",
    ) {
        if let Some(c) = re.captures(text) {
            let raw = c[1].replace(',', ".");
            if let Ok(v) = raw.parse::<f64>() {
                if v > 0.0 && v < 200.0 {
                    let match_l = c.get(0).map(|m| m.as_str().to_ascii_lowercase()).unwrap_or_default();
                    let gallons = if match_l.contains("gal") {
                        v
                    } else {
                        // liters → gallons
                        v / 3.785_411_784
                    };
                    if gallons > 0.0 && gallons < 100.0 {
                        out.gallons = Some(gallons);
                    }
                }
            }
        }
    }

    // Money amounts — $, €, £, USD/EUR/GBP — classify using the full line for context
    let mut amounts: Vec<f64> = Vec::new();
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        // Amounts like $42.50, €42,50, 42.50 €, 60 EUR (no \b after currency symbols)
        let Ok(re) = Regex::new(
            r"(?i)(?:\$|usd\s*|€|eur\s*|£|gbp\s*)(\d{1,5}(?:[.,]\d{2})?)|(\d{1,5}(?:[.,]\d{2})?)\s*(?:€|eur\b|usd\b|gbp\b|\$)",
        ) else {
            continue;
        };
        for c in re.captures_iter(line) {
            let raw = c
                .get(1)
                .or_else(|| c.get(2))
                .map(|m| m.as_str().replace(',', "."))
                .unwrap_or_default();
            if let Ok(v) = raw.parse::<f64>() {
                amounts.push(v);
                if lower.contains("labor") || lower.contains("labour") || lower.contains("main d'") {
                    out.labor_cost = Some(v);
                } else if lower.contains("part") || lower.contains("pièce") || lower.contains("pieza")
                {
                    out.parts_cost = Some(v);
                } else if lower.contains("fuel")
                    || lower.contains("gas")
                    || lower.contains("petrol")
                    || lower.contains("diesel")
                    || lower.contains("essence")
                {
                    out.fuel_cost = Some(v);
                } else if lower.contains("total")
                    || lower.contains("amount due")
                    || lower.contains("balance")
                    || lower.contains("summe")
                    || lower.contains("gesamt")
                {
                    out.total_cost = Some(v);
                }
            }
        }
    }
    // Fallback total = largest amount
    if out.total_cost.is_none() {
        out.total_cost = amounts.iter().copied().reduce(f64::max);
    }
    // If only total and no parts/labor split, put total into parts for DIY convenience
    if out.parts_cost.is_none() && out.labor_cost.is_none() {
        if let Some(t) = out.total_cost {
            out.parts_cost = Some(t);
        }
    }

    // Shop: first non-empty line that looks like a name (no $ and short)
    for line in text.lines().map(str::trim).filter(|l| l.len() > 2 && l.len() < 48) {
        let lower = line.to_ascii_lowercase();
        if lower.contains("total")
            || lower.contains("invoice")
            || lower.contains("receipt")
            || line.contains('$')
            || lower.starts_with("date")
            || lower.starts_with("tel")
            || lower.starts_with("phone")
        {
            continue;
        }
        if line.chars().any(|c| c.is_ascii_alphabetic()) {
            out.shop_name = Some(line.to_string());
            break;
        }
    }

    // Title keywords (order = priority)
    let lower_all = text.to_ascii_lowercase();
    let title_rules: &[(&str, &str)] = &[
        ("oil change", "Oil change"),
        ("oil filter", "Oil & filter"),
        ("synthetic oil", "Oil change"),
        ("tire rotation", "Tire rotation"),
        ("rotation", "Tire rotation"),
        ("alignment", "Wheel alignment"),
        ("brake pad", "Brake service"),
        ("brake fluid", "Brake fluid"),
        ("wiper", "Wiper blades"),
        ("battery", "Battery"),
        ("air filter", "Air filter"),
        ("cabin filter", "Cabin filter"),
        ("inspection", "Inspection"),
        ("tune-up", "Tune-up"),
        ("tune up", "Tune-up"),
        ("fuel", "Fuel fill-up"),
        ("gasoline", "Fuel fill-up"),
        ("diesel", "Fuel fill-up"),
    ];
    for (key, title) in title_rules {
        if lower_all.contains(key) {
            out.title = Some((*title).into());
            break;
        }
    }
    if out.title.is_none() && out.gallons.is_some() {
        out.title = Some("Fuel fill-up".into());
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_receipt() {
        let text = r#"
        Joe's Auto Shop
        Date: 07/16/2024
        Odometer: 98,432 mi
        Oil change
        Parts $42.50
        Labor $60.00
        Total $102.50
        "#;
        let p = parse_receipt_text(text);
        assert_eq!(p.date.as_deref(), Some("2024-07-16"));
        assert_eq!(p.mileage, Some(98432));
        assert_eq!(p.parts_cost, Some(42.50));
        assert_eq!(p.labor_cost, Some(60.00));
        assert!(p.shop_name.as_deref().unwrap_or("").contains("Joe"));
        assert_eq!(p.title.as_deref(), Some("Oil change"));
    }

    #[test]
    fn parses_gallons() {
        let p = parse_receipt_text("Fuel stop 12.4 gal $45.00");
        assert_eq!(p.gallons, Some(12.4));
    }

    #[test]
    fn normalizes_ocr_noise() {
        let raw = "Joe's Auto\nTota1 $1O2.50\nOii change\nOdometer: 98O32 mi\n";
        let n = normalize_ocr_text(raw);
        assert!(n.contains("Total"), "got: {n}");
        assert!(n.contains("Oil") || n.to_ascii_lowercase().contains("oil"), "got: {n}");
        assert!(n.contains("98032"), "got: {n}");
        let p = parse_receipt_text(raw);
        assert_eq!(p.mileage, Some(98032));
        assert_eq!(p.total_cost, Some(102.50));
    }

    #[test]
    fn parses_metric_fuel_and_odometer() {
        // 45.2 L ≈ 11.94 gal; 158200 km ≈ 98326 mi
        let text = "Tankstelle Muster\nKm-Stand: 158200 km\n45.2 L\nTotal €72.50\n";
        let p = parse_receipt_text(text);
        assert!(p.gallons.is_some());
        let g = p.gallons.unwrap();
        assert!((g - 11.94).abs() < 0.05, "gallons={g}");
        assert!(p.mileage.is_some());
        let mi = p.mileage.unwrap();
        assert!((98_000..=99_000).contains(&mi), "miles={mi}");
        assert_eq!(p.total_cost, Some(72.50));
    }

    #[test]
    fn parses_euro_parts_labor() {
        let text = "Garage Lyon\nDate: 16/07/2024\nPièces 42,50 €\nMain d'oeuvre 60,00 €\nTotal 102,50 €\n";
        let p = parse_receipt_text(text);
        assert_eq!(p.date.as_deref(), Some("2024-07-16"));
        assert_eq!(p.parts_cost, Some(42.50));
        assert_eq!(p.labor_cost, Some(60.00));
        assert_eq!(p.total_cost, Some(102.50));
    }

    #[test]
    fn parses_tire_receipt() {
        let text = r#"
        Discount Tire
        Michelin Defender LTX
        Size: 265/70R17
        Total $612.00
        Odometer: 45,200 mi
        "#;
        let p = parse_tire_receipt_text(text);
        assert!(p.brand.as_deref().unwrap_or("").contains("Michelin") || p.model.is_some());
        assert_eq!(p.size.as_deref(), Some("265/70R17"));
        assert_eq!(p.cost, Some(612.0));
        assert_eq!(p.mileage, Some(45200));
    }
}

/// Tire-shop receipt fields (brand / model / size / cost) for the tire tracker.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedTireReceipt {
    pub brand: Option<String>,
    pub model: Option<String>,
    pub size: Option<String>,
    pub cost: Option<f64>,
    pub mileage: Option<u32>,
    pub shop_name: Option<String>,
    pub notes: Option<String>,
}

/// Parse free-form tire purchase receipt text (paste from OCR / email).
pub fn parse_tire_receipt_text(text: &str) -> ParsedTireReceipt {
    let text = normalize_ocr_text(text);
    let base = parse_receipt_text(&text);
    let text = text.as_str();
    let mut out = ParsedTireReceipt {
        cost: base.total_cost.or(base.parts_cost),
        mileage: base.mileage,
        shop_name: base.shop_name.clone(),
        notes: base.shop_name.map(|s| format!("Shop: {s}")),
        ..Default::default()
    };

    // Tire size e.g. 225/65R17, P215/60R16, 265/70R17 112T
    if let Ok(re) = Regex::new(r"(?i)\bP?(\d{3})\s*/\s*(\d{2})\s*[RrZz]\s*(\d{2})\b") {
        if let Some(c) = re.captures(text) {
            out.size = Some(format!("{}/{}R{}", &c[1], &c[2], &c[3]));
        }
    }

    // Known brands (common NA/EU tire makers)
    const BRANDS: &[&str] = &[
        "Michelin",
        "Goodyear",
        "Bridgestone",
        "Continental",
        "Pirelli",
        "Yokohama",
        "Hankook",
        "Toyo",
        "Falken",
        "Cooper",
        "Firestone",
        "BFGoodrich",
        "BF Goodrich",
        "General",
        "Kumho",
        "Nitto",
        "Dunlop",
        "Uniroyal",
        "Nexen",
        "Kelly",
        "Mastercraft",
        "Vredestein",
        "Sumitomo",
    ];
    for brand in BRANDS {
        let brand_l = brand.to_ascii_lowercase();
        for line in text.lines() {
            let line_l = line.to_ascii_lowercase();
            if let Some(idx) = line_l.find(&brand_l) {
                out.brand = Some((*brand).to_string());
                let after = line.get(idx + brand.len()..).unwrap_or("").trim();
                let mut model = after
                    .trim_start_matches(|c: char| !c.is_alphanumeric())
                    .split('$')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if let Some(sz) = &out.size {
                    model = model.replace(sz, "").trim().to_string();
                }
                // Strip common size if still in model via regex leftovers
                if let Ok(re) = Regex::new(r"(?i)P?\d{3}\s*/\s*\d{2}\s*[RrZz]\s*\d{2}") {
                    model = re.replace_all(&model, "").trim().to_string();
                }
                if model.len() >= 2 && model.len() < 48 {
                    out.model = Some(model);
                }
                return out;
            }
        }
    }

    out
}
