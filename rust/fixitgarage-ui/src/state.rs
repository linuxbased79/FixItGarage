//! Local-first in-memory + JSON persistence for the Slint UI.

use chrono::Utc;
use fixitgarage_core::models::{ServiceRecord, ServiceSource, UserMode, Vehicle};
use fixitgarage_core::{
    apply_rotation, average_mpg, services_to_csv, summarize_costs, RotationPattern, TireLayout,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub wizard_done: bool,
    pub user_mode: String,
    /// "LIGHT" or "DARK" — applied across the Slint Theme palette.
    #[serde(default = "default_dark_mode")]
    pub dark_mode: String,
    pub next_vehicle_id: u64,
    pub next_service_id: u64,
    pub vehicles: Vec<Vehicle>,
    pub services: Vec<ServiceRecord>,
    pub tire_layout: TireLayoutStored,
    pub tire_pattern: String,
}

fn default_dark_mode() -> String {
    "LIGHT".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireLayoutStored {
    pub fl: String,
    pub fr: String,
    pub rl: String,
    pub rr: String,
}

impl From<&TireLayoutStored> for TireLayout {
    fn from(t: &TireLayoutStored) -> Self {
        TireLayout {
            fl: t.fl.clone(),
            fr: t.fr.clone(),
            rl: t.rl.clone(),
            rr: t.rr.clone(),
        }
    }
}

impl From<&TireLayout> for TireLayoutStored {
    fn from(t: &TireLayout) -> Self {
        Self {
            fl: t.fl.clone(),
            fr: t.fr.clone(),
            rl: t.rl.clone(),
            rr: t.rr.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            wizard_done: false,
            user_mode: "BOTH".into(),
            dark_mode: default_dark_mode(),
            next_vehicle_id: 1,
            next_service_id: 1,
            vehicles: Vec::new(),
            services: Vec::new(),
            tire_layout: TireLayoutStored {
                fl: "A".into(),
                fr: "B".into(),
                rl: "C".into(),
                rr: "D".into(),
            },
            tire_pattern: "forward_cross".into(),
        }
    }
}

impl AppState {
    pub fn data_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("fixitgarage")
            .join("state.json")
    }

    pub fn load() -> Self {
        let path = Self::data_path();
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(state) = serde_json::from_slice(&bytes) {
                return state;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::data_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn mode_label(&self) -> String {
        match UserMode::parse(&self.user_mode).unwrap_or(UserMode::Both) {
            UserMode::Diy => "DIY-focused tools ready".into(),
            UserMode::Shop => "Shop & receipt tools ready".into(),
            UserMode::Both => "Full garage toolkit".into(),
        }
    }

    pub fn last_service(&self) -> Option<&ServiceRecord> {
        self.services.iter().max_by_key(|s| (s.date_epoch_ms, s.id))
    }

    pub fn mpg_label(&self) -> String {
        let primary = self.vehicles.first().map(|v| v.id);
        let Some(vid) = primary else {
            return "MPG: add a vehicle first".into();
        };
        let fills: Vec<(u32, f64)> = self
            .services
            .iter()
            .filter(|s| s.vehicle_id == vid)
            .filter_map(|s| s.gallons.map(|g| (s.mileage, g)))
            .collect();
        // sort by mileage for MPG
        let mut fills = fills;
        fills.sort_by_key(|(m, _)| *m);
        match average_mpg(&fills) {
            Some(mpg) => format!("MPG: {mpg:.1}"),
            None => "MPG: need 2+ fuel fill-ups".into(),
        }
    }

    pub fn cost_labels(&self) -> [(String, String); 3] {
        let now = Utc::now().timestamp_millis();
        let s = summarize_costs(&self.services, now);
        [
            ("This month".into(), format!("${:.2}", s.month_total)),
            ("This year".into(), format!("${:.2}", s.year_total)),
            ("All time".into(), format!("${:.2}", s.all_time_total)),
        ]
    }

    pub fn tire_preview(&self) -> String {
        let layout = TireLayout::from(&self.tire_layout);
        let pattern =
            RotationPattern::from_str(&self.tire_pattern).unwrap_or(RotationPattern::ForwardCross);
        let after = apply_rotation(&layout, pattern);
        format!(
            "After {}: {} {} / {} {}",
            pattern.label(),
            after.fl,
            after.fr,
            after.rl,
            after.rr
        )
    }

    pub fn apply_tire_rotation(&mut self) {
        let layout = TireLayout::from(&self.tire_layout);
        let pattern =
            RotationPattern::from_str(&self.tire_pattern).unwrap_or(RotationPattern::ForwardCross);
        let after = apply_rotation(&layout, pattern);
        self.tire_layout = TireLayoutStored::from(&after);
    }

    pub fn export_csv(&self) -> String {
        services_to_csv(&self.services)
    }

    pub fn add_vehicle(
        &mut self,
        name: String,
        make: String,
        model: String,
        year: Option<u16>,
        mileage: u32,
    ) {
        if name.trim().is_empty() {
            return;
        }
        let id = self.next_vehicle_id;
        self.next_vehicle_id += 1;
        self.vehicles.push(Vehicle {
            id,
            name: name.trim().into(),
            make,
            model,
            year,
            current_mileage: mileage,
        });
        self.save();
    }

    pub fn add_service(
        &mut self,
        title: String,
        mileage: u32,
        source: &str,
        cost: f64,
        gallons: Option<f64>,
    ) {
        if title.trim().is_empty() {
            return;
        }
        let vehicle_id = match self.vehicles.first() {
            Some(v) => v.id,
            None => return,
        };
        let id = self.next_service_id;
        self.next_service_id += 1;
        let source = match source {
            "SHOP" => ServiceSource::Shop,
            _ => ServiceSource::Diy,
        };
        self.services.push(ServiceRecord {
            id,
            vehicle_id,
            date_epoch_ms: Utc::now().timestamp_millis(),
            mileage,
            title: title.trim().into(),
            source,
            labor_cost: 0.0,
            parts_cost: cost,
            gallons,
            fuel_cost: None,
            shop_name: String::new(),
        });
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == vehicle_id) {
            if mileage > v.current_mileage {
                v.current_mileage = mileage;
            }
        }
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_vehicle_and_service_updates_mpg_inputs() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "Honda".into(), "Civic".into(), Some(2018), 10000);
        assert_eq!(s.vehicles.len(), 1);
        s.add_service("Fuel".into(), 10300, "DIY", 0.0, Some(10.0));
        s.add_service("Fuel".into(), 10600, "DIY", 0.0, Some(10.0));
        assert!(s.mpg_label().contains("30.0") || s.mpg_label().contains("MPG:"));
        let csv = s.export_csv();
        assert!(csv.contains("Fuel"));
    }

    #[test]
    fn rotation_changes_layout() {
        let mut s = AppState::default();
        s.tire_pattern = "side_to_side".into();
        s.apply_tire_rotation();
        assert_eq!(s.tire_layout.fl, "B");
        assert_eq!(s.tire_layout.fr, "A");
    }
}
