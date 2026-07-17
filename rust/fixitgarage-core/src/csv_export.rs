//! CSV export for service records (matches Android column set).

use crate::models::ServiceRecord;
use chrono::{TimeZone, Utc};

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn format_date(epoch_ms: i64) -> String {
    match Utc.timestamp_millis_opt(epoch_ms) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => String::new(),
    }
}

/// Export service records to CSV text with header row.
pub fn services_to_csv(records: &[ServiceRecord]) -> String {
    let mut out = String::from(
        "id,vehicleId,date,mileage,title,source,laborCost,partsCost,gallons,fuelCost,shopName\n",
    );
    for r in records {
        let gallons = r
            .gallons
            .map(|g| g.to_string())
            .unwrap_or_default();
        let fuel = r
            .fuel_cost
            .map(|c| c.to_string())
            .unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            r.id,
            r.vehicle_id,
            format_date(r.date_epoch_ms),
            r.mileage,
            escape_csv(&r.title),
            r.source.as_str(),
            r.labor_cost,
            r.parts_cost,
            gallons,
            fuel,
            escape_csv(&r.shop_name),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ServiceSource;

    #[test]
    fn exports_header_and_row() {
        let csv = services_to_csv(&[ServiceRecord {
            id: 1,
            vehicle_id: 2,
            date_epoch_ms: 1_700_000_000_000,
            mileage: 50000,
            title: "Oil change".into(),
            source: ServiceSource::Diy,
            labor_cost: 0.0,
            parts_cost: 42.5,
            gallons: None,
            fuel_cost: None,
            shop_name: String::new(),
        }]);
        assert!(csv.starts_with("id,vehicleId,"));
        assert!(csv.contains("Oil change"));
        assert!(csv.contains("DIY"));
    }
}
