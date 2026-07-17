//! Automatic MPG from consecutive full-tank fill-ups.

/// Compute average MPG from fill-ups as `(odometer_miles, gallons)`.
///
/// For each consecutive pair, uses `(miles_driven) / gallons_of_later_fill`.
/// Returns `None` if fewer than two valid segments exist.
pub fn average_mpg(fills: &[(u32, f64)]) -> Option<f64> {
    if fills.len() < 2 {
        return None;
    }
    let mut segments = Vec::new();
    for window in fills.windows(2) {
        let (m0, _) = window[0];
        let (m1, gallons) = window[1];
        if m1 > m0 && gallons > 0.0 {
            let miles = f64::from(m1 - m0);
            segments.push(miles / gallons);
        }
    }
    if segments.is_empty() {
        None
    } else {
        Some(segments.iter().sum::<f64>() / segments.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_two_fills() {
        assert!(average_mpg(&[(1000, 10.0)]).is_none());
    }

    #[test]
    fn computes_segment_average() {
        // 300/10 = 30, 280/10 = 28 → 29
        let mpg = average_mpg(&[(10000, 10.0), (10300, 10.0), (10580, 10.0)]).unwrap();
        assert!((mpg - 29.0).abs() < 0.01);
    }

    #[test]
    fn ignores_zero_gallons() {
        assert!(average_mpg(&[(1000, 10.0), (1300, 0.0)]).is_none());
    }
}
