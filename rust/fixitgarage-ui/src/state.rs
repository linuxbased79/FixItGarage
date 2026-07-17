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

/// BATTERY | WIPER_FRONT | WIPER_REAR | BRAKE_PADS_FRONT | BRAKE_PADS_REAR | BRAKE_FLUID
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
            vehicles: Vec::new(),
            services: Vec::new(),
            parts: Vec::new(),
            components: Vec::new(),
            notes: Vec::new(),
            reminders: Vec::new(),
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
        if title.trim().is_empty() {
            return;
        }
        let vehicle_id = match self.selected_vehicle_id.or_else(|| self.vehicles.first().map(|v| v.id))
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

    pub fn complete_reminder(&mut self, id: u64) {
        if let Some(r) = self.reminders.iter_mut().find(|r| r.id == id) {
            r.completed = true;
            // If oil level, schedule next in 3 months
            if r.title.to_lowercase().contains("oil level") {
                let vid = r.vehicle_id;
                let next = oil_level_due_after(Utc::now().timestamp_millis());
                self.add_reminder_raw(vid, "Oil level check".into(), Some(next), None);
            }
            self.save();
        }
    }

    pub fn component_summary(&self, component_type: &str) -> String {
        let Some(vid) = self.selected_vehicle_id else {
            return "Select a vehicle first.".into();
        };
        match self
            .components
            .iter()
            .find(|c| c.vehicle_id == vid && c.component_type == component_type)
        {
            Some(c) => {
                let date = c
                    .installed_epoch_ms
                    .map(format_epoch)
                    .unwrap_or_else(|| "—".into());
                let mi = c
                    .installed_mileage
                    .map(|m| format!("{m} mi"))
                    .unwrap_or_else(|| "—".into());
                format!("Installed: {date} · {mi}\n{}", c.notes)
            }
            None => "No entry yet.".into(),
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
}
