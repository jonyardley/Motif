//! Scheduler scoring function and session-picking logic.

use crate::model::Difficulty;

/// Wraps `Difficulty::weight` for symmetry with the other curve functions in this module.
pub fn difficulty_weight(d: Difficulty) -> f64 {
    d.weight()
}

/// Regular staleness curve: floors at 1.0 (so just-practiced segments still score),
/// grows with the square-root of days-since-last-practice, and caps at 10.0 so that
/// ancient segments don't permanently dominate the queue.
pub fn staleness_factor(days_since: f64) -> f64 {
    (1.0 + days_since.max(0.0).sqrt()).min(10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_weight_matches_spec() {
        assert_eq!(difficulty_weight(Difficulty::Struggling), 4.0);
        assert_eq!(difficulty_weight(Difficulty::Mastered), 1.0);
    }

    #[test]
    fn staleness_floors_at_one_for_today() {
        assert_eq!(staleness_factor(0.0), 1.0);
    }

    #[test]
    fn staleness_grows_with_days() {
        assert!(staleness_factor(1.0) > staleness_factor(0.0));
        assert!(staleness_factor(7.0) > staleness_factor(1.0));
    }

    #[test]
    fn staleness_caps_at_ten() {
        assert_eq!(staleness_factor(10_000.0), 10.0);
    }

    #[test]
    fn staleness_treats_negative_days_as_zero() {
        // Defensive — a clock-skew bug shouldn't make scores explode.
        assert_eq!(staleness_factor(-5.0), 1.0);
    }
}
