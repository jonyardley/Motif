//! Stall detection: a derived signal that surfaces a nudge in the practice view
//! when a segment has been worked many times over multiple weeks with no
//! improvement. Triggers the v1 jumping-off feature (tag → suggestion table).

use crate::model::Segment;
use chrono::{DateTime, Duration, Utc};

/// True iff the segment has at least `attempt_threshold` practice attempts within
/// the last `week_threshold` weeks AND the highest self-rating recorded in that
/// window is no better than the earliest self-rating recorded in that window
/// (i.e. no upward movement toward Performance-Ready).
///
/// Attempts with no self-rating are ignored for the rating-comparison check
/// but still count toward the attempt threshold.
///
/// When *every* attempt in the window is unrated, returns `true`: the user keeps
/// practising without judging progress, which is exactly when the "try a different
/// exercise" nudge is most useful.
///
/// The window comparison uses *max* (not *latest*) rating intentionally — if the
/// user reached Confident at any point in the window, a temporary regression to
/// Learning should not be treated as a stall (the right intervention is rest
/// or consolidation, not a different exercise).
pub fn is_stalled(
    segment: &Segment,
    now: DateTime<Utc>,
    attempt_threshold: usize,
    week_threshold: i64,
) -> bool {
    let cutoff = now - Duration::weeks(week_threshold);
    let recent: Vec<_> = segment
        .practice_history
        .iter()
        .filter(|a| a.started_at >= cutoff)
        .collect();

    if recent.len() < attempt_threshold {
        return false;
    }

    let rated: Vec<_> = recent.iter().filter_map(|a| a.self_rating_after).collect();

    if rated.is_empty() {
        // Many attempts, no ratings — treat as stalled (user keeps practising without judging
        // progress; the nudge to try a different exercise is appropriate).
        return true;
    }

    let first_rank = rated.first().unwrap().rank();
    let max_rank = rated.iter().map(|d| d.rank()).max().unwrap();
    max_rank <= first_rank
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AttemptId, Difficulty, MemorisationState, PieceId, PracticeAttempt, ScopeStage, Segment,
        SegmentId,
    };
    use chrono::TimeZone;

    fn seg() -> Segment {
        Segment {
            id: SegmentId("s".into()),
            piece_id: PieceId("p".into()),
            label: None,
            rects: vec![],
            difficulty: Difficulty::Shaping,
            tags: vec![],
            notes: String::new(),
            tempo_marking: None,
            dynamic_marking: None,
            expression_note: None,
            scope_history: Vec::<ScopeStage>::new(),
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            practice_history: vec![],
            memorisation_state: MemorisationState::None,
            goal: None,
        }
    }

    fn att(date: &str, rating: Option<Difficulty>) -> PracticeAttempt {
        PracticeAttempt {
            id: AttemptId("a".into()),
            segment_id: SegmentId("s".into()),
            started_at: chrono::DateTime::parse_from_rfc3339(date)
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: rating,
        }
    }

    #[test]
    fn not_stalled_when_too_few_attempts() {
        let mut s = seg();
        s.practice_history
            .push(att("2026-05-30T10:00:00Z", Some(Difficulty::Learning)));
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        assert!(!is_stalled(&s, now, 6, 2));
    }

    #[test]
    fn stalled_when_many_attempts_no_improvement() {
        let mut s = seg();
        for date in [
            "2026-05-18T10:00:00Z",
            "2026-05-20T10:00:00Z",
            "2026-05-22T10:00:00Z",
            "2026-05-24T10:00:00Z",
            "2026-05-26T10:00:00Z",
            "2026-05-28T10:00:00Z",
        ] {
            s.practice_history
                .push(att(date, Some(Difficulty::Learning)));
        }
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        assert!(is_stalled(&s, now, 6, 2));
    }

    #[test]
    fn not_stalled_when_upward_rating_change_present() {
        let mut s = seg();
        s.practice_history
            .push(att("2026-05-18T10:00:00Z", Some(Difficulty::Learning)));
        s.practice_history
            .push(att("2026-05-20T10:00:00Z", Some(Difficulty::Learning)));
        s.practice_history
            .push(att("2026-05-22T10:00:00Z", Some(Difficulty::Learning)));
        s.practice_history
            .push(att("2026-05-24T10:00:00Z", Some(Difficulty::Learning)));
        s.practice_history
            .push(att("2026-05-26T10:00:00Z", Some(Difficulty::Shaping)));
        s.practice_history
            .push(att("2026-05-28T10:00:00Z", Some(Difficulty::Shaping)));
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        assert!(!is_stalled(&s, now, 6, 2));
    }

    #[test]
    fn stalled_when_all_attempts_unrated() {
        let mut s = seg();
        for date in [
            "2026-05-18T10:00:00Z",
            "2026-05-20T10:00:00Z",
            "2026-05-22T10:00:00Z",
            "2026-05-24T10:00:00Z",
            "2026-05-26T10:00:00Z",
            "2026-05-28T10:00:00Z",
        ] {
            s.practice_history.push(att(date, None));
        }
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        assert!(is_stalled(&s, now, 6, 2));
    }

    #[test]
    fn old_attempts_outside_window_dont_count() {
        let mut s = seg();
        // Six attempts spread over six weeks — only the last few fall inside the 2-week window.
        for date in [
            "2026-04-10T10:00:00Z",
            "2026-04-17T10:00:00Z",
            "2026-04-24T10:00:00Z",
            "2026-05-01T10:00:00Z",
            "2026-05-08T10:00:00Z",
            "2026-05-15T10:00:00Z",
        ] {
            s.practice_history
                .push(att(date, Some(Difficulty::Learning)));
        }
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        // Only one attempt within the last 2 weeks → below threshold of 6.
        assert!(!is_stalled(&s, now, 6, 2));
    }
}
