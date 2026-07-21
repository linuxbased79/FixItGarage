//! Local-first in-memory + JSON persistence for the Slint UI.

use chrono::{Datelike, TimeZone, Utc};
use fixitgarage_core::models::{ServiceRecord, ServiceSource, UserMode, Vehicle};
use fixitgarage_core::reminders::{is_due_by_date, is_due_by_mileage, oil_level_due_after};
use fixitgarage_core::{
    apply_rotation, average_mpg, map_corners, map_corners5, services_to_csv, summarize_costs,
    RotationPattern, TireLayout,
};
use crate::units::{
    display_oil_level, format_distance, format_economy, format_fuel, format_tread,
    tread_coin_guide as units_tread_coin_guide, tread_limit_label, UnitSystem,
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
    /// IMPERIAL (mi, gal, 32nds) or METRIC (km, L, mm). Storage stays miles/gallons/mm.
    #[serde(default = "default_units")]
    pub units: String,
    /// SYSTEM (follow OS) or EN / ES / FR / DE language pack override.
    #[serde(default = "default_language")]
    pub language: String,
    /// Use OpenDyslexic font for dyslexia-friendly reading (optional).
    #[serde(default)]
    pub dyslexia_font: bool,
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
    /// Legacy global layout (migrated into tire_config_by_vehicle on load).
    pub tire_layout: TireLayoutStored,
    pub tire_pattern: String,
    /// When true, rotation includes the full-size spare (most people leave this off).
    #[serde(default)]
    pub include_spare_in_rotation: bool,
    /// Per-vehicle tire positions, pattern, and spare option.
    #[serde(default)]
    pub tire_config_by_vehicle: Vec<VehicleTireConfig>,
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
    /// Stored only in on-device `state.json` (app private storage).
    /// **Never** included in shared JSON backups (`write_backup_file` clears it).
    #[serde(default)]
    pub cloud_password: String,
    /// Throttle system notifications (ms since epoch). Re-notify after 12 hours.
    #[serde(default)]
    pub last_notified_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TreadDepths {
    pub fl: Option<f64>,
    pub fr: Option<f64>,
    pub rl: Option<f64>,
    pub rr: Option<f64>,
    #[serde(default)]
    pub spare: Option<f64>,
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
    #[serde(default)]
    pub spare: Option<u32>,
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
    #[serde(default)]
    pub before_spare: String,
    pub after_fl: String,
    pub after_fr: String,
    pub after_rl: String,
    pub after_rr: String,
    #[serde(default)]
    pub after_spare: String,
    /// True when this rotation included the full-size spare.
    #[serde(default)]
    pub included_spare: bool,
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
    // Metric wording → imperial canonical storage
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "ok" | "good" | "full / ok" => "Full".into(),
        "0.5" | "half" | "1/2" | "half quart low" | "0.5 l low" | "½ l low" | "0,5 l low" => {
            "½ quart low".into()
        }
        "1" | "1 qt" | "1 quart" | "1 l low" | "1l low" => "1 quart low".into(),
        "2" | "2 qt" | "2 quarts" | "2 l low" | "2l low" => "2 quarts low".into(),
        "3" | "3 qt" | "3 quarts" | "3 l low" | "3l low" => "3 quarts low".into(),
        "over" | "high" | "too full" => "Overfilled".into(),
        _ if !t.is_empty() => t.to_string(),
        _ => "Full".into(),
    }
}

fn default_dark_mode() -> String {
    // Dark by default; user choice is saved in state.json and restored on next launch.
    "DARK".into()
}

fn default_units() -> String {
    "IMPERIAL".into()
}

fn default_language() -> String {
    "SYSTEM".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TireLayoutStored {
    pub fl: String,
    pub fr: String,
    pub rl: String,
    pub rr: String,
    /// Full-size spare ID (default "E"). Empty string = no spare tracked.
    #[serde(default = "default_spare_label")]
    pub spare: String,
}

fn default_spare_label() -> String {
    "E".into()
}

/// Tire tracker settings for one vehicle (positions are not shared across cars).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VehicleTireConfig {
    pub vehicle_id: u64,
    pub layout: TireLayoutStored,
    pub pattern: String,
    #[serde(default)]
    pub include_spare: bool,
}

impl VehicleTireConfig {
    pub fn default_for(vehicle_id: u64) -> Self {
        Self {
            vehicle_id,
            layout: TireLayoutStored {
                fl: "A".into(),
                fr: "B".into(),
                rl: "C".into(),
                rr: "D".into(),
                spare: "E".into(),
            },
            pattern: "forward_cross".into(),
            include_spare: false,
        }
    }
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
            spare: t.spare.clone(),
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
            spare: t.spare.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            wizard_done: false,
            user_mode: "BOTH".into(),
            dark_mode: default_dark_mode(),
            units: default_units(),
            language: default_language(),
            dyslexia_font: false,
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
                spare: "E".into(),
            },
            tire_pattern: "forward_cross".into(),
            include_spare_in_rotation: false,
            tire_config_by_vehicle: Vec::new(),
            tread_by_vehicle: Vec::new(),
            tire_mileage_by_vehicle: Vec::new(),
            cloud_webdav_url: String::new(),
            cloud_username: String::new(),
            cloud_password: String::new(),
            last_notified_epoch_ms: 0,
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
        crate::platform::app_data_dir().join("state.json")
    }

    pub fn load() -> Self {
        let try_load = |p: &std::path::Path| -> Option<(AppState, std::fs::Metadata)> {
            let meta = std::fs::metadata(p).ok()?;
            let bytes = std::fs::read(p).ok()?;
            let state = serde_json::from_slice::<AppState>(&bytes).ok()?;
            Some((state, meta))
        };

        // Scan every candidate path and pick the richest / newest state file.
        // This recovers data written to the wrong place by older builds.
        let mut best: Option<(AppState, u64, std::time::SystemTime)> = None;
        let mut paths: Vec<PathBuf> = crate::platform::app_data_dir_candidates()
            .into_iter()
            .map(|d| d.join("state.json"))
            .collect();
        // Always include the canonical path first after resolve.
        paths.insert(0, Self::data_path());
        let mut seen = std::collections::HashSet::new();
        paths.retain(|p| seen.insert(p.clone()));

        for p in &paths {
            if let Some((state, meta)) = try_load(p) {
                let n = state.vehicles.len() as u64;
                let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                let replace = match &best {
                    None => true,
                    Some((_, bn, bm)) => n > *bn || (n == *bn && modified > *bm),
                };
                if replace {
                    eprintln!(
                        "FixItGarage: found state at {} ({} vehicles)",
                        p.display(),
                        n
                    );
                    best = Some((state, n, modified));
                }
            }
        }

        if let Some((mut state, n, _)) = best {
            state.ensure_selection();
            state.migrate_tire_configs();
            state.ensure_oil_reminders();
            // Re-save into the canonical durable path.
            if n > 0 || state.wizard_done {
                state.save();
            }
            return state;
        }
        eprintln!(
            "FixItGarage: no state.json found; starting empty (canonical={})",
            Self::data_path().display()
        );
        Self::default()
    }

    /// Returns true if the file was written successfully.
    pub fn save(&self) -> bool {
        let path = Self::data_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("FixItGarage: create data dir failed: {e} ({})", parent.display());
                return false;
            }
        }
        let json = match serde_json::to_vec_pretty(self) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("FixItGarage: serialize state failed: {e}");
                return false;
            }
        };

        // Direct write + fsync is more reliable on Android than rename across layers.
        match std::fs::File::create(&path) {
            Ok(mut f) => {
                use std::io::Write;
                if let Err(e) = f.write_all(&json) {
                    eprintln!("FixItGarage: write state failed: {e} ({})", path.display());
                    return false;
                }
                if let Err(e) = f.sync_all() {
                    eprintln!("FixItGarage: fsync state failed: {e} ({})", path.display());
                }
                // Verify round-trip readable
                match std::fs::read(&path) {
                    Ok(bytes) if bytes.len() == json.len() => {
                        eprintln!(
                            "FixItGarage: saved {} bytes → {} ({} vehicles)",
                            json.len(),
                            path.display(),
                            self.vehicles.len()
                        );
                        true
                    }
                    Ok(bytes) => {
                        eprintln!(
                            "FixItGarage: save size mismatch wrote {} read {}",
                            json.len(),
                            bytes.len()
                        );
                        false
                    }
                    Err(e) => {
                        eprintln!("FixItGarage: save verify read failed: {e}");
                        false
                    }
                }
            }
            Err(e) => {
                eprintln!("FixItGarage: create state file failed: {e} ({})", path.display());
                false
            }
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

    /// Move legacy global tire layout into per-vehicle configs (once).
    pub fn migrate_tire_configs(&mut self) {
        let ids: Vec<u64> = self.vehicles.iter().map(|v| v.id).collect();
        let mut changed = false;
        for id in ids {
            if self.tire_config_by_vehicle.iter().any(|c| c.vehicle_id == id) {
                continue;
            }
            // Seed first vehicle from legacy globals; others get defaults
            let seed_from_global = self.tire_config_by_vehicle.is_empty();
            let mut cfg = VehicleTireConfig::default_for(id);
            if seed_from_global {
                cfg.layout = self.tire_layout.clone();
                cfg.pattern = self.tire_pattern.clone();
                cfg.include_spare = self.include_spare_in_rotation;
            }
            self.tire_config_by_vehicle.push(cfg);
            changed = true;
        }
        if changed {
            self.save();
        }
    }

    fn ensure_tire_config(&mut self, vehicle_id: u64) {
        if !self
            .tire_config_by_vehicle
            .iter()
            .any(|c| c.vehicle_id == vehicle_id)
        {
            self.tire_config_by_vehicle
                .push(VehicleTireConfig::default_for(vehicle_id));
        }
    }

    /// Tire config for selected vehicle (read-only snapshot).
    pub fn selected_tire_config(&self) -> VehicleTireConfig {
        let Some(vid) = self.selected_vehicle_id else {
            return VehicleTireConfig::default_for(0);
        };
        self.tire_config_by_vehicle
            .iter()
            .find(|c| c.vehicle_id == vid)
            .cloned()
            .unwrap_or_else(|| VehicleTireConfig::default_for(vid))
    }

    fn tire_config_mut(&mut self) -> Option<&mut VehicleTireConfig> {
        let vid = self.selected_vehicle_id?;
        self.ensure_tire_config(vid);
        self.tire_config_by_vehicle
            .iter_mut()
            .find(|c| c.vehicle_id == vid)
    }

    pub fn selected_vehicle(&self) -> Option<&Vehicle> {
        let id = self.selected_vehicle_id?;
        self.vehicles.iter().find(|v| v.id == id)
    }

    pub fn select_vehicle(&mut self, id: u64) {
        if self.vehicles.iter().any(|v| v.id == id) {
            self.selected_vehicle_id = Some(id);
            self.ensure_tire_config(id);
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

    pub fn unit_system(&self) -> UnitSystem {
        UnitSystem::from_str_loose(&self.units)
    }

    pub fn set_units(&mut self, units: &str) {
        self.units = UnitSystem::from_str_loose(units).as_str().into();
        self.save();
    }

    pub fn set_language(&mut self, language: &str) {
        use crate::i18n::LanguagePref;
        self.language = LanguagePref::from_str_loose(language).as_str().into();
        self.save();
    }

    pub fn language_pref(&self) -> crate::i18n::LanguagePref {
        crate::i18n::LanguagePref::from_str_loose(&self.language)
    }

    pub fn set_dyslexia_font(&mut self, enabled: bool) {
        self.dyslexia_font = enabled;
        self.save();
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
        let u = self.unit_system();
        let unit = u.economy_unit();
        match self.average_mpg() {
            Some(mpg) => format!("{} avg", format_economy(mpg, u)),
            None => {
                if self.selected_vehicle_id.is_none() {
                    format!("{unit}: select a vehicle")
                } else {
                    format!("{unit}: need 2+ fuel fill-ups")
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
        let u = self.unit_system();
        let mpg = self
            .average_mpg()
            .map(|m| format!("{} avg", format_economy(m, u)))
            .unwrap_or_else(|| format!("{}: —", u.economy_unit()));
        let costs = self.cost_labels();
        let month = format!("This month: {}", costs[0].1);
        let year = format!("This year: {}", costs[1].1);
        let dues = {
            let s = self.component_alerts_summary();
            if s == "All clear." {
                "Reminders: all clear".into()
            } else {
                s
            }
        };
        (mpg, month, year, dues)
    }

    /// Fuel fill-up history with per-leg economy where possible.
    pub fn fuel_history_lines(&self) -> Vec<(String, String)> {
        let Some(vid) = self.selected_vehicle_id else {
            return Vec::new();
        };
        let u = self.unit_system();
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
            let mut detail = format!(
                "{} · {}",
                format_distance(s.mileage, u),
                format_fuel(gal, u)
            );
            if let Some(fc) = s.fuel_cost {
                detail.push_str(&format!(" · ${fc:.2}"));
            }
            if i > 0 {
                let prev = fills[i - 1];
                let miles = s.mileage.saturating_sub(prev.mileage);
                if miles > 0 && gal > 0.0 {
                    let leg = f64::from(miles) / gal;
                    detail.push_str(&format!(" · {} this fill", format_economy(leg, u)));
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
                    || s.notes.to_ascii_lowercase().contains(&q)
                    || s.source.as_str().to_ascii_lowercase().contains(&q)
                    || s.mileage.to_string().contains(&q)
            });
        }
        list.sort_by(|a, b| b.date_epoch_ms.cmp(&a.date_epoch_ms).then(b.id.cmp(&a.id)));
        list
    }

    pub fn cost_labels(&self) -> [(String, String); 5] {
        let now = Utc::now().timestamp_millis();
        let selected: Vec<ServiceRecord> = self
            .services_for_selected()
            .into_iter()
            .cloned()
            .collect();
        let s = summarize_costs(&selected, now);
        let (tire_m, tire_y, tire_all) = self.tire_cost_totals(now);
        // Spec: operational costs include all maintenance (services + tire purchases)
        [
            (
                "This month".into(),
                format!("${:.2}", s.month_total + tire_m),
            ),
            (
                "This year".into(),
                format!("${:.2}", s.year_total + tire_y),
            ),
            (
                "All time".into(),
                format!("${:.2}", s.all_time_total + tire_all),
            ),
            (
                "Services (all time)".into(),
                format!("${:.2}", s.all_time_total),
            ),
            ("Tires (all time)".into(), format!("${:.2}", tire_all)),
        ]
    }

    /// Tire purchase costs for selected vehicle, split by calendar month/year/all.
    fn tire_cost_totals(&self, now_epoch_ms: i64) -> (f64, f64, f64) {
        let Some(vid) = self.selected_vehicle_id else {
            return (0.0, 0.0, 0.0);
        };
        let now = match Utc.timestamp_millis_opt(now_epoch_ms) {
            chrono::LocalResult::Single(dt) => dt,
            _ => return (0.0, 0.0, 0.0),
        };
        let y = now.year();
        let m = now.month();
        let mut month = 0.0;
        let mut year = 0.0;
        let mut all = 0.0;
        for t in self.tire_purchases.iter().filter(|t| t.vehicle_id == vid) {
            all += t.cost;
            if let chrono::LocalResult::Single(dt) = Utc.timestamp_millis_opt(t.date_epoch_ms) {
                if dt.year() == y {
                    year += t.cost;
                    if dt.month() == m {
                        month += t.cost;
                    }
                }
            }
        }
        (month, year, all)
    }

    pub fn tire_preview(&self) -> String {
        let cfg = self.selected_tire_config();
        let after = self.preview_after_layout();
        let pattern =
            RotationPattern::from_str(&cfg.pattern).unwrap_or(RotationPattern::ForwardCross);
        let mut s = format!(
            "After {}: {} {} / {} {}",
            pattern.label(),
            after.fl,
            after.fr,
            after.rl,
            after.rr
        );
        if cfg.include_spare {
            s.push_str(&format!(" · SP {}", after.spare));
            s.push_str(" (incl. spare)");
        }
        s
    }

    /// Positions after applying the currently selected pattern (preview only).
    pub fn preview_after_layout(&self) -> TireLayout {
        let cfg = self.selected_tire_config();
        let layout = TireLayout::from(&cfg.layout);
        let pattern =
            RotationPattern::from_str(&cfg.pattern).unwrap_or(RotationPattern::ForwardCross);
        apply_rotation(&layout, pattern, cfg.include_spare)
    }

    pub fn set_tire_pattern(&mut self, pattern: String) {
        if let Some(cfg) = self.tire_config_mut() {
            cfg.pattern = pattern.clone();
        }
        self.tire_pattern = pattern; // legacy mirror
        self.save();
    }

    pub fn set_include_spare(&mut self, include: bool) {
        if let Some(cfg) = self.tire_config_mut() {
            cfg.include_spare = include;
        }
        // Keep legacy field in sync for older code paths
        self.include_spare_in_rotation = include;
        self.save();
    }

    pub fn set_spare_label(&mut self, label: String) {
        let t = label.trim();
        let spare = if t.is_empty() {
            "E".into()
        } else {
            t.to_string()
        };
        if let Some(cfg) = self.tire_config_mut() {
            cfg.layout.spare = spare;
        }
        self.save();
    }

    pub fn apply_tire_rotation(&mut self) {
        let cfg = self.selected_tire_config();
        let before = cfg.layout.clone();
        let layout = TireLayout::from(&cfg.layout);
        let pattern =
            RotationPattern::from_str(&cfg.pattern).unwrap_or(RotationPattern::ForwardCross);
        let include = cfg.include_spare;
        let after = apply_rotation(&layout, pattern, include);
        let after_stored = TireLayoutStored::from(&after);
        if let Some(vid) = self.selected_vehicle_id {
            let id = self.next_rotation_id;
            self.next_rotation_id += 1;
            let mileage = self.selected_vehicle().map(|v| v.current_mileage);
            let mut pattern_label = pattern.label().to_string();
            if include {
                pattern_label.push_str(" + spare");
            }
            self.tire_rotations.push(TireRotationLog {
                id,
                vehicle_id: vid,
                pattern: pattern_label,
                before_fl: before.fl,
                before_fr: before.fr,
                before_rl: before.rl,
                before_rr: before.rr,
                before_spare: before.spare.clone(),
                after_fl: after_stored.fl.clone(),
                after_fr: after_stored.fr.clone(),
                after_rl: after_stored.rl.clone(),
                after_rr: after_stored.rr.clone(),
                after_spare: after_stored.spare.clone(),
                included_spare: include,
                mileage,
                date_epoch_ms: Utc::now().timestamp_millis(),
            });
            // Spec: mileage per tire + tread follow the rubber through rotation
            self.remap_corner_data_for_vehicle(vid, pattern, include);
            if let Some(c) = self.tire_config_mut() {
                c.layout = after_stored;
            }
        }
    }

    fn remap_corner_data_for_vehicle(
        &mut self,
        vehicle_id: u64,
        pattern: RotationPattern,
        include_spare: bool,
    ) {
        if let Some(slot) = self
            .tire_mileage_by_vehicle
            .iter_mut()
            .find(|t| t.vehicle_id == vehicle_id)
        {
            if include_spare {
                let (fl, fr, rl, rr, spare) = map_corners5(
                    &slot.miles.fl,
                    &slot.miles.fr,
                    &slot.miles.rl,
                    &slot.miles.rr,
                    &slot.miles.spare,
                    pattern,
                );
                slot.miles = TireCornerMiles {
                    fl,
                    fr,
                    rl,
                    rr,
                    spare,
                };
            } else {
                let (fl, fr, rl, rr) = map_corners(
                    &slot.miles.fl,
                    &slot.miles.fr,
                    &slot.miles.rl,
                    &slot.miles.rr,
                    pattern,
                );
                slot.miles = TireCornerMiles {
                    fl,
                    fr,
                    rl,
                    rr,
                    spare: slot.miles.spare,
                };
            }
        }
        if let Some(slot) = self
            .tread_by_vehicle
            .iter_mut()
            .find(|t| t.vehicle_id == vehicle_id)
        {
            if include_spare {
                let (fl, fr, rl, rr, spare) = map_corners5(
                    &slot.depths.fl,
                    &slot.depths.fr,
                    &slot.depths.rl,
                    &slot.depths.rr,
                    &slot.depths.spare,
                    pattern,
                );
                slot.depths.fl = fl;
                slot.depths.fr = fr;
                slot.depths.rl = rl;
                slot.depths.rr = rr;
                slot.depths.spare = spare;
            } else {
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
    }

    pub fn export_csv(&self) -> String {
        self.export_all_csv()
    }

    /// Multi-section CSV for backup/share (services + vehicles + trackers).
    pub fn export_all_csv(&self) -> String {
        let mut out = String::new();
        out.push_str("### vehicles\n");
        out.push_str("id,name,make,model,year,mileage,vin\n");
        for v in &self.vehicles {
            out.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                v.id,
                csv_esc(&v.name),
                csv_esc(&v.make),
                csv_esc(&v.model),
                v.year.map(|y| y.to_string()).unwrap_or_default(),
                v.current_mileage,
                csv_esc(&v.vin),
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
        vin: Option<String>,
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
            if let Some(vin) = vin {
                v.vin = vin
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| c.to_ascii_uppercase())
                    .collect();
            }
            self.save();
        }
    }

    /// Apply VIN + optional decoded make/model/year onto selected vehicle.
    pub fn apply_vin_decode_to_selected(
        &mut self,
        vin: String,
        make: Option<String>,
        model: Option<String>,
        year: Option<u16>,
    ) {
        let Some(id) = self.selected_vehicle_id else {
            return;
        };
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == id) {
            v.vin = vin;
            if let Some(m) = make {
                if !m.is_empty() && (v.make.is_empty() || v.make.eq_ignore_ascii_case("unknown")) {
                    v.make = m;
                } else if v.make.is_empty() {
                    v.make = m;
                }
            }
            if let Some(m) = model {
                if !m.is_empty() && v.model.is_empty() {
                    v.model = m;
                }
            }
            if year.is_some() && v.year.is_none() {
                v.year = year;
            }
            self.save();
        }
    }

    pub fn set_cloud_settings(&mut self, url: String, user: String, pass: String) -> Result<(), String> {
        let url = url.trim().to_string();
        if !url.is_empty() {
            let lower = url.to_ascii_lowercase();
            if !lower.starts_with("https://") {
                return Err(
                    "WebDAV URL must start with https:// (cleartext HTTP is blocked).".into(),
                );
            }
        }
        self.cloud_webdav_url = url;
        self.cloud_username = user.trim().into();
        // Empty password field = keep existing (so URL/user can be edited without re-typing).
        if !pass.is_empty() {
            self.cloud_password = pass;
        }
        self.save();
        Ok(())
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
        self.tire_config_by_vehicle.retain(|t| t.vehicle_id != id);
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

    pub fn set_tread_depths(
        &mut self,
        fl: Option<f64>,
        fr: Option<f64>,
        rl: Option<f64>,
        rr: Option<f64>,
        spare: Option<f64>,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        let depths = TreadDepths {
            fl,
            fr,
            rl,
            rr,
            spare,
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
        let u = self.unit_system();
        let t = self.tread_for_selected();
        let fmt = |v: Option<f64>| {
            v.map(|x| format_tread(x, u))
                .unwrap_or_else(|| "—".into())
        };
        let when = t
            .measured_epoch_ms
            .map(format_epoch)
            .unwrap_or_else(|| "never".into());
        let mut s = format!(
            "FL {} · FR {} · RL {} · RR {}",
            fmt(t.fl),
            fmt(t.fr),
            fmt(t.rl),
            fmt(t.rr),
        );
        if self.selected_tire_config().include_spare || t.spare.is_some() {
            s.push_str(&format!(" · SP {}", fmt(t.spare)));
        }
        s.push_str(&format!(" (measured {when})"));
        s
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

    /// Due reminders across **all** vehicles (for launch notifications).
    /// Returns (notif_id, title, body).
    pub fn all_due_notification_items(&self) -> Vec<(i32, String, String)> {
        let now = Utc::now().timestamp_millis();
        let mut out = Vec::new();
        for r in self.reminders.iter().filter(|r| !r.completed) {
            let odo = self
                .vehicles
                .iter()
                .find(|v| v.id == r.vehicle_id)
                .map(|v| v.current_mileage)
                .unwrap_or(0);
            let vname = self
                .vehicles
                .iter()
                .find(|v| v.id == r.vehicle_id)
                .map(|v| v.name.as_str())
                .unwrap_or("Vehicle");
            if is_due_by_date(r.due_epoch_ms, now) || is_due_by_mileage(r.due_mileage, odo) {
                let detail = reminder_status_line_units(r, odo, self.unit_system());
                out.push((
                    43000 + (r.id as i32 % 5000),
                    format!("Due: {}", r.title),
                    format!("{vname} · {detail}"),
                ));
            }
        }
        out
    }

    /// Open date-based reminders still in the future (for AlarmManager scheduling).
    /// (request_code, due_epoch_ms, label)
    pub fn future_date_alarms(&self) -> Vec<(i32, i64, String)> {
        let now = Utc::now().timestamp_millis();
        self.reminders
            .iter()
            .filter(|r| !r.completed)
            .filter_map(|r| {
                let due = r.due_epoch_ms?;
                if due > now {
                    Some((
                        50000 + (r.id as i32 % 10000),
                        due,
                        r.title.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Upcoming open reminders for selected vehicle (next 90 days or within 5k mi).
    pub fn upcoming_reminders_lines(&self) -> Vec<(String, String)> {
        let Some(vid) = self.selected_vehicle_id else {
            return Vec::new();
        };
        let now = Utc::now().timestamp_millis();
        let horizon = now + 90 * 86_400_000;
        let odo = self.selected_vehicle().map(|v| v.current_mileage).unwrap_or(0);
        let mut rows: Vec<_> = self
            .reminders
            .iter()
            .filter(|r| r.vehicle_id == vid && !r.completed)
            .filter(|r| {
                let date_soon = r
                    .due_epoch_ms
                    .map(|d| d <= horizon)
                    .unwrap_or(false);
                let mi_soon = r
                    .due_mileage
                    .map(|m| m <= odo.saturating_add(5_000))
                    .unwrap_or(false);
                date_soon || mi_soon || is_due_by_date(r.due_epoch_ms, now) || is_due_by_mileage(r.due_mileage, odo)
            })
            .map(|r| {
                (
                    r.title.clone(),
                    reminder_status_line_units(r, odo, self.unit_system()),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows.truncate(12);
        rows
    }

    /// True if we should fire notifications (not more than once per 12 hours unless forced).
    pub fn should_notify_now(&self) -> bool {
        let now = Utc::now().timestamp_millis();
        now.saturating_sub(self.last_notified_epoch_ms) >= 12 * 3_600_000
            || self.last_notified_epoch_ms == 0
    }

    pub fn mark_notified(&mut self) {
        self.last_notified_epoch_ms = Utc::now().timestamp_millis();
        self.save();
    }

    /// Combined home alert text for any vehicle-level component issues.
    pub fn component_alerts_summary(&self) -> String {
        let mut bits = Vec::new();
        if self.has_due_reminders() {
            bits.push(self.due_reminders_summary());
        }
        if self.has_brakes_due() {
            bits.push(self.brake_due_warning());
        }
        if self.has_old_battery() {
            bits.push(self.battery_age_warning());
        }
        if self.has_old_wipers() {
            bits.push(self.wiper_due_warning());
        }
        if self.has_low_tread() {
            bits.push(self.tread_warning());
        }
        if bits.is_empty() {
            "All clear.".into()
        } else {
            bits.join(" · ")
        }
    }

    /// US/EU common legal minimum for passenger tires (~2/32" ≈ 1.6 mm).
    pub const TREAD_MIN_MM: f64 = 1.6;

    pub fn tread_warning(&self) -> String {
        let u = self.unit_system();
        let t = self.tread_for_selected();
        let mut low = Vec::new();
        let mut check = |label: &str, v: Option<f64>| {
            if let Some(mm) = v {
                if mm < Self::TREAD_MIN_MM {
                    low.push(format!("{label} {}", format_tread(mm, u)));
                }
            }
        };
        check("FL", t.fl);
        check("FR", t.fr);
        check("RL", t.rl);
        check("RR", t.rr);
        if low.is_empty() {
            if t.measured_epoch_ms.is_none() {
                format!("No tread measurement yet. {}", tread_limit_label(u))
            } else {
                format!(
                    "All measured corners are at or above the legal limit ({}).",
                    format_tread(Self::TREAD_MIN_MM, u)
                )
            }
        } else {
            format!(
                "⚠ Below {} (replace soon): {}",
                format_tread(Self::TREAD_MIN_MM, u),
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

    /// Write app state JSON backup for share/export.
    /// **Security:** WebDAV password is always stripped from the file.
    pub fn write_backup_file(&self) -> Result<PathBuf, String> {
        let dir = Self::data_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&dir);
        let stamp = Utc::now().format("%Y%m%d-%H%M%S");
        let path = dir.join(format!("fixitgarage-backup-{stamp}.json"));
        let mut export = self.clone();
        export.cloud_password.clear();
        let json = serde_json::to_vec_pretty(&export).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Restore state from a backup JSON file path.
    /// **Security:** any password present in the file is discarded (re-enter WebDAV password).
    pub fn restore_from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path.trim()).map_err(|e| format!("read: {e}"))?;
        let mut state: AppState =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
        state.cloud_password.clear();
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
        vin: String,
    ) {
        if name.trim().is_empty() {
            return;
        }
        let id = self.next_vehicle_id;
        self.next_vehicle_id += 1;
        let vin: String = vin
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect();
        self.vehicles.push(Vehicle {
            id,
            name: name.trim().into(),
            make,
            model,
            year,
            current_mileage: mileage,
            vin,
        });
        self.selected_vehicle_id = Some(id);
        self.ensure_tire_config(id);
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
        notes: String,
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
            notes: notes.trim().into(),
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
        notes: String,
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
            notes,
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
        let ptype = part_type.clone();
        let installed = Utc::now().timestamp_millis();
        if let Some(existing) = self
            .parts
            .iter_mut()
            .find(|p| p.vehicle_id == vid && p.part_type == part_type)
        {
            existing.brand = brand;
            existing.part_number = part_number;
            existing.oil_viscosity = oil_viscosity;
            existing.notes = notes;
            existing.installed_epoch_ms = Some(installed);
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
                installed_epoch_ms: Some(installed),
                installed_mileage: mileage,
            });
        }
        // Spec: smart reminders for filter changes by mileage + date
        self.schedule_part_reminder(vid, &ptype, Some(installed), mileage);
        self.save();
    }

    fn schedule_part_reminder(
        &mut self,
        vehicle_id: u64,
        part_type: &str,
        installed_epoch_ms: Option<i64>,
        installed_mileage: Option<u32>,
    ) {
        let (title, due_date, due_mi): (String, Option<i64>, Option<u32>) = match part_type {
            "ENGINE_AIR_FILTER" => (
                "Replace engine air filter".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 12)),
                installed_mileage.map(|m| m.saturating_add(15_000)),
            ),
            "CABIN_FILTER" => (
                "Replace cabin air filter".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 12)),
                installed_mileage.map(|m| m.saturating_add(15_000)),
            ),
            "OIL_FILTER" => (
                "Replace oil filter".into(),
                installed_epoch_ms.map(|ms| add_months_approx(ms, 6)),
                installed_mileage.map(|m| m.saturating_add(5_000)),
            ),
            // Oil type is reference data, not a timed replacement
            _ => return,
        };
        self.reminders.retain(|r| {
            !(r.vehicle_id == vehicle_id && !r.completed && r.title == title)
        });
        self.add_reminder_raw(vehicle_id, title, due_date, due_mi);
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

    /// Camera / coin gauge assist for tread measurement.
    pub fn tread_coin_guide(&self) -> String {
        units_tread_coin_guide(self.unit_system())
    }

    pub fn set_tire_corner_miles(
        &mut self,
        fl: Option<u32>,
        fr: Option<u32>,
        rl: Option<u32>,
        rr: Option<u32>,
        spare: Option<u32>,
    ) {
        let Some(vid) = self.selected_vehicle_id else {
            return;
        };
        let miles = TireCornerMiles {
            fl,
            fr,
            rl,
            rr,
            spare,
        };
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
        let u = self.unit_system();
        let m = self.tire_miles_for_selected();
        let f = |v: Option<u32>| {
            v.map(|x| format_distance(x, u))
                .unwrap_or_else(|| "—".into())
        };
        let mut s = format!(
            "FL {} · FR {} · RL {} · RR {}",
            f(m.fl),
            f(m.fr),
            f(m.rl),
            f(m.rr)
        );
        if self.selected_tire_config().include_spare || m.spare.is_some() {
            s.push_str(&format!(" · SP {}", f(m.spare)));
        }
        s
    }

    pub fn delete_tire_purchase(&mut self, id: u64) {
        self.tire_purchases.retain(|p| p.id != id);
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
                let u = self.unit_system();
                let mi = l
                    .mileage
                    .map(|m| format!(" · {}", format_distance(m, u)))
                    .unwrap_or_default();
                let level = display_oil_level(&l.level, u);
                format!("Last check: {}{} — {}", date, mi, level)
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
                    .map(|m| format_distance(m, self.unit_system()))
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

pub fn reminder_status_line_units(
    r: &ReminderEntry,
    current_mileage: u32,
    units: UnitSystem,
) -> String {
    let now = Utc::now().timestamp_millis();
    let mut bits = Vec::new();
    if let Some(ms) = r.due_epoch_ms {
        bits.push(format!("due {}", format_epoch(ms)));
        if is_due_by_date(Some(ms), now) {
            bits.push("OVERDUE".into());
        }
    }
    if let Some(m) = r.due_mileage {
        bits.push(format!("at {}", format_distance(m, units)));
        if is_due_by_mileage(Some(m), current_mileage) {
            bits.push("DUE BY ODOMETER".into());
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
        s.add_vehicle("A".into(), "".into(), "".into(), None, 1000, String::new());
        let a = s.selected_vehicle_id.unwrap();
        s.add_vehicle("B".into(), "".into(), "".into(), None, 2000, String::new());
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
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 1000, String::new());
        s.set_tire_pattern("side_to_side".into());
        s.apply_tire_rotation();
        let lay = s.selected_tire_config().layout;
        assert_eq!(lay.fl, "B");
        assert_eq!(lay.fr, "A");
    }

    #[test]
    fn rotation_moves_mileage_with_tires() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 10000, String::new());
        s.set_tire_corner_miles(Some(1), Some(2), Some(3), Some(4), Some(5));
        s.set_tire_pattern("side_to_side".into());
        s.apply_tire_rotation();
        let m = s.tire_miles_for_selected();
        assert_eq!(m.fl, Some(2));
        assert_eq!(m.fr, Some(1));
        assert_eq!(m.rl, Some(4));
        assert_eq!(m.rr, Some(3));
        assert_eq!(m.spare, Some(5)); // spare not rotated when option off
    }

    #[test]
    fn rotation_with_spare_moves_five() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 10000, String::new());
        s.set_include_spare(true);
        s.set_tire_corner_miles(Some(1), Some(2), Some(3), Some(4), Some(5));
        s.set_tire_pattern("forward_cross".into());
        s.apply_tire_rotation();
        // FL←RL(3), FR←RR(4), RL←FR(2), RR←SP(5), SP←FL(1)
        let m = s.tire_miles_for_selected();
        assert_eq!(m.fl, Some(3));
        assert_eq!(m.fr, Some(4));
        assert_eq!(m.rl, Some(2));
        assert_eq!(m.rr, Some(5));
        assert_eq!(m.spare, Some(1));
        assert_eq!(s.selected_tire_config().layout.spare, "A");
    }

    #[test]
    fn tire_layout_is_per_vehicle() {
        let mut s = AppState::default();
        s.add_vehicle("Car A".into(), "".into(), "".into(), None, 1000, String::new());
        let a = s.selected_vehicle_id.unwrap();
        s.set_tire_pattern("side_to_side".into());
        s.apply_tire_rotation();
        s.add_vehicle("Car B".into(), "".into(), "".into(), None, 2000, String::new());
        let b = s.selected_vehicle_id.unwrap();
        // B should still have default A B C D
        assert_eq!(s.selected_tire_config().layout.fl, "A");
        s.select_vehicle(a);
        assert_eq!(s.selected_tire_config().layout.fl, "B"); // rotated
        s.select_vehicle(b);
        assert_eq!(s.selected_tire_config().layout.fl, "A");
    }

    #[test]
    fn all_due_notifications_and_upcoming() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 50000, String::new());
        s.add_reminder("Inspect belts".into(), "2000-01-01", None);
        assert!(!s.all_due_notification_items().is_empty());
        assert!(s.should_notify_now());
        s.mark_notified();
        assert!(!s.should_notify_now());
        // Upcoming includes due/soon items
        assert!(!s.upcoming_reminders_lines().is_empty());
    }

    #[test]
    fn part_save_schedules_filter_reminder() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 40000, String::new());
        s.upsert_part(
            "ENGINE_AIR_FILTER".into(),
            "Fram".into(),
            "CA123".into(),
            "".into(),
            "".into(),
            Some(40000),
        );
        assert!(s
            .reminders
            .iter()
            .any(|r| !r.completed && r.title.contains("air filter")));
    }

    #[test]
    fn costs_include_tire_purchases() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 1000, String::new());
        s.add_service("Oil".into(), 1000, "DIY", 40.0, None);
        s.add_tire_purchase(
            "Michelin".into(),
            "X".into(),
            "225/65R17".into(),
            600.0,
            Some(1000),
            "".into(),
        );
        let labels = s.cost_labels();
        // All time should be services 40 + tires 600
        assert!(labels[2].1.contains("640"));
    }

    #[test]
    fn shop_mode_hides_tires() {
        let mut s = AppState::default();
        s.user_mode = "SHOP".into();
        assert!(!s.feature_flags().show_tires);
        assert!(!s.feature_flags().show_parts);
    }

    #[test]
    fn backup_strips_webdav_password() {
        let mut s = AppState::default();
        s.cloud_webdav_url = "https://cloud.example/dav/".into();
        s.cloud_username = "user".into();
        s.cloud_password = "super-secret".into();
        let path = s.write_backup_file().expect("backup");
        let raw = std::fs::read_to_string(&path).expect("read");
        assert!(!raw.contains("super-secret"), "password leaked into backup");
        assert!(raw.contains("cloud.example") || raw.contains("https://"));
        // In-memory password still present for local upload
        assert_eq!(s.cloud_password, "super-secret");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn webdav_settings_require_https_and_keep_password() {
        let mut s = AppState::default();
        assert!(s
            .set_cloud_settings("http://insecure.example/".into(), "u".into(), "p".into())
            .is_err());
        s.set_cloud_settings("https://secure.example/dav/".into(), "u".into(), "p1".into())
            .unwrap();
        assert_eq!(s.cloud_password, "p1");
        // blank password keeps previous
        s.set_cloud_settings("https://secure.example/dav/".into(), "u2".into(), "".into())
            .unwrap();
        assert_eq!(s.cloud_username, "u2");
        assert_eq!(s.cloud_password, "p1");
    }

    #[test]
    fn oil_level_logged_on_complete() {
        let mut s = AppState::default();
        s.add_vehicle("Daily".into(), "".into(), "".into(), None, 50000, String::new());
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
