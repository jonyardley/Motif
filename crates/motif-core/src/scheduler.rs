//! Scheduler scoring function and session-picking logic.

use crate::model::Difficulty;
use crate::model::Segment;
use crate::model::{PieceId, SegmentId};
use chrono::{DateTime, Utc};

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

/// The single scoring function shared by the heatmap visualisation and the
/// guided-session picker. By construction what the user *sees* matches what
/// the app *picks*.
///
/// `score = difficulty_weight × staleness_factor × under_invested_factor`
///
/// For Mastered segments, the Mastered staleness curve is used so that they
/// stay in long-interval rotation rather than dominating or being dropped.
pub fn score(segment: &Segment, now: DateTime<Utc>) -> f64 {
    let dw = difficulty_weight(segment.difficulty);
    let days = days_since_last_practice(segment, now);
    let sf = match segment.difficulty {
        crate::model::Difficulty::Mastered => staleness_factor_mastered(days),
        _ => staleness_factor(days),
    };
    let uif = under_invested_factor(segment.total_seconds_practiced());
    dw * sf * uif
}

/// Days since the most recent practice attempt, or a large sentinel (10_000) if
/// the segment has never been practised — which guarantees a brand-new segment
/// has a high staleness factor and surfaces in the queue.
pub fn days_since_last_practice(segment: &Segment, now: DateTime<Utc>) -> f64 {
    match segment.last_practiced_at() {
        None => 10_000.0,
        Some(last) => {
            let secs = (now - last).num_seconds().max(0) as f64;
            secs / 86_400.0
        }
    }
}

/// Picks the next session's segments, applying the cross-piece interleaving rule:
/// avoid two consecutive segments from the same piece when an equally-or-near-equally
/// scored alternative from a different piece is available.
///
/// Algorithm: sort all segments by raw score descending, then greedily pick — at each
/// step, prefer the highest-scoring segment whose piece is different from the
/// previously-picked segment's piece. Falls back to the highest remaining if no
/// alternative exists.
pub fn pick_session(
    segments: &[Segment],
    n: usize,
    now: DateTime<Utc>,
) -> Vec<SegmentId> {
    let mut scored: Vec<(SegmentId, PieceId, f64)> = segments
        .iter()
        .map(|s| (s.id.clone(), s.piece_id.clone(), score(s, now)))
        .collect();
    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut picked: Vec<(SegmentId, PieceId)> = Vec::with_capacity(n);
    while picked.len() < n && !scored.is_empty() {
        let last_piece = picked.last().map(|(_, p)| p.clone());
        let idx = match &last_piece {
            None => 0,
            Some(p) => scored
                .iter()
                .position(|(_, pid, _)| pid != p)
                .unwrap_or(0),
        };
        let (sid, pid, _) = scored.remove(idx);
        picked.push((sid, pid));
    }

    picked.into_iter().map(|(s, _)| s).collect()
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

    use crate::model::{
        AttemptId, MemorisationState, PieceId, PracticeAttempt, Segment, SegmentId,
    };
    use chrono::TimeZone;

    fn seg_with_history(
        difficulty: Difficulty,
        attempts: Vec<(&str, u32)>,
    ) -> Segment {
        Segment {
            id: SegmentId("s".into()),
            piece_id: PieceId("p".into()),
            label: None,
            rects: vec![],
            difficulty,
            tags: vec![],
            notes: String::new(),
            tempo_marking: None,
            dynamic_marking: None,
            expression_note: None,
            scope_history: vec![],
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            practice_history: attempts
                .into_iter()
                .enumerate()
                .map(|(i, (date, secs))| PracticeAttempt {
                    id: AttemptId(format!("a{}", i)),
                    segment_id: SegmentId("s".into()),
                    started_at: chrono::DateTime::parse_from_rfc3339(date)
                        .unwrap()
                        .with_timezone(&Utc),
                    duration_seconds: secs,
                    recording_ref: None,
                    self_rating_after: None,
                })
                .collect(),
            memorisation_state: MemorisationState::None,
            goal: None,
        }
    }

    #[test]
    fn struggling_unpractised_outscores_mastered_unpractised() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let struggling = seg_with_history(Difficulty::Struggling, vec![]);
        let mastered = seg_with_history(Difficulty::Mastered, vec![]);
        // Both unpractised → both very stale, but struggling weight × regular curve
        // should beat mastered weight × mastered curve (which caps at 2.0).
        assert!(score(&struggling, now) > score(&mastered, now));
    }

    #[test]
    fn just_practiced_struggling_still_has_meaningful_score() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        // Practised an hour ago for 10 minutes.
        let s = seg_with_history(Difficulty::Struggling, vec![("2026-06-01T11:00:00Z", 600)]);
        // weight 4 × staleness ~1.0 × under_invested 1.2 = ~4.8
        let value = score(&s, now);
        assert!(value > 4.0, "expected > 4.0, got {value}");
        assert!(value < 7.0, "expected < 7.0, got {value}");
    }

    #[test]
    fn never_practised_has_high_staleness() {
        let s = seg_with_history(Difficulty::Working, vec![]);
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(days_since_last_practice(&s, now), 10_000.0);
    }

    fn seg_in_piece(id: &str, piece: &str, difficulty: Difficulty) -> Segment {
        let mut s = seg_with_history(difficulty, vec![]);
        s.id = SegmentId(id.into());
        s.piece_id = PieceId(piece.into());
        s
    }

    #[test]
    fn pick_session_returns_top_n_when_only_one_piece() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let segs = vec![
            seg_in_piece("s1", "p", Difficulty::Struggling),
            seg_in_piece("s2", "p", Difficulty::Working),
            seg_in_piece("s3", "p", Difficulty::Solid),
        ];
        let picked = pick_session(&segs, 2, now);
        assert_eq!(picked, vec![SegmentId("s1".into()), SegmentId("s2".into())]);
    }

    #[test]
    fn pick_session_interleaves_across_pieces() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        // Two struggling segments in piece A, one struggling in piece B.
        // Raw score order would put both A's first; interleaving should put B in between.
        let segs = vec![
            seg_in_piece("a1", "A", Difficulty::Struggling),
            seg_in_piece("a2", "A", Difficulty::Struggling),
            seg_in_piece("b1", "B", Difficulty::Struggling),
        ];
        let picked = pick_session(&segs, 3, now);
        assert_eq!(picked[0], SegmentId("a1".into()));
        assert_eq!(picked[1], SegmentId("b1".into()));
        assert_eq!(picked[2], SegmentId("a2".into()));
    }

    #[test]
    fn pick_session_falls_back_when_no_alternative_piece() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        // Only one piece — interleaving has nothing to do, returns top-N as-is.
        let segs = vec![
            seg_in_piece("s1", "p", Difficulty::Struggling),
            seg_in_piece("s2", "p", Difficulty::Working),
        ];
        let picked = pick_session(&segs, 2, now);
        assert_eq!(picked, vec![SegmentId("s1".into()), SegmentId("s2".into())]);
    }

    #[test]
    fn pick_session_returns_empty_for_empty_input() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        assert!(pick_session(&[], 5, now).is_empty());
    }

    #[test]
    fn pick_session_returns_fewer_than_n_when_not_enough_segments() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let segs = vec![seg_in_piece("s1", "p", Difficulty::Struggling)];
        let picked = pick_session(&segs, 5, now);
        assert_eq!(picked.len(), 1);
    }
}
