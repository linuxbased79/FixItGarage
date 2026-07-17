//! Smart reminders: date + mileage, oil-level every 3 months.

use chrono::{DateTime, Datelike, Duration, Timelike, TimeZone, Utc};

/// Product requirement: oil level check reminders every 3 months.
pub const OIL_LEVEL_INTERVAL_MONTHS: i32 = 3;

pub fn is_due_by_date(due_epoch_ms: Option<i64>, now_epoch_ms: i64) -> bool {
    match due_epoch_ms {
        Some(due) => due <= now_epoch_ms,
        None => false,
    }
}

pub fn is_due_by_mileage(due_mileage: Option<u32>, current_mileage: u32) -> bool {
    match due_mileage {
        Some(due) => current_mileage >= due,
        None => false,
    }
}

/// Approximate add-months for oil-level scheduling (calendar months).
pub fn oil_level_due_after(from_epoch_ms: i64) -> i64 {
    add_months(from_epoch_ms, OIL_LEVEL_INTERVAL_MONTHS)
}

fn add_months(epoch_ms: i64, months: i32) -> i64 {
    let dt = match Utc.timestamp_millis_opt(epoch_ms) {
        chrono::LocalResult::Single(d) => d,
        _ => return epoch_ms,
    };
    let (y, m, d) = (dt.year(), dt.month() as i32, dt.day());
    let total_m = y * 12 + (m - 1) + months;
    let ny = total_m.div_euclid(12);
    let nm = total_m.rem_euclid(12) + 1;
    // Clamp day for short months
    let last_day = last_day_of_month(ny, nm as u32);
    let day = d.min(last_day);
    Utc.with_ymd_and_hms(ny, nm as u32, day, dt.hour(), dt.minute(), dt.second())
        .single()
        .map(|d: DateTime<Utc>| d.timestamp_millis())
        .unwrap_or_else(|| (dt + Duration::days(30 * i64::from(months))).timestamp_millis())
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next = Utc.with_ymd_and_hms(ny, nm, 1, 0, 0, 0).unwrap();
    (first_next - Duration::days(1)).day()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_by_mileage() {
        assert!(is_due_by_mileage(Some(5000), 5000));
        assert!(!is_due_by_mileage(Some(5000), 4999));
        assert!(!is_due_by_mileage(None, 99999));
    }

    #[test]
    fn oil_interval_constant() {
        assert_eq!(OIL_LEVEL_INTERVAL_MONTHS, 3);
    }
}
