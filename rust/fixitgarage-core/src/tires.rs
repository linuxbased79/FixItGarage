//! Tire layout and rotation patterns (top-down: FL FR / RL RR, optional spare).

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

/// Labels at each corner (often tire IDs like "A","B","C","D") plus optional full-size spare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TireLayout {
    pub fl: String,
    pub fr: String,
    pub rl: String,
    pub rr: String,
    /// Full-size matching spare label (e.g. "E"). Empty if unused.
    #[serde(default)]
    pub spare: String,
}

impl Default for TireLayout {
    fn default() -> Self {
        Self {
            fl: "A".into(),
            fr: "B".into(),
            rl: "C".into(),
            rr: "D".into(),
            spare: "E".into(),
        }
    }
}

impl fmt::Display for TireLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.spare.trim().is_empty() {
            write!(
                f,
                "  {}   {}\n    [CAR]\n  {}   {}",
                self.fl, self.fr, self.rl, self.rr
            )
        } else {
            write!(
                f,
                "  {}   {}\n    [CAR]\n  {}   {}\n   SP:{}",
                self.fl, self.fr, self.rl, self.rr, self.spare
            )
        }
    }
}

/// Apply a rotation pattern and return the new layout (does not mutate input).
///
/// When `include_spare` is false (default for most drivers), the spare label stays put
/// and only the four corners move. When true, uses 5-tire patterns for a full-size spare.
pub fn apply_rotation(
    current: &TireLayout,
    pattern: RotationPattern,
    include_spare: bool,
) -> TireLayout {
    if include_spare && !current.spare.trim().is_empty() {
        let (fl, fr, rl, rr, spare) = map_corners5(
            &current.fl,
            &current.fr,
            &current.rl,
            &current.rr,
            &current.spare,
            pattern,
        );
        TireLayout {
            fl,
            fr,
            rl,
            rr,
            spare,
        }
    } else {
        let (fl, fr, rl, rr) = map_corners(
            &current.fl,
            &current.fr,
            &current.rl,
            &current.rr,
            pattern,
        );
        TireLayout {
            fl,
            fr,
            rl,
            rr,
            spare: current.spare.clone(),
        }
    }
}

/// Remap any per-corner values the same way tire positions move (4 corners).
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

/// Remap five positions when a full-size spare is included in the rotation.
///
/// Patterns (source → destination):
/// - **Forward cross**: rears to front; left front → spare; spare → right rear; right front → left rear.
/// - **Rearward cross**: inverse of forward.
/// - **X pattern**: diagonals swap; spare cycles with FR.
/// - **Side to side**: left↔right on both axles; spare cycles with RR.
pub fn map_corners5<T: Clone>(
    fl: &T,
    fr: &T,
    rl: &T,
    rr: &T,
    spare: &T,
    pattern: RotationPattern,
) -> (T, T, T, T, T) {
    match pattern {
        // FL←RL, FR←RR, RL←FR, RR←SP, SP←FL
        RotationPattern::ForwardCross => (
            rl.clone(),
            rr.clone(),
            fr.clone(),
            spare.clone(),
            fl.clone(),
        ),
        // Inverse of forward (FL←SP, FR←RL, RL←FL, RR←FR, SP←RR)
        RotationPattern::RearwardCross => (
            spare.clone(),
            rl.clone(),
            fl.clone(),
            fr.clone(),
            rr.clone(),
        ),
        // X + spare: FL↔RR, FR↔RL is 4-tire X; with spare: FL←RR, FR←SP, RL←FL, RR←FR, SP←RL
        RotationPattern::XPattern => (
            rr.clone(),
            spare.clone(),
            fl.clone(),
            fr.clone(),
            rl.clone(),
        ),
        // Side-to-side + spare: swap L/R, spare with RR
        // FL↔FR, RL↔RR would leave spare fixed; instead: FL↔FR, RL↔SP, RR↔RL... 
        // FL←FR, FR←FL, RL←RR, RR←SP, SP←RL
        RotationPattern::SideToSide => (
            fr.clone(),
            fl.clone(),
            rr.clone(),
            spare.clone(),
            rl.clone(),
        ),
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
            spare: "E".into(),
        };
        let after = apply_rotation(&before, RotationPattern::ForwardCross, false);
        assert_eq!(after.fl, "C");
        assert_eq!(after.fr, "D");
        assert_eq!(after.rl, "B");
        assert_eq!(after.rr, "A");
        assert_eq!(after.spare, "E"); // spare stays put
    }

    #[test]
    fn side_to_side_swaps_left_right() {
        let before = TireLayout {
            fl: "A".into(),
            fr: "B".into(),
            rl: "C".into(),
            rr: "D".into(),
            spare: "E".into(),
        };
        let after = apply_rotation(&before, RotationPattern::SideToSide, false);
        assert_eq!(after.fl, "B");
        assert_eq!(after.fr, "A");
        assert_eq!(after.rl, "D");
        assert_eq!(after.rr, "C");
        assert_eq!(after.spare, "E");
    }

    #[test]
    fn map_corners_matches_layout() {
        let (fl, fr, rl, rr) = map_corners(&10u32, &20, &30, &40, RotationPattern::ForwardCross);
        assert_eq!((fl, fr, rl, rr), (30, 40, 20, 10));
    }

    #[test]
    fn forward_with_spare_moves_five() {
        let before = TireLayout {
            fl: "A".into(),
            fr: "B".into(),
            rl: "C".into(),
            rr: "D".into(),
            spare: "E".into(),
        };
        let after = apply_rotation(&before, RotationPattern::ForwardCross, true);
        // FL←C, FR←D, RL←B, RR←E, SP←A
        assert_eq!(after.fl, "C");
        assert_eq!(after.fr, "D");
        assert_eq!(after.rl, "B");
        assert_eq!(after.rr, "E");
        assert_eq!(after.spare, "A");
    }

    #[test]
    fn spare_empty_skips_five_even_if_flag_set() {
        let before = TireLayout {
            fl: "A".into(),
            fr: "B".into(),
            rl: "C".into(),
            rr: "D".into(),
            spare: "".into(),
        };
        let after = apply_rotation(&before, RotationPattern::ForwardCross, true);
        assert_eq!(after.fl, "C");
        assert_eq!(after.spare, "");
    }
}
