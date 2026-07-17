//! Lightweight "OCR assist": parse common fields from pasted receipt text.
//! Real camera OCR can fill this same form later.

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
}

/// Extract date / mileage / gallons / money from free-form receipt text.
pub fn parse_receipt_text(text: &str) -> ParsedReceipt {
    let mut out = ParsedReceipt::default();

    // Dates: 2024-07-16, 07/16/2024, 7/16/24
    if let Ok(re) = Regex::new(
        r"(?i)\b(20\d{2})[-/](\d{1,2})[-/](\d{1,2})\b|\b(\d{1,2})[/\-](\d{1,2})[/\-](20\d{2}|\d{2})\b",
    ) {
        if let Some(c) = re.captures(text) {
            if let (Some(y), Some(m), Some(d)) = (c.get(1), c.get(2), c.get(3)) {
                out.date = Some(format!(
                    "{:04}-{:02}-{:02}",
                    y.as_str().parse::<u32>().unwrap_or(0),
                    m.as_str().parse::<u32>().unwrap_or(0),
                    d.as_str().parse::<u32>().unwrap_or(0)
                ));
            } else if let (Some(m), Some(d), Some(y)) = (c.get(4), c.get(5), c.get(6)) {
                let mut year = y.as_str().parse::<u32>().unwrap_or(0);
                if year < 100 {
                    year += 2000;
                }
                out.date = Some(format!(
                    "{:04}-{:02}-{:02}",
                    year,
                    m.as_str().parse::<u32>().unwrap_or(0),
                    d.as_str().parse::<u32>().unwrap_or(0)
                ));
            }
        }
    }

    // Mileage: 123,456 mi / ODO 123456 / mileage: 98000
    if let Ok(re) = Regex::new(
        r"(?i)(?:odo(?:meter)?|mileage|miles?|mi\.?)\s*[:=]?\s*([\d,]{3,7})\b|\b([\d,]{4,7})\s*(?:mi|miles)\b",
    ) {
        if let Some(c) = re.captures(text) {
            let raw = c.get(1).or_else(|| c.get(2)).map(|m| m.as_str()).unwrap_or("");
            let digits: String = raw.chars().filter(|ch| ch.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                if (1_000..=2_000_000).contains(&n) {
                    out.mileage = Some(n);
                }
            }
        }
    }

    // Gallons
    if let Ok(re) = Regex::new(r"(?i)\b(\d{1,3}(?:\.\d{1,3})?)\s*(?:gal|gallons?)\b") {
        if let Some(c) = re.captures(text) {
            if let Ok(g) = c[1].parse::<f64>() {
                if g > 0.0 && g < 100.0 {
                    out.gallons = Some(g);
                }
            }
        }
    }

    // Money amounts
    let mut amounts: Vec<(usize, f64, String)> = Vec::new();
    if let Ok(re) = Regex::new(r"(?i)(?:\$|usd\s*)(\d{1,5}(?:\.\d{2})?)") {
        for c in re.captures_iter(text) {
            if let Ok(v) = c[1].parse::<f64>() {
                let start = c.get(0).map(|m| m.start()).unwrap_or(0);
                let ctx = text
                    .get(start.saturating_sub(24)..start.saturating_add(24))
                    .unwrap_or("")
                    .to_ascii_lowercase();
                amounts.push((start, v, ctx));
            }
        }
    }

    for (_pos, v, ctx) in &amounts {
        if ctx.contains("labor") || ctx.contains("labour") {
            out.labor_cost = Some(*v);
        } else if ctx.contains("part") || ctx.contains("parts") {
            out.parts_cost = Some(*v);
        } else if ctx.contains("fuel") || ctx.contains("gas") || ctx.contains("petrol") {
            out.fuel_cost = Some(*v);
        } else if ctx.contains("total") || ctx.contains("amount due") || ctx.contains("balance") {
            out.total_cost = Some(*v);
        }
    }
    // Fallback total = largest amount
    if out.total_cost.is_none() {
        out.total_cost = amounts.iter().map(|(_, v, _)| *v).fold(None, |acc, v| {
            Some(acc.map(|a: f64| a.max(v)).unwrap_or(v))
        });
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
    }

    #[test]
    fn parses_gallons() {
        let p = parse_receipt_text("Fuel stop 12.4 gal $45.00");
        assert_eq!(p.gallons, Some(12.4));
    }
}
