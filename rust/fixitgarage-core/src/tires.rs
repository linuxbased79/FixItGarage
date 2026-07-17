//! Tire layout and rotation patterns (top-down: FL FR / RL RR).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::FigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RotationPattern {
    ForwardCross,
    RearwardCross,
    XPattern,
    SideToSide,
}

impl RotationPattern {
    pub fn label(self) -> &'static str {
        match self {
            Self::ForwardCross => "Forward cross",
            Self::RearwardCross => "Rearward cross",
            Self::XPattern => "X pattern",
            Self::SideToSide => "Side to side",
        }
    }
}

impl FromStr for RotationPattern {
    type Err = FigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "forward_cross" | "forward" | "forwardcross" => Ok(Self::ForwardCross),
            "rearward_cross" | "rearward" | "rearwardcross" => Ok(Self::RearwardCross),
            "x_pattern" | "x" | "xpattern" => Ok(Self::XPattern),
            "side_to_side" | "side" | "sidetoside" => Ok(Self::SideToSide),
            other => Err(FigError::InvalidInput(format!("unknown rotation pattern: {other}"))),
        }
    }
}

/// Labels at each corner (often tire IDs like "A","B","C","D").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TireLayout {
    pub fl: String,
    pub fr: String,
    pub rl: String,
    pub rr: String,
}

impl Default for TireLayout {
    fn default() -> Self {
        Self {
            fl: "FL".into(),
            fr: "FR".into(),
            rl: "RL".into(),
            rr: "RR".into(),
        }
    }
}

impl fmt::Display for TireLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  {}   {}\n    [CAR]\n  {}   {}",
            self.fl, self.fr, self.rl, self.rr
        )
    }
}

/// Apply a rotation pattern and return the new layout (does not mutate input).
pub fn apply_rotation(current: &TireLayout, pattern: RotationPattern) -> TireLayout {
    let (fl, fr, rl, rr) = map_corners(
        &current.fl,
        &current.fr,
        &current.rl,
        &current.rr,
        pattern,
    );
    TireLayout { fl, fr, rl, rr }
}

/// Remap any per-corner values the same way tire positions move.
/// Used for labels, tread depths, and mileage-per-tire so data follows the rubber.
pub fn map_corners<T: Clone>(
    fl: &T,
    fr: &T,
    rl: &T,
    rr: &T,
    pattern: RotationPattern,
) -> (T, T, T, T) {
    match pattern {
        RotationPattern::ForwardCross => (rl.clone(), rr.clone(), fr.clone(), fl.clone()),
        RotationPattern::RearwardCross => (rr.clone(), rl.clone(), fl.clone(), fr.clone()),
        RotationPattern::XPattern => (rr.clone(), rl.clone(), fr.clone(), fl.clone()),
        RotationPattern::SideToSide => (fr.clone(), fl.clone(), rr.clone(), rl.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_cross_moves_rears_forward() {
        let before = TireLayout {
            fl: "A".into(),
            fr: "B".into(),
            rl: "C".into(),
            rr: "D".into(),
        };
        let after = apply_rotation(&before, RotationPattern::ForwardCross);
        assert_eq!(after.fl, "C");
        assert_eq!(after.fr, "D");
        assert_eq!(after.rl, "B");
        assert_eq!(after.rr, "A");
    }

    #[test]
    fn side_to_side_swaps_left_right() {
        let before = TireLayout {
            fl: "A".into(),
            fr: "B".into(),
            rl: "C".into(),
            rr: "D".into(),
        };
        let after = apply_rotation(&before, RotationPattern::SideToSide);
        assert_eq!(after.fl, "B");
        assert_eq!(after.fr, "A");
        assert_eq!(after.rl, "D");
        assert_eq!(after.rr, "C");
    }

    #[test]
    fn map_corners_matches_layout() {
        let (fl, fr, rl, rr) = map_corners(&10u32, &20, &30, &40, RotationPattern::ForwardCross);
        assert_eq!((fl, fr, rl, rr), (30, 40, 20, 10));
    }
}
