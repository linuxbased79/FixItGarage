//! Monthly / yearly operational cost rollups.

use crate::models::ServiceRecord;
use chrono::{Datelike, TimeZone, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct CostSummary {
    pub month_total: f64,
    pub year_total: f64,
    pub all_time_total: f64,
    pub month_count: usize,
    pub year_count: usize,
    pub all_time_count: usize,
}

/// Summarize costs for a given "now" instant (epoch ms), defaulting callers can use current time.
pub fn summarize_costs(records: &[ServiceRecord], now_epoch_ms: i64) -> CostSummary {
    let now = match Utc.timestamp_millis_opt(now_epoch_ms) {
        chrono::LocalResult::Single(dt) => dt,
        _ => Utc::now(),
    };
    let y = now.year();
    let m = now.month();

    let mut summary = CostSummary {
        month_total: 0.0,
        year_total: 0.0,
        all_time_total: 0.0,
        month_count: 0,
        year_count: 0,
        all_time_count: 0,
    };

    for r in records {
        let total = r.total_cost();
        summary.all_time_total += total;
        summary.all_time_count += 1;

        let Some(dt) = (match Utc.timestamp_millis_opt(r.date_epoch_ms) {
            chrono::LocalResult::Single(d) => Some(d),
            _ => None,
        }) else {
            continue;
        };

        if dt.year() == y {
            summary.year_total += total;
            summary.year_count += 1;
            if dt.month() == m {
                summary.month_total += total;
                summary.month_count += 1;
            }
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ServiceSource;

    #[test]
    fn rolls_up_totals() {
        let now = Utc
            .with_ymd_and_hms(2024, 6, 15, 12, 0, 0)
            .unwrap()
            .timestamp_millis();
        let rec = |ms: i64, cost: f64| ServiceRecord {
            id: 1,
            vehicle_id: 1,
            date_epoch_ms: ms,
            mileage: 1,
            title: "x".into(),
            source: ServiceSource::Shop,
            labor_cost: cost,
            parts_cost: 0.0,
            gallons: None,
            fuel_cost: None,
            shop_name: String::new(),
            notes: String::new(),
        };
        let june = Utc
            .with_ymd_and_hms(2024, 6, 1, 0, 0, 0)
            .unwrap()
            .timestamp_millis();
        let jan = Utc
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap()
            .timestamp_millis();
        let prev = Utc
            .with_ymd_and_hms(2023, 6, 1, 0, 0, 0)
            .unwrap()
            .timestamp_millis();

        let s = summarize_costs(&[rec(june, 10.0), rec(jan, 20.0), rec(prev, 5.0)], now);
        assert!((s.month_total - 10.0).abs() < 0.01);
        assert!((s.year_total - 30.0).abs() < 0.01);
        assert!((s.all_time_total - 35.0).abs() < 0.01);
    }
}
