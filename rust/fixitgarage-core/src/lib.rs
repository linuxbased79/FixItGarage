//! FixItGarage core domain logic (GPL-3.0).
//!
//! Shared pure-Rust library for MPG calculation, tire rotation layouts,
//! CSV export, cost rollups, and smart reminders. Intended for CLI use now
//! and optional UniFFI / JNI bridging into the Android app later.

pub mod cost;
pub mod csv_export;
pub mod error;
pub mod models;
pub mod mpg;
pub mod reminders;
pub mod tires;

pub use cost::{summarize_costs, CostSummary};
pub use csv_export::services_to_csv;
pub use error::FigError;
pub use models::{ServiceRecord, ServiceSource, UserMode, Vehicle};
pub use mpg::average_mpg;
pub use reminders::{
    is_due_by_date, is_due_by_mileage, oil_level_due_after, OIL_LEVEL_INTERVAL_MONTHS,
};
pub use tires::{apply_rotation, map_corners, RotationPattern, TireLayout};
