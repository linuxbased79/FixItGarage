//! Local-first in-memory + JSON persistence for the Slint UI.

use chrono::{TimeZone, Utc};
use fixitgarage_core::models::{ServiceRecord, ServiceSource, UserMode, Vehicle};
use fixitgarage_core::reminders::{is_due_by_date, is_due_by_mileage, oil_level_due_after};
use fixitgarage_core::{
    apply_rotation, average_mpg, map_corners, services_to_csv, summarize_costs, RotationPattern,
    TireLayout,
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
    /// Tread depth in mm per corner, per vehicle (manual or future camera).
    #[serde(default)]
    pub tread_by_vehicle: Vec<VehicleTread>,
    /// Odometer reading "on this tire" per corner (spec: mileage per tire).
    #[serde(default)]
    pub tire_mileage_by_vehicle: Vec<VehicleTireMileage>,
    /// Optional Nextcloud/ownCloud/WebDAV backup (local-first; cloud is opt-in).
    #[serde(default)]
    pub cloud_webdav_url: String,
    #[serde(default)]
    pub cloud_username: String,
    #[serde(default)]
    pub cloud_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TreadDepths {
    pub fl: Option<f64>,
    pub fr: Option<f64>,
    pub rl: Option<f64>,
    pub rr: Option<f64>,
    pub measured_epoch_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VehicleTread {
    pub vehicle_id: u64,
    pub depths: TreadDepths,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TireCornerMiles {
    pub fl: Option<u32>,
    pub fr: Option<u32>,
    pub rl: Option<u32>,
    pub rr: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VehicleTireMileage {
    pub vehicle_id: u64,
    pub miles: TireCornerMiles,
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
            tread_by_vehicle: Vec::new(),
            tire_mileage_by_vehicle: Vec::new(),
            cloud_webdav_url: String::new(),
            cloud_username: String::new(),
            cloud_password: String::new(),
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
                state.ensure_oil_reminders();
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
        match self.average_mpg() {
            Some(mpg) => format!("MPG: {mpg:.1}"),
            None => {
                if self.selected_vehicle_id.is_none() {
                    "MPG: select a vehicle".into()
                } else {
                    "MPG: need 2+ fuel fill-ups".into()
                }
            }
        }
    }

    pub fn average_mpg(&self) -> Option<f64> {
        let vid = self.selected_vehicle_id?;
        let mut fills: Vec<(u32, f64)> = self
            .services
            .iter()
            .filter(|s| s.vehicle_id == vid)
            .filter_map(|s| s.gallons.map(|g| (s.mileage, g)))
            .collect();
        fills.sort_by_key(|(m, _)| *m);
        average_mpg(&fills)
    }

    /// Dashboard numbers for the home screen.
    pub fn dashboard_lines(&self) -> (String, String, String, String) {
        let mpg = self
            .average_mpg()
            .map(|m| format!("{m:.1} MPG avg"))
            .unwrap_or_else(|| "MPG: —".into());
        let costs = self.cost_labels();
        let month = format!("This month: {}", costs[0].1);
        let year = format!("This year: {}", costs[1].1);
        let dues = if self.has_due_reminders() {
            self.due_reminders_summary()
        } else {
            "Reminders: all clear".into()
        };
        (mpg, month, year, dues)
    }

    /// Fuel fill-up history with per-leg MPG where possible.
    pub fn fuel_history_lines(&self) -> Vec<(String, String)> {
        let Some(vid) = self.selected_vehicle_id else {
            return Vec::new();
        };
        let mut fills: Vec<&ServiceRecord> = self
            .services
            .iter()
            .filter(|s| s.vehicle_id == vid && s.gallons.map(|g| g > 0.0).unwrap_or(false))
            .collect();
        fills.sort_by_key(|s| s.mileage);
        let mut out = Vec::new();
        for i in 0..fills.len() {
            let s = fills[i];
            let date = format_epoch(s.date_epoch_ms);
            let gal = s.gallons.unwrap_or(0.0);
            let mut detail = format!("{} mi · {gal:.2} gal", s.mileage);
            if let Some(fc) = s.fuel_cost {
                detail.push_str(&format!(" · ${fc:.2}"));
            }
            if i > 0 {
                let prev = fills[i - 1];
                let miles = s.mileage.saturating_sub(prev.mileage);
                if miles > 0 && gal > 0.0 {
                    let leg = f64::from(miles) / gal;
                    detail.push_str(&format!(" · {leg:.1} MPG this fill"));
                }
            }
            out.push((format!("{date} — {}", s.title), detail));
        }
        out.reverse();
        out
    }

    pub fn filtered_services(&self, query: &str) -> Vec<&ServiceRecord> {
        let q = query.trim().to_ascii_lowercase();
        let mut list = self.services_for_selected();
        if !q.is_empty() {
            list.retain(|s| {
                s.title.to_ascii_lowercase().contains(&q)
                    || s.shop_name.to_ascii_lowercase().contains(&q)
                    || s.source.as_str().to_ascii_lowercase().contains(&q)
                    || s.mileage.to_string().contains(&q)
            });
        }
        list.sort_by(|a, b| b.date_epoch_ms.cmp(&a.date_epoch_ms).then(b.id.cmp(&a.id)));
        list
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
        let after = self.preview_after_layout();
        let pattern =
            RotationPattern::from_str(&self.tire_pattern).unwrap_or(RotationPattern::ForwardCross);
        format!(
            "After {}: {} {} / {} {}",
            pattern.label(),
            after.fl,
            after.fr,
            after.rl,
            after.rr
        )
    }

    /// Positions after applying the currently selected pattern (preview only).
    pub fn preview_after_layout(&self) -> TireLayout {
        let layout = TireLayout::from(&self.tire_layout);
        let pattern =
            RotationPattern::from_str(&self.tire_pattern).unwrap_or(RotationPattern::ForwardCross);
        apply_rotation(&layout, pattern)
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
            // Spec: mileage per tire + tread follow the rubber through rotation
            self.remap_corner_data_for_vehicle(vid, pattern);
        }
        self.tire_layout = after_stored;
    }

    fn remap_corner_data_for_vehicle(&mut self, vehicle_id: u64, pattern: RotationPattern) {
        if let Some(slot) = self
            .tire_mileage_by_vehicle
            .iter_mut()
            .find(|t| t.vehicle_id == vehicle_id)
        {
            let (fl, fr, rl, rr) = map_corners(
                &slot.miles.fl,
                &slot.miles.fr,
                &slot.miles.rl,
                &slot.miles.rr,
                pattern,
            );
            slot.miles = TireCornerMiles { fl, fr, rl, rr };
        }
        if let Some(slot) = self
            .tread_by_vehicle
            .iter_mut()
            .find(|t| t.vehicle_id == vehicle_id)
        {
            let (fl, fr, rl, rr) = map_corners(
                &slot.depths.fl,
                &slot.depths.fr,
                &slot.depths.rl,
                &slot.depths.rr,
                pattern,
            );
            slot.depths.fl = fl;
            slot.depths.fr = fr;
            slot.depths.rl = rl;
            slot.depths.rr = rr;
        }
    }

    pub fn export_csv(&self) -> String {
        self.export_all_csv()
    }

    /// Multi-section CSV for backup/share (services + vehicles + trackers).
    pub fn export_all_csv(&self) -> String {
        let mut out = String::new();
        out.push_str("### vehicles\n");
        out.push_str("id,name,make,model,year,mileage\n");
        for v in &self.vehicles {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                v.id,
                csv_esc(&v.name),
                csv_esc(&v.make),
                csv_esc(&v.model),
                v.year.map(|y| y.to_string()).unwrap_or_default(),
                v.current_mileage
            ));
        }
        out.push('\n');
        out.push_str("### services\n");
        out.push_str(&services_to_csv(&self.services));
        out.push('\n');
        out.push_str("### parts\n");
        out.push_str("id,vehicleId,type,brand,partNumber,oilViscosity,notes,mileage\n");
        for p in &self.parts {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                p.id,
                p.vehicle_id,
                csv_esc(&p.part_type),
                csv_esc(&p.brand),
                csv_esc(&p.part_number),
                csv_esc(&p.oil_viscosity),
                csv_esc(&p.notes),
                p.installed_mileage.map(|m| m.to_string()).unwrap_or_default()
            ));
        }
        out.push('\n');
        out.push_str("### oil_level_logs\n");
        out.push_str("vehicleId,date,level,mileage\n");
        for o in &self.oil_level_logs {
            out.push_str(&format!(
                "{},{},{},{}\n",
                o.vehicle_id,
                format_epoch(o.epoch_ms),
                csv_esc(&o.level),
                o.mileage.map(|m| m.to_string()).unwrap_or_default()
            ));
        }
        out.push('\n');
        out.push_str("### components\n");
        out.push_str("id,vehicleId,type,date,mileage,notes\n");
        for c in &self.components {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                c.id,
                c.vehicle_id,
                csv_esc(&c.component_type),
                c.installed_epoch_ms
                    .map(format_epoch)
                    .unwrap_or_default(),
                c.installed_mileage.map(|m| m.to_string()).unwrap_or_default(),
                csv_esc(&c.notes)
            ));
        }
        out.push('\n');
        out.push_str("### tire_purchases\n");
        out.push_str("id,vehicleId,brand,model,size,cost,mileage,date,notes\n");
        for t in &self.tire_purchases {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                t.id,
                t.vehicle_id,
                csv_esc(&t.brand),
                csv_esc(&t.model),
                csv_esc(&t.size),
                t.cost,
                t.mileage.map(|m| m.to_string()).unwrap_or_default(),
                format_epoch(t.date_epoch_ms),
                csv_esc(&t.notes)
            ));
        }
        out.push('\n');
        out.push_str("### notes\n");
        out.push_str("id,vehicleId,title,body,updated\n");
        for n in &self.notes {
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                n.id,
                n.vehicle_id,
                csv_esc(&n.title),
                csv_esc(&n.body),
                format_epoch(n.updated_epoch_ms)
            ));
        }
        out
    }

    pub fn update_vehicle_mileage(&mut self, id: u64, mileage: u32) {
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == id) {
            v.current_mileage = mileage;
            self.save();
        }
    }

    pub fn update_selected_vehicle_details(
        &mut self,
        name: String,
        make: String,
        model: String,
        year: Option<u16>,
        mileage: Option<u32>,
    ) {
        let Some(id) = self.selected_vehicle_id else {
            return;
        };
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == id) {
            if !name.trim().is_empty() {
                v.name = name.trim().into();
            }
            v.make = make.trim().into();
            v.model = model.trim().into();
            if year.is_some() {
                v.year = year;
            }
            if let Some(m) = mileage {
                v.current_mileage = m;
            }
            self.save();
        }
    }

    pub fn set_cloud_settings(&mut self, url: String, user: String, pass: String) {
        self.cloud_webdav_url = url.trim().into();
        self.cloud_username = user.trim().into();
        self.cloud_password = pass; // stored only on device in state.json
        self.save();
    }

    pub fn delete_vehicle(&mut self, id: u64) {
        self.vehicles.retain(|v| v.id != id);
        self.services.retain(|s| s.vehicle_id != id);
        self.parts.retain(|p| p.vehicle_id != id);
        self.components.retain(|c| c.vehicle_id != id);
        self.notes.retain(|n| n.vehicle_id != id);
        self.reminders.retain(|r| r.vehicle_id != id);
        self.oil_level_logs.retain(|o| o.vehicle_id != id);
        self.issue_photos.retain(|p| p.vehicle_id != id);
        self.tire_purchases.retain(|t| t.vehicle_id != id);
        self.tire_rotations.retain(|t| t.vehicle_id != id);
        self.tread_by_vehicle.retain(|t| t.vehicle_id != id);
        self.tire_mileage_by_vehicle.retain(|t| t.vehicle_id != id);
        if self.selected_vehicle_id == Some(id) {
            self.selected_vehicle_id = self.vehicles.first().map(|v| v.id);
        }
        self.save();
    }

    pub fn delete_service(&mut self, id: u64) {
        self.services.retain(|s| s.id != id);
        self.save();
    }

    pub fn delete_note(&mut self, id: u64) {
        self.notes.retain(|n| n.id != id);
        self.save();
    }

    pub fn set_tread_depths(&mut self, fl: Option<f64>, fr: Option<f64>, rl: Option<f64>, rr: Option<f64>) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        let depths = TreadDepths {
            fl,
            fr,
            rl,
            rr,
            measured_epoch_ms: Some(Utc::now().timestamp_millis()),
        };
        if let Some(slot) = self.tread_by_vehicle.iter_mut().find(|t| t.vehicle_id == vid) {
            slot.depths = depths;
        } else {
            self.tread_by_vehicle.push(VehicleTread {
                vehicle_id: vid,
                depths,
            });
        }
        self.save();
    }

    pub fn tread_for_selected(&self) -> TreadDepths {
        let Some(vid) = self.selected_vehicle_id else {
            return TreadDepths::default();
        };
        self.tread_by_vehicle
            .iter()
            .find(|t| t.vehicle_id == vid)
            .map(|t| t.depths.clone())
            .unwrap_or_default()
    }

    pub fn tread_summary(&self) -> String {
        let t = self.tread_for_selected();
        let fmt = |v: Option<f64>| {
            v.map(|x| format!("{x:.1} mm"))
                .unwrap_or_else(|| "—".into())
        };
        let when = t
            .measured_epoch_ms
            .map(format_epoch)
            .unwrap_or_else(|| "never".into());
        format!(
            "FL {} · FR {} · RL {} · RR {} (measured {})",
            fmt(t.fl),
            fmt(t.fr),
            fmt(t.rl),
            fmt(t.rr),
            when
        )
    }

    /// Due / overdue open reminders for selected vehicle (for home banner).
    pub fn due_reminders_summary(&self) -> String {
        let mileage = self.selected_vehicle().map(|v| v.current_mileage).unwrap_or(0);
        let now = Utc::now().timestamp_millis();
        let due: Vec<_> = self
            .open_reminders_for_selected()
            .into_iter()
            .filter(|r| {
                is_due_by_date(r.due_epoch_ms, now) || is_due_by_mileage(r.due_mileage, mileage)
            })
            .collect();
        if due.is_empty() {
            "No due reminders.".into()
        } else {
            let titles: Vec<_> = due.iter().map(|r| r.title.as_str()).collect();
            format!("{} due: {}", due.len(), titles.join(", "))
        }
    }

    pub fn has_due_reminders(&self) -> bool {
        let mileage = self.selected_vehicle().map(|v| v.current_mileage).unwrap_or(0);
        let now = Utc::now().timestamp_millis();
        self.open_reminders_for_selected().into_iter().any(|r| {
            is_due_by_date(r.due_epoch_ms, now) || is_due_by_mileage(r.due_mileage, mileage)
        })
    }

    /// US/EU common legal minimum for passenger tires (~2/32" ≈ 1.6 mm).
    pub const TREAD_MIN_MM: f64 = 1.6;

    pub fn tread_warning(&self) -> String {
        let t = self.tread_for_selected();
        let mut low = Vec::new();
        let mut check = |label: &str, v: Option<f64>| {
            if let Some(mm) = v {
                if mm < Self::TREAD_MIN_MM {
                    low.push(format!("{label} {mm:.1} mm"));
                }
            }
        };
        check("FL", t.fl);
        check("FR", t.fr);
        check("RL", t.rl);
        check("RR", t.rr);
        if low.is_empty() {
            if t.measured_epoch_ms.is_none() {
                "No tread measurement yet. Legal minimum is usually 1.6 mm (2/32\").".into()
            } else {
                "All measured corners are at or above 1.6 mm.".into()
            }
        } else {
            format!(
                "⚠ Below 1.6 mm (replace soon): {}",
                low.join(", ")
            )
        }
    }

    pub fn has_low_tread(&self) -> bool {
        let t = self.tread_for_selected();
        [t.fl, t.fr, t.rl, t.rr]
            .into_iter()
            .flatten()
            .any(|mm| mm < Self::TREAD_MIN_MM)
    }

    /// Write full app state JSON backup. Returns path written.
    pub fn write_backup_file(&self) -> Result<PathBuf, String> {
        let dir = Self::data_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&dir);
        let stamp = Utc::now().format("%Y%m%d-%H%M%S");
        let path = dir.join(format!("fixitgarage-backup-{stamp}.json"));
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Restore state from a backup JSON file path.
    pub fn restore_from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path.trim()).map_err(|e| format!("read: {e}"))?;
        let mut state: AppState =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
        state.ensure_selection();
        state.ensure_oil_reminders();
        state.save();
        Ok(state)
    }

    /// Ensure every vehicle has an open oil-level reminder (for older data).
    pub fn ensure_oil_reminders(&mut self) {
        let mut changed = false;
        for v in self.vehicles.clone() {
            let has_open = self.reminders.iter().any(|r| {
                r.vehicle_id == v.id
                    && !r.completed
                    && r.title.to_lowercase().contains("oil level")
            });
            if !has_open {
                let due = oil_level_due_after(Utc::now().timestamp_millis());
                self.add_reminder_raw(v.id, "Oil level check".into(), Some(due), None);
                changed = true;
            }
        }
        if changed {
            self.save();
        }
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
        let ctype = component_type.clone();
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
        // Spec: smart reminders for brakes/battery/wipers by date + mileage
        self.schedule_component_reminder(vid, &ctype, installed, mileage);
        self.save();
    }

    fn schedule_component_reminder(
        &mut self,
        vehicle_id: u64,
        component_type: &str,
        installed_epoch_ms: Option<i64>,
        installed_mileage: Option<u32>,
    ) {
        let (title, due_date, due_mi): (String, Option<i64>, Option<u32>) = match component_type {
            "BATTERY" => (
                "Replace battery".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 48)), // ~4 years
                None,
            ),
            "BRAKE_PADS_FRONT" => (
                "Front brake pads service".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 24)),
                installed_mileage.map(|m| m.saturating_add(30_000)),
            ),
            "BRAKE_PADS_REAR" => (
                "Rear brake pads service".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 24)),
                installed_mileage.map(|m| m.saturating_add(30_000)),
            ),
            "BRAKE_FLUID" => (
                "Brake fluid flush".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 24)),
                None,
            ),
            "WIPER_DRIVER" | "WIPER_PASSENGER" | "WIPER_REAR" | "WIPER_FRONT" => (
                format!("Replace {component_type} blades").replace('_', " "),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 12)),
                None,
            ),
            _ => return,
        };
        // Replace prior open reminder with same title for this vehicle
        self.reminders.retain(|r| {
            !(r.vehicle_id == vehicle_id && !r.completed && r.title == title)
        });
        self.add_reminder_raw(vehicle_id, title, due_date, due_mi);
    }

    pub fn battery_age_warning(&self) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return String::new();
        };
        let Some(c) = self
            .components
            .iter()
            .find(|c| c.vehicle_id == vid && c.component_type == "BATTERY")
        else {
            return "No battery install date logged.".into();
        };
        let Some(ms) = c.installed_epoch_ms else {
            return "Battery age unknown.".into();
        };
        let age_days = (Utc::now().timestamp_millis() - ms) / (86_400_000);
        let years = age_days as f64 / 365.25;
        if years >= 4.0 {
            format!("⚠ Battery ~{years:.1} years old — plan replacement soon.")
        } else if years >= 3.0 {
            format!("Battery age ~{years:.1} years — monitor starting performance.")
        } else {
            format!("Battery age ~{years:.1} years.")
        }
    }

    pub fn has_old_battery(&self) -> bool {
        self.battery_age_warning().contains('⚠')
    }

    /// Home alert when front/rear pads or fluid auto-reminders are due (spec: brake tracker + reminders).
    pub fn brake_due_warning(&self) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return String::new();
        };
        let now = Utc::now().timestamp_millis();
        let odo = self.selected_vehicle().map(|v| v.current_mileage).unwrap_or(0);
        let mut due: Vec<&str> = Vec::new();
        for r in &self.reminders {
            if r.vehicle_id != vid || r.completed {
                continue;
            }
            let title_l = r.title.to_ascii_lowercase();
            if !title_l.contains("brake") {
                continue;
            }
            let date_due = is_due_by_date(r.due_epoch_ms, now);
            let mi_due = is_due_by_mileage(r.due_mileage, odo);
            if date_due || mi_due {
                due.push(r.title.as_str());
            }
        }
        if due.is_empty() {
            // Age check from component install even if reminder was cleared
            for ctype in ["BRAKE_PADS_FRONT", "BRAKE_PADS_REAR", "BRAKE_FLUID"] {
                if let Some(c) = self
                    .components
                    .iter()
                    .find(|c| c.vehicle_id == vid && c.component_type == ctype)
                {
                    if let Some(ms) = c.installed_epoch_ms {
                        let years =
                            (now - ms) as f64 / (86_400_000.0 * 365.25);
                        if years >= 2.0 {
                            due.push(ctype);
                        }
                    }
                    if let Some(mi) = c.installed_mileage {
                        if odo.saturating_sub(mi) >= 30_000
                            && (ctype == "BRAKE_PADS_FRONT" || ctype == "BRAKE_PADS_REAR")
                        {
                            due.push(ctype);
                        }
                    }
                }
            }
        }
        if due.is_empty() {
            "Brakes look on schedule.".into()
        } else {
            format!("⚠ Brake service due: {}", due.join(", "))
        }
    }

    pub fn has_brakes_due(&self) -> bool {
        self.brake_due_warning().contains('⚠')
    }

    /// Wiper blades past ~1 year install age.
    pub fn wiper_due_warning(&self) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return String::new();
        };
        let now = Utc::now().timestamp_millis();
        let mut old = Vec::new();
        for ctype in ["WIPER_DRIVER", "WIPER_PASSENGER", "WIPER_REAR", "WIPER_FRONT"] {
            if let Some(c) = self
                .components
                .iter()
                .find(|c| c.vehicle_id == vid && c.component_type == ctype)
            {
                if let Some(ms) = c.installed_epoch_ms {
                    let years = (now - ms) as f64 / (86_400_000.0 * 365.25);
                    if years >= 1.0 {
                        old.push(format!(
                            "{} (~{years:.1}y)",
                            ctype.replace("WIPER_", "").replace('_', " ")
                        ));
                    }
                }
            }
        }
        if old.is_empty() {
            "Wipers look fresh.".into()
        } else {
            format!("⚠ Replace wiper blades: {}", old.join(", "))
        }
    }

    pub fn has_old_wipers(&self) -> bool {
        self.wiper_due_warning().contains('⚠')
    }

    /// US coin gauge assist for camera tread measurement (approx legal / mid wear).
    pub fn tread_coin_guide(&self) -> String {
        "Coin gauge (camera assist): US penny Lincoln head ~1.6 mm (2/32\") wear limit; \
         quarter Washington head ~3.2 mm (4/32\") mid wear. Place coin in groove, photo, then enter mm."
            .into()
    }

    pub fn set_tire_corner_miles(
        &mut self,
        fl: Option<u32>,
        fr: Option<u32>,
        rl: Option<u32>,
        rr: Option<u32>,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        let miles = TireCornerMiles { fl, fr, rl, rr };
        if let Some(slot) = self
            .tire_mileage_by_vehicle
            .iter_mut()
            .find(|t| t.vehicle_id == vid)
        {
            slot.miles = miles;
        } else {
            self.tire_mileage_by_vehicle.push(VehicleTireMileage {
                vehicle_id: vid,
                miles,
            });
        }
        self.save();
    }

    pub fn tire_miles_for_selected(&self) -> TireCornerMiles {
        let Some(vid) = self.selected_vehicle_id else {
            return TireCornerMiles::default();
        };
        self.tire_mileage_by_vehicle
            .iter()
            .find(|t| t.vehicle_id == vid)
            .map(|t| t.miles.clone())
            .unwrap_or_default()
    }

    pub fn tire_miles_summary(&self) -> String {
        let m = self.tire_miles_for_selected();
        let f = |v: Option<u32>| v.map(|x| format!("{x} mi")).unwrap_or_else(|| "—".into());
        format!(
            "FL {} · FR {} · RL {} · RR {}",
            f(m.fl),
            f(m.fr),
            f(m.rl),
            f(m.rr)
        )
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

fn csv_esc(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Approximate calendar months (30.44 days) for reminder scheduling.
fn add_months_approx(epoch_ms: i64, months: i32) -> i64 {
    let days = (f64::from(months) * 30.44) as i64;
    epoch_ms + days * 86_400_000
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
    fn rotation_moves_mileage_with_tires() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 10000);
        s.set_tire_corner_miles(Some(1), Some(2), Some(3), Some(4));
        s.tire_pattern = "side_to_side".into();
        s.apply_tire_rotation();
        let m = s.tire_miles_for_selected();
        assert_eq!(m.fl, Some(2));
        assert_eq!(m.fr, Some(1));
        assert_eq!(m.rl, Some(4));
        assert_eq!(m.rr, Some(3));
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
