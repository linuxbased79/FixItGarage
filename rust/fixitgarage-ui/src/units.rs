//! Imperial / metric display preference.
//!
//! **Canonical storage** (backward compatible with existing state.json):
//! - Distance: miles (u32)
//! - Fuel volume: US gallons (f64)
//! - Tread: millimeters (f64)
//!
//! Metric mode converts for UI only (km, liters, L/100km).

use serde::{Deserialize, Serialize};

pub const MI_TO_KM: f64 = 1.609_344;
pub const GAL_TO_L: f64 = 3.785_411_784;
/// mm per 1/32 inch (tread bars in the US).
pub const MM_PER_32ND: f64 = 25.4 / 32.0;
/// US MPG → L/100 km.
pub const MPG_TO_LP100: f64 = 235.214_583;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnitSystem {
    Imperial,
    Metric,
}

impl UnitSystem {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "METRIC" | "SI" | "KM" | "METRIC_SYSTEM" => Self::Metric,
            _ => Self::Imperial,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Imperial => "IMPERIAL",
            Self::Metric => "METRIC",
        }
    }

    pub fn is_metric(self) -> bool {
        matches!(self, Self::Metric)
    }

    pub fn distance_unit(self) -> &'static str {
        match self {
            Self::Imperial => "mi",
            Self::Metric => "km",
        }
    }

    pub fn distance_label(self) -> &'static str {
        match self {
            Self::Imperial => "Mileage (mi)",
            Self::Metric => "Odometer (km)",
        }
    }

    pub fn fuel_unit(self) -> &'static str {
        match self {
            Self::Imperial => "gal",
            Self::Metric => "L",
        }
    }

    pub fn fuel_label(self) -> &'static str {
        match self {
            Self::Imperial => "Gallons",
            Self::Metric => "Liters",
        }
    }

    pub fn economy_unit(self) -> &'static str {
        match self {
            Self::Imperial => "MPG",
            Self::Metric => "L/100km",
        }
    }

    pub fn tread_unit(self) -> &'static str {
        match self {
            Self::Imperial => "1/32\"",
            Self::Metric => "mm",
        }
    }

    pub fn oil_volume_unit(self) -> &'static str {
        match self {
            Self::Imperial => "qt",
            Self::Metric => "L",
        }
    }
}

// ---- Distance (stored as miles) ----

pub fn miles_to_display(miles: u32, units: UnitSystem) -> u32 {
    match units {
        UnitSystem::Imperial => miles,
        UnitSystem::Metric => ((f64::from(miles) * MI_TO_KM).round() as u32).max(0),
    }
}

/// Parse a user distance value into stored miles.
pub fn display_to_miles(value: u32, units: UnitSystem) -> u32 {
    match units {
        UnitSystem::Imperial => value,
        UnitSystem::Metric => ((f64::from(value) / MI_TO_KM).round() as u32).max(0),
    }
}

pub fn format_distance(miles: u32, units: UnitSystem) -> String {
    format!(
        "{} {}",
        miles_to_display(miles, units),
        units.distance_unit()
    )
}

// ---- Fuel (stored as US gallons) ----

pub fn gallons_to_display(gallons: f64, units: UnitSystem) -> f64 {
    match units {
        UnitSystem::Imperial => gallons,
        UnitSystem::Metric => gallons * GAL_TO_L,
    }
}

pub fn display_to_gallons(value: f64, units: UnitSystem) -> f64 {
    match units {
        UnitSystem::Imperial => value,
        UnitSystem::Metric => value / GAL_TO_L,
    }
}

pub fn format_fuel(gallons: f64, units: UnitSystem) -> String {
    let v = gallons_to_display(gallons, units);
    format!("{v:.2} {}", units.fuel_unit())
}

// ---- Economy (stored as US MPG) ----

pub fn mpg_to_display(mpg: f64, units: UnitSystem) -> f64 {
    match units {
        UnitSystem::Imperial => mpg,
        UnitSystem::Metric => {
            if mpg <= 0.0 {
                0.0
            } else {
                MPG_TO_LP100 / mpg
            }
        }
    }
}

pub fn format_economy(mpg: f64, units: UnitSystem) -> String {
    let v = mpg_to_display(mpg, units);
    match units {
        UnitSystem::Imperial => format!("{v:.1} MPG"),
        UnitSystem::Metric => format!("{v:.1} L/100km"),
    }
}

// ---- Tread (stored as mm) ----

pub fn mm_to_display(mm: f64, units: UnitSystem) -> f64 {
    match units {
        UnitSystem::Metric => mm,
        UnitSystem::Imperial => mm / MM_PER_32ND, // in 32nds
    }
}

pub fn display_to_mm(value: f64, units: UnitSystem) -> f64 {
    match units {
        UnitSystem::Metric => value,
        UnitSystem::Imperial => value * MM_PER_32ND,
    }
}

pub fn format_tread(mm: f64, units: UnitSystem) -> String {
    let v = mm_to_display(mm, units);
    match units {
        UnitSystem::Metric => format!("{v:.1} mm"),
        UnitSystem::Imperial => format!("{v:.1}/32\""),
    }
}

/// Legal wear limit display (~1.6 mm / 2/32").
pub fn tread_limit_label(units: UnitSystem) -> String {
    match units {
        UnitSystem::Metric => "Legal wear limit is usually 1.6 mm.".into(),
        UnitSystem::Imperial => "Legal wear limit is usually 2/32\" (~1.6 mm).".into(),
    }
}

pub fn tread_coin_guide(units: UnitSystem) -> String {
    match units {
        UnitSystem::Metric => {
            "Camera assist: place a 1 € coin or similar in the groove for scale, photo each corner, then enter depth in mm (legal min often 1.6 mm)."
                .into()
        }
        UnitSystem::Imperial => {
            "Coin gauge (camera assist): US penny Lincoln head ~2/32\" wear limit; \
             quarter Washington head ~4/32\" mid wear. Enter depth in 32nds of an inch."
                .into()
        }
    }
}

// ---- Oil level labels ----

pub fn oil_level_options(units: UnitSystem) -> &'static [&'static str] {
    match units {
        UnitSystem::Imperial => &[
            "Full",
            "½ quart low",
            "1 quart low",
            "2 quarts low",
            "3 quarts low",
            "Overfilled",
        ],
        UnitSystem::Metric => &[
            "Full",
            "0.5 L low",
            "1 L low",
            "2 L low",
            "3 L low",
            "Overfilled",
        ],
    }
}

/// Map any stored oil level string to the preferred unit wording for display.
pub fn display_oil_level(stored: &str, units: UnitSystem) -> String {
    let n = crate::state::normalize_oil_level(stored);
    // normalize returns imperial-ish canonical; remap for metric UI
    if units.is_metric() {
        match n.as_str() {
            "Full" => "Full".into(),
            "½ quart low" => "0.5 L low".into(),
            "1 quart low" => "1 L low".into(),
            "2 quarts low" => "2 L low".into(),
            "3 quarts low" => "3 L low".into(),
            "Overfilled" => "Overfilled".into(),
            other => other.to_string(),
        }
    } else {
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_roundtrip_metric() {
        let mi = 62_137u32; // ~100000 km
        let km = miles_to_display(mi, UnitSystem::Metric);
        assert!((km as i64 - 100_000).abs() < 5);
        let back = display_to_miles(km, UnitSystem::Metric);
        assert!((back as i64 - mi as i64).abs() <= 2);
    }

    #[test]
    fn fuel_liters() {
        let gal = 10.0;
        let l = gallons_to_display(gal, UnitSystem::Metric);
        assert!((l - 37.854).abs() < 0.01);
        assert!((display_to_gallons(l, UnitSystem::Metric) - gal).abs() < 0.001);
    }

    #[test]
    fn economy_lp100() {
        // 30 MPG ≈ 7.84 L/100km
        let v = mpg_to_display(30.0, UnitSystem::Metric);
        assert!((v - 7.84).abs() < 0.05);
    }

    #[test]
    fn tread_32nds() {
        // 1.6 mm ≈ 2/32"
        let t = mm_to_display(1.6, UnitSystem::Imperial);
        assert!((t - 2.0).abs() < 0.05);
        assert!((display_to_mm(2.0, UnitSystem::Imperial) - 1.6).abs() < 0.05);
    }
}
