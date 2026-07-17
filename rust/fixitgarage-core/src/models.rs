use serde::{Deserialize, Serialize};

/// Setup-wizard preference mirrored from the Android app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserMode {
    Diy,
    Shop,
    Both,
}

impl UserMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "DIY" => Some(Self::Diy),
            "SHOP" => Some(Self::Shop),
            "BOTH" => Some(Self::Both),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServiceSource {
    Diy,
    Shop,
}

impl ServiceSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Diy => "DIY",
            Self::Shop => "SHOP",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vehicle {
    pub id: u64,
    pub name: String,
    pub make: String,
    pub model: String,
    pub year: Option<u16>,
    pub current_mileage: u32,
    /// 17-character VIN (optional). Used for NHTSA recall checks.
    #[serde(default)]
    pub vin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceRecord {
    pub id: u64,
    pub vehicle_id: u64,
    /// Unix epoch milliseconds
    pub date_epoch_ms: i64,
    pub mileage: u32,
    pub title: String,
    pub source: ServiceSource,
    pub labor_cost: f64,
    pub parts_cost: f64,
    pub gallons: Option<f64>,
    pub fuel_cost: Option<f64>,
    pub shop_name: String,
    /// Free-form notes (parts used, DIY steps, shop invoice #, …).
    #[serde(default)]
    pub notes: String,
}

impl ServiceRecord {
    pub fn total_cost(&self) -> f64 {
        self.labor_cost + self.parts_cost + self.fuel_cost.unwrap_or(0.0)
    }
}
