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

/// Mastered staleness curve: targets long-interval re-exposure at ~14 / 30 / 60 days,
/// per the overlearning literature. Below 14 days the score is intentionally low so
/// mastered segments don't crowd out struggling ones; from 14 days onward the score
/// ramps up so a mastered segment that hasn't been touched for two months is
/// genuinely competitive in the queue.
pub fn staleness_factor_mastered(days_since: f64) -> f64 {
    let d = days_since.max(0.0);
    if d < 14.0 {
        0.2
    } else if d < 30.0 {
        0.6
    } else if d < 60.0 {
        1.2
    } else {
        2.0
    }
}

/// Boosts segments with little total practice time so the scheduler doesn't keep
/// cycling the same five segments. Decays to 1.0 (no boost) once meaningful time
/// has been spent. Time bands chosen so a brand-new segment with zero history
/// gets a 1.5x boost; after ~30 cumulative minutes the boost has fully decayed.
pub fn under_invested_factor(total_seconds_practiced: u64) -> f64 {
    let minutes = total_seconds_practiced as f64 / 60.0;
    if minutes < 5.0 {
        1.5
    } else if minutes < 15.0 {
        1.2
    } else if minutes < 30.0 {
        1.1
    } else {
        1.0
    }
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

    #[test]
    fn mastered_staleness_is_quiet_in_first_two_weeks() {
        assert_eq!(staleness_factor_mastered(0.0), 0.2);
        assert_eq!(staleness_factor_mastered(13.9), 0.2);
    }

    #[test]
    fn mastered_staleness_ramps_at_target_intervals() {
        assert_eq!(staleness_factor_mastered(14.0), 0.6);
        assert_eq!(staleness_factor_mastered(30.0), 1.2);
        assert_eq!(staleness_factor_mastered(60.0), 2.0);
        assert_eq!(staleness_factor_mastered(365.0), 2.0); // capped at 2.0
    }

    #[test]
    fn under_invested_boost_decays_with_time() {
        assert_eq!(under_invested_factor(0), 1.5);
        assert_eq!(under_invested_factor(60 * 4), 1.5);
        assert_eq!(under_invested_factor(60 * 10), 1.2);
        assert_eq!(under_invested_factor(60 * 20), 1.1);
        assert_eq!(under_invested_factor(60 * 60), 1.0);
    }
}
