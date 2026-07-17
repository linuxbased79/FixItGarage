//! Local-first in-memory + JSON persistence for the Slint UI.

use chrono::{TimeZone, Utc};
use fixitgarage_core::models::{ServiceRecord, ServiceSource, UserMode, Vehicle};
use fixitgarage_core::reminders::{is_due_by_date, is_due_by_mileage, oil_level_due_after};
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
    #[serde(default = "default_dark_mode")]
    pub dark_mode: String,
    #[serde(default)]
    pub selected_vehicle_id: Option<u64>,
    pub next_vehicle_id: u64,
    pub next_service_id: u64,
    #[serde(default)]
    pub next_part_id: u64,
    #[serde(default)]
    pub next_note_id: u64,
    #[serde(default)]
    pub next_reminder_id: u64,
    #[serde(default)]
    pub next_component_id: u64,
    #[serde(default)]
    pub next_photo_id: u64,
    #[serde(default)]
    pub next_tire_purchase_id: u64,
    #[serde(default)]
    pub next_rotation_id: u64,
    pub vehicles: Vec<Vehicle>,
    pub services: Vec<ServiceRecord>,
    #[serde(default)]
    pub parts: Vec<PartEntry>,
    #[serde(default)]
    pub components: Vec<ComponentEntry>,
    #[serde(default)]
    pub notes: Vec<NoteEntry>,
    #[serde(default)]
    pub reminders: Vec<ReminderEntry>,
    /// History of oil dipstick readings (from oil-level checks).
    #[serde(default)]
    pub oil_level_logs: Vec<OilLevelLog>,
    #[serde(default)]
    pub issue_photos: Vec<IssuePhoto>,
    #[serde(default)]
    pub tire_purchases: Vec<TirePurchase>,
    #[serde(default)]
    pub tire_rotations: Vec<TireRotationLog>,
    pub tire_layout: TireLayoutStored,
    pub tire_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuePhoto {
    pub id: u64,
    pub vehicle_id: u64,
    pub caption: String,
    pub notes: String,
    /// Local path or content URI when a photo was attached/captured.
    pub file_path: String,
    pub created_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TirePurchase {
    pub id: u64,
    pub vehicle_id: u64,
    pub brand: String,
    pub model: String,
    pub size: String,
    pub cost: f64,
    pub mileage: Option<u32>,
    pub date_epoch_ms: i64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireRotationLog {
    pub id: u64,
    pub vehicle_id: u64,
    pub pattern: String,
    pub before_fl: String,
    pub before_fr: String,
    pub before_rl: String,
    pub before_rr: String,
    pub after_fl: String,
    pub after_fr: String,
    pub after_rl: String,
    pub after_rr: String,
    pub mileage: Option<u32>,
    pub date_epoch_ms: i64,
}

/// User-friendly dipstick reading when logging an oil level check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OilLevelLog {
    pub vehicle_id: u64,
    pub epoch_ms: i64,
    /// e.g. "Full", "1 quart low"
    pub level: String,
    pub mileage: Option<u32>,
}

/// Standard oil-level choices (easy to understand on a dipstick check).
pub const OIL_LEVEL_OPTIONS: &[&str] = &[
    "Full",
    "½ quart low",
    "1 quart low",
    "2 quarts low",
    "3 quarts low",
    "Overfilled",
];

pub fn normalize_oil_level(s: &str) -> String {
    let t = s.trim();
    for opt in OIL_LEVEL_OPTIONS {
        if t.eq_ignore_ascii_case(opt) {
            return (*opt).to_string();
        }
    }
    // Accept common aliases
    match t.to_ascii_lowercase().as_str() {
        "ok" | "good" | "full / ok" => "Full".into(),
        "0.5" | "half" | "1/2" | "half quart low" => "½ quart low".into(),
        "1" | "1 qt" | "1 quart" => "1 quart low".into(),
        "2" | "2 qt" | "2 quarts" => "2 quarts low".into(),
        "3" | "3 qt" | "3 quarts" => "3 quarts low".into(),
        "over" | "high" | "too full" => "Overfilled".into(),
        _ if !t.is_empty() => t.to_string(),
        _ => "Full".into(),
    }
}

fn default_dark_mode() -> String {
    // Dark by default; user choice is saved in state.json and restored on next launch.
    "DARK".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TireLayoutStored {
    pub fl: String,
    pub fr: String,
    pub rl: String,
    pub rr: String,
}

/// ENGINE_AIR_FILTER | CABIN_FILTER | OIL_FILTER | OIL_TYPE | OTHER
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartEntry {
    pub id: u64,
    pub vehicle_id: u64,
    pub part_type: String,
    pub brand: String,
    pub part_number: String,
    pub oil_viscosity: String,
    pub notes: String,
    pub installed_epoch_ms: Option<i64>,
    pub installed_mileage: Option<u32>,
}

/// BATTERY | WIPER_DRIVER (left) | WIPER_PASSENGER (right) | WIPER_REAR
/// | BRAKE_PADS_FRONT | BRAKE_PADS_REAR | BRAKE_FLUID
/// Legacy: WIPER_FRONT may exist in old saves (shown under driver if present).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub id: u64,
    pub vehicle_id: u64,
    pub component_type: String,
    pub installed_epoch_ms: Option<i64>,
    pub installed_mileage: Option<u32>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    pub id: u64,
    pub vehicle_id: u64,
    pub title: String,
    pub body: String,
    pub updated_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderEntry {
    pub id: u64,
    pub vehicle_id: u64,
    pub title: String,
    pub due_epoch_ms: Option<i64>,
    pub due_mileage: Option<u32>,
    pub completed: bool,
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
            selected_vehicle_id: None,
            next_vehicle_id: 1,
            next_service_id: 1,
            next_part_id: 1,
            next_note_id: 1,
            next_reminder_id: 1,
            next_component_id: 1,
            next_photo_id: 1,
            next_tire_purchase_id: 1,
            next_rotation_id: 1,
            vehicles: Vec::new(),
            services: Vec::new(),
            parts: Vec::new(),
            components: Vec::new(),
            notes: Vec::new(),
            reminders: Vec::new(),
            oil_level_logs: Vec::new(),
            issue_photos: Vec::new(),
            tire_purchases: Vec::new(),
            tire_rotations: Vec::new(),
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

/// Which UI features to show based on wizard mode.
#[derive(Debug, Clone, Copy)]
pub struct FeatureFlags {
    pub show_tires: bool,
    pub show_parts: bool,
    pub show_diy_trackers: bool, // battery/wipers DIY-ish + oil level
    pub show_shop_emphasis: bool,
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
            if let Ok(mut state) = serde_json::from_slice::<AppState>(&bytes) {
                state.ensure_selection();
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

    pub fn ensure_selection(&mut self) {
        if self.vehicles.is_empty() {
            self.selected_vehicle_id = None;
            return;
        }
        let ok = self
            .selected_vehicle_id
            .map(|id| self.vehicles.iter().any(|v| v.id == id))
            .unwrap_or(false);
        if !ok {
            self.selected_vehicle_id = self.vehicles.first().map(|v| v.id);
        }
    }

    pub fn selected_vehicle(&self) -> Option<&Vehicle> {
        let id = self.selected_vehicle_id?;
        self.vehicles.iter().find(|v| v.id == id)
    }

    pub fn select_vehicle(&mut self, id: u64) {
        if self.vehicles.iter().any(|v| v.id == id) {
            self.selected_vehicle_id = Some(id);
            self.save();
        }
    }

    pub fn feature_flags(&self) -> FeatureFlags {
        match UserMode::parse(&self.user_mode).unwrap_or(UserMode::Both) {
            UserMode::Diy => FeatureFlags {
                show_tires: true,
                show_parts: true,
                show_diy_trackers: true,
                show_shop_emphasis: false,
            },
            UserMode::Shop => FeatureFlags {
                show_tires: false,
                show_parts: false,
                show_diy_trackers: true, // battery/brakes still useful for shops
                show_shop_emphasis: true,
            },
            UserMode::Both => FeatureFlags {
                show_tires: true,
                show_parts: true,
                show_diy_trackers: true,
                show_shop_emphasis: true,
            },
        }
    }

    pub fn mode_label(&self) -> String {
        match UserMode::parse(&self.user_mode).unwrap_or(UserMode::Both) {
            UserMode::Diy => "DIY-focused tools ready".into(),
            UserMode::Shop => "Shop & receipt tools ready".into(),
            UserMode::Both => "Full garage toolkit".into(),
        }
    }

    pub fn services_for_selected(&self) -> Vec<&ServiceRecord> {
        let Some(vid) = self.selected_vehicle_id else {
            return Vec::new();
        };
        self.services
            .iter()
            .filter(|s| s.vehicle_id == vid)
            .collect()
    }

    pub fn last_service(&self) -> Option<&ServiceRecord> {
        self.services_for_selected()
            .into_iter()
            .max_by_key(|s| (s.date_epoch_ms, s.id))
    }

    pub fn mpg_label(&self) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return "MPG: select a vehicle".into();
        };
        let mut fills: Vec<(u32, f64)> = self
            .services
            .iter()
            .filter(|s| s.vehicle_id == vid)
            .filter_map(|s| s.gallons.map(|g| (s.mileage, g)))
            .collect();
        fills.sort_by_key(|(m, _)| *m);
        match average_mpg(&fills) {
            Some(mpg) => format!("MPG: {mpg:.1}"),
            None => "MPG: need 2+ fuel fill-ups".into(),
        }
    }

    pub fn cost_labels(&self) -> [(String, String); 3] {
        let now = Utc::now().timestamp_millis();
        let selected: Vec<ServiceRecord> = self
            .services_for_selected()
            .into_iter()
            .cloned()
            .collect();
        let s = summarize_costs(&selected, now);
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
        let before = self.tire_layout.clone();
        let layout = TireLayout::from(&self.tire_layout);
        let pattern =
            RotationPattern::from_str(&self.tire_pattern).unwrap_or(RotationPattern::ForwardCross);
        let after = apply_rotation(&layout, pattern);
        let after_stored = TireLayoutStored::from(&after);
        if let Some(vid) = self.selected_vehicle_id {
            let id = self.next_rotation_id;
            self.next_rotation_id += 1;
            let mileage = self.selected_vehicle().map(|v| v.current_mileage);
            self.tire_rotations.push(TireRotationLog {
                id,
                vehicle_id: vid,
                pattern: pattern.label().into(),
                before_fl: before.fl,
                before_fr: before.fr,
                before_rl: before.rl,
                before_rr: before.rr,
                after_fl: after_stored.fl.clone(),
                after_fr: after_stored.fr.clone(),
                after_rl: after_stored.rl.clone(),
                after_rr: after_stored.rr.clone(),
                mileage,
                date_epoch_ms: Utc::now().timestamp_millis(),
            });
        }
        self.tire_layout = after_stored;
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
        self.selected_vehicle_id = Some(id);
        // Default oil-level reminder every 3 months
        let due = oil_level_due_after(Utc::now().timestamp_millis());
        self.add_reminder_raw(
            id,
            "Oil level check".into(),
            Some(due),
            None,
        );
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
        self.add_service_full(
            title,
            mileage,
            source,
            cost,
            0.0,
            gallons,
            None,
            Utc::now().timestamp_millis(),
            String::new(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_service_full(
        &mut self,
        title: String,
        mileage: u32,
        source: &str,
        parts_cost: f64,
        labor_cost: f64,
        gallons: Option<f64>,
        fuel_cost: Option<f64>,
        date_epoch_ms: i64,
        shop_name: String,
    ) {
        if title.trim().is_empty() {
            return;
        }
        let vehicle_id = match self
            .selected_vehicle_id
            .or_else(|| self.vehicles.first().map(|v| v.id))
        {
            Some(id) => id,
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
            date_epoch_ms,
            mileage,
            title: title.trim().into(),
            source,
            labor_cost,
            parts_cost,
            gallons,
            fuel_cost,
            shop_name,
        });
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == vehicle_id) {
            if mileage > v.current_mileage {
                v.current_mileage = mileage;
            }
        }
        self.save();
    }

    /// Receipt import: date, mileage, gallons, total split into parts/labor, shop name.
    #[allow(clippy::too_many_arguments)]
    pub fn add_receipt(
        &mut self,
        title: String,
        date_str: &str,
        mileage: u32,
        gallons: Option<f64>,
        parts_cost: f64,
        labor_cost: f64,
        fuel_cost: Option<f64>,
        shop_name: String,
        source: &str,
    ) {
        let date_epoch = parse_date_to_epoch(date_str).unwrap_or_else(|| Utc::now().timestamp_millis());
        let title = if title.trim().is_empty() {
            "Receipt".into()
        } else {
            title
        };
        self.add_service_full(
            title,
            mileage,
            source,
            parts_cost,
            labor_cost,
            gallons,
            fuel_cost,
            date_epoch,
            shop_name,
        );
    }

    pub fn add_issue_photo(&mut self, caption: String, notes: String, file_path: String) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        if caption.trim().is_empty() && notes.trim().is_empty() {
            return;
        }
        let id = self.next_photo_id;
        self.next_photo_id += 1;
        let path = if file_path.trim().is_empty() {
            format!("pending-photo-{id}")
        } else {
            file_path.trim().into()
        };
        self.issue_photos.push(IssuePhoto {
            id,
            vehicle_id: vid,
            caption: if caption.trim().is_empty() {
                "Issue photo".into()
            } else {
                caption.trim().into()
            },
            notes,
            file_path: path,
            created_epoch_ms: Utc::now().timestamp_millis(),
        });
        self.save();
    }

    pub fn add_tire_purchase(
        &mut self,
        brand: String,
        model: String,
        size: String,
        cost: f64,
        mileage: Option<u32>,
        notes: String,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        let id = self.next_tire_purchase_id;
        self.next_tire_purchase_id += 1;
        self.tire_purchases.push(TirePurchase {
            id,
            vehicle_id: vid,
            brand: brand.trim().into(),
            model: model.trim().into(),
            size: size.trim().into(),
            cost,
            mileage,
            date_epoch_ms: Utc::now().timestamp_millis(),
            notes,
        });
        self.save();
    }

    pub fn upsert_part(
        &mut self,
        part_type: String,
        brand: String,
        part_number: String,
        oil_viscosity: String,
        notes: String,
        mileage: Option<u32>,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        if let Some(existing) = self
            .parts
            .iter_mut()
            .find(|p| p.vehicle_id == vid && p.part_type == part_type)
        {
            existing.brand = brand;
            existing.part_number = part_number;
            existing.oil_viscosity = oil_viscosity;
            existing.notes = notes;
            existing.installed_epoch_ms = Some(Utc::now().timestamp_millis());
            existing.installed_mileage = mileage;
        } else {
            let id = self.next_part_id;
            self.next_part_id += 1;
            self.parts.push(PartEntry {
                id,
                vehicle_id: vid,
                part_type,
                brand,
                part_number,
                oil_viscosity,
                notes,
                installed_epoch_ms: Some(Utc::now().timestamp_millis()),
                installed_mileage: mileage,
            });
        }
        self.save();
    }

    pub fn upsert_component(
        &mut self,
        component_type: String,
        notes: String,
        mileage: Option<u32>,
        installed_date_str: &str,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        let installed = parse_date_to_epoch(installed_date_str)
            .or_else(|| Some(Utc::now().timestamp_millis()));
        if let Some(existing) = self
            .components
            .iter_mut()
            .find(|c| c.vehicle_id == vid && c.component_type == component_type)
        {
            existing.notes = notes;
            existing.installed_epoch_ms = installed;
            existing.installed_mileage = mileage;
        } else {
            let id = self.next_component_id;
            self.next_component_id += 1;
            self.components.push(ComponentEntry {
                id,
                vehicle_id: vid,
                component_type,
                installed_epoch_ms: installed,
                installed_mileage: mileage,
                notes,
            });
        }
        self.save();
    }

    pub fn add_note(&mut self, title: String, body: String) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        if title.trim().is_empty() {
            return;
        }
        let id = self.next_note_id;
        self.next_note_id += 1;
        self.notes.push(NoteEntry {
            id,
            vehicle_id: vid,
            title: title.trim().into(),
            body,
            updated_epoch_ms: Utc::now().timestamp_millis(),
        });
        self.save();
    }

    fn add_reminder_raw(
        &mut self,
        vehicle_id: u64,
        title: String,
        due_epoch_ms: Option<i64>,
        due_mileage: Option<u32>,
    ) {
        let id = self.next_reminder_id;
        self.next_reminder_id += 1;
        self.reminders.push(ReminderEntry {
            id,
            vehicle_id,
            title,
            due_epoch_ms,
            due_mileage,
            completed: false,
        });
    }

    pub fn add_reminder(
        &mut self,
        title: String,
        due_date: &str,
        due_mileage: Option<u32>,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        if title.trim().is_empty() {
            return;
        }
        let due_epoch = parse_date_to_epoch(due_date);
        self.add_reminder_raw(vid, title.trim().into(), due_epoch, due_mileage);
        self.save();
    }

    /// Complete a reminder. For oil-level checks, pass the dipstick reading
    /// (e.g. "Full", "1 quart low") so it is logged and shown later.
    pub fn complete_reminder(&mut self, id: u64, oil_level: Option<&str>) {
        let Some(r) = self.reminders.iter_mut().find(|r| r.id == id) else {
            return;
        };
        let is_oil = r.title.to_lowercase().contains("oil level");
        let vid = r.vehicle_id;
        r.completed = true;

        if is_oil {
            let level = normalize_oil_level(oil_level.unwrap_or("Full"));
            let mileage = self
                .vehicles
                .iter()
                .find(|v| v.id == vid)
                .map(|v| v.current_mileage);
            self.oil_level_logs.push(OilLevelLog {
                vehicle_id: vid,
                epoch_ms: Utc::now().timestamp_millis(),
                level: level.clone(),
                mileage,
            });
            // Keep last ~50 readings per app
            if self.oil_level_logs.len() > 50 {
                let drop_n = self.oil_level_logs.len() - 50;
                self.oil_level_logs.drain(0..drop_n);
            }
            let next = oil_level_due_after(Utc::now().timestamp_millis());
            self.add_reminder_raw(vid, "Oil level check".into(), Some(next), None);
        }
        self.save();
    }

    /// Latest oil-level reading for the selected vehicle, for UI display.
    pub fn last_oil_level_summary(&self) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return "No vehicle selected.".into();
        };
        self.oil_level_logs
            .iter()
            .rev()
            .find(|l| l.vehicle_id == vid)
            .map(|l| {
                let date = match Utc.timestamp_millis_opt(l.epoch_ms) {
                    chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
                    _ => "—".into(),
                };
                let mi = l
                    .mileage
                    .map(|m| format!(" · {m} mi"))
                    .unwrap_or_default();
                format!("Last check: {}{} — {}", date, mi, l.level)
            })
            .unwrap_or_else(|| "No oil level logged yet.".into())
    }

    pub fn is_oil_level_reminder(title: &str) -> bool {
        title.to_lowercase().contains("oil level")
    }

    pub fn component_summary(&self, component_type: &str) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return "Select a vehicle first.".into();
        };
        // Prefer exact type; fall back to legacy WIPER_FRONT for driver side.
        let entry = self
            .components
            .iter()
            .find(|c| c.vehicle_id == vid && c.component_type == component_type)
            .or_else(|| {
                if component_type == "WIPER_DRIVER" {
                    self.components.iter().find(|c| {
                        c.vehicle_id == vid && c.component_type == "WIPER_FRONT"
                    })
                } else {
                    None
                }
            });
        match entry {
            Some(c) => {
                let date = c
                    .installed_epoch_ms
                    .map(format_epoch)
                    .unwrap_or_else(|| "—".into());
                let mi = c
                    .installed_mileage
                    .map(|m| format!("{m} mi"))
                    .unwrap_or_else(|| "—".into());
                let size = if c.notes.trim().is_empty() {
                    "size not set".into()
                } else {
                    c.notes.trim().to_string()
                };
                format!("Installed: {date} · {mi}\nSize / notes: {size}")
            }
            None => "No entry yet — set size (inches) if different from other sides.".into(),
        }
    }

    pub fn part_summary(&self, part_type: &str) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return "Select a vehicle first.".into();
        };
        match self
            .parts
            .iter()
            .find(|p| p.vehicle_id == vid && p.part_type == part_type)
        {
            Some(p) => {
                let mut s = format!("{} {}", p.brand, p.part_number);
                if !p.oil_viscosity.is_empty() {
                    s.push_str(&format!(" · {}", p.oil_viscosity));
                }
                if !p.notes.is_empty() {
                    s.push_str(&format!("\n{}", p.notes));
                }
                s
            }
            None => "No entry yet.".into(),
        }
    }

    pub fn open_reminders_for_selected(&self) -> Vec<&ReminderEntry> {
        let Some(vid) = self.selected_vehicle_id else {
            return Vec::new();
        };
        let mileage = self.selected_vehicle().map(|v| v.current_mileage).unwrap_or(0);
        let now = Utc::now().timestamp_millis();
        let mut list: Vec<&ReminderEntry> = self
            .reminders
            .iter()
            .filter(|r| r.vehicle_id == vid && !r.completed)
            .collect();
        list.sort_by_key(|r| {
            let due = is_due_by_date(r.due_epoch_ms, now)
                || is_due_by_mileage(r.due_mileage, mileage);
            (!due, r.due_epoch_ms.unwrap_or(i64::MAX), r.id)
        });
        list
    }
}

fn parse_date_to_epoch(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Accept YYYY-MM-DD
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() == 3 {
        let y: i32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        let d: u32 = parts[2].parse().ok()?;
        return Utc
            .with_ymd_and_hms(y, m, d, 12, 0, 0)
            .single()
            .map(|dt| dt.timestamp_millis());
    }
    None
}

fn format_epoch(ms: i64) -> String {
    match Utc.timestamp_millis_opt(ms) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => String::new(),
    }
}

pub fn reminder_status_line(r: &ReminderEntry, current_mileage: u32) -> String {
    let now = Utc::now().timestamp_millis();
    let mut bits = Vec::new();
    if let Some(ms) = r.due_epoch_ms {
        bits.push(format!("due {}", format_epoch(ms)));
        if is_due_by_date(Some(ms), now) {
            bits.push("OVERDUE".into());
        }
    }
    if let Some(m) = r.due_mileage {
        bits.push(format!("at {m} mi"));
        if is_due_by_mileage(Some(m), current_mileage) {
            bits.push("DUE BY MILEAGE".into());
        }
    }
    if bits.is_empty() {
        "no schedule".into()
    } else {
        bits.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_vehicle_services_scoped() {
        let mut s = AppState::default();
        s.add_vehicle("A".into(), "".into(), "".into(), None, 1000);
        let a = s.selected_vehicle_id.unwrap();
        s.add_vehicle("B".into(), "".into(), "".into(), None, 2000);
        let b = s.selected_vehicle_id.unwrap();
        assert_ne!(a, b);
        s.select_vehicle(a);
        s.add_service("Oil".into(), 1100, "DIY", 40.0, None);
        s.select_vehicle(b);
        s.add_service("Shop".into(), 2100, "SHOP", 200.0, None);
        s.select_vehicle(a);
        assert_eq!(s.services_for_selected().len(), 1);
        assert_eq!(s.services_for_selected()[0].title, "Oil");
    }

    #[test]
    fn rotation_changes_layout() {
        let mut s = AppState::default();
        s.tire_pattern = "side_to_side".into();
        s.apply_tire_rotation();
        assert_eq!(s.tire_layout.fl, "B");
        assert_eq!(s.tire_layout.fr, "A");
    }

    #[test]
    fn shop_mode_hides_tires() {
        let mut s = AppState::default();
        s.user_mode = "SHOP".into();
        assert!(!s.feature_flags().show_tires);
        assert!(!s.feature_flags().show_parts);
    }

    #[test]
    fn oil_level_logged_on_complete() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 50000);
        let oil_id = s
            .reminders
            .iter()
            .find(|r| r.title.contains("Oil level"))
            .map(|r| r.id)
            .expect("oil reminder");
        s.complete_reminder(oil_id, Some("1 quart low"));
        assert_eq!(s.oil_level_logs.len(), 1);
        assert_eq!(s.oil_level_logs[0].level, "1 quart low");
        assert!(s.last_oil_level_summary().contains("1 quart low"));
        // Next oil-level reminder scheduled
        assert!(s
            .reminders
            .iter()
            .any(|r| !r.completed && r.title.contains("Oil level")));
    }
}
