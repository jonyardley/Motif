//! End-to-end behavioural test: simulate a few weeks of practice and assert
//! that the scheduler picks reasonable segments, scope expansion archives
//! correctly, and stall detection fires when expected.

use chrono::{Duration, TimeZone, Utc};
use motif_core::model::{
    AttemptId, Difficulty, MemorisationState, PageId, Piece, PieceId, PracticeAttempt, Rect,
    Segment, SegmentId,
};
use motif_core::scheduler::pick_session;
use motif_core::stall::is_stalled;

fn make_segment(id: &str, piece: &str, difficulty: Difficulty) -> Segment {
    Segment {
        id: SegmentId(id.into()),
        piece_id: PieceId(piece.into()),
        label: None,
        rects: vec![Rect {
            page_id: PageId("page-1".into()),
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 50.0,
        }],
        difficulty,
        tags: vec![],
        notes: String::new(),
        tempo_marking: None,
        dynamic_marking: None,
        expression_note: None,
        scope_history: vec![],
        created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        practice_history: vec![],
        memorisation_state: MemorisationState::None,
        goal: None,
    }
}

#[test]
fn scheduler_prioritises_struggling_and_neglected_over_mastered_and_recent() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

    let mut struggling = make_segment("s-hard", "piece", Difficulty::Struggling);
    // 4 days since last practice
    struggling.record_attempt(PracticeAttempt {
        id: AttemptId("a1".into()),
        segment_id: SegmentId("s-hard".into()),
        started_at: now - Duration::days(4),
        duration_seconds: 300,
        recording_ref: None,
        self_rating_after: None,
    });

    let mut mastered_fresh = make_segment("s-easy-fresh", "piece", Difficulty::Mastered);
    mastered_fresh.record_attempt(PracticeAttempt {
        id: AttemptId("a2".into()),
        segment_id: SegmentId("s-easy-fresh".into()),
        started_at: now - Duration::hours(2),
        duration_seconds: 300,
        recording_ref: None,
        self_rating_after: None,
    });

    let picked = pick_session(&[struggling, mastered_fresh], 1, now);
    assert_eq!(picked, vec![SegmentId("s-hard".into())]);
}

#[test]
fn scope_expansion_preserves_practice_history_across_stages() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let mut s = make_segment("s", "piece", Difficulty::Solid);

    s.record_attempt(PracticeAttempt {
        id: AttemptId("a1".into()),
        segment_id: SegmentId("s".into()),
        started_at: now - Duration::days(3),
        duration_seconds: 600,
        recording_ref: None,
        self_rating_after: Some(Difficulty::Solid),
    });

    s.expand_scope(
        vec![Rect {
            page_id: PageId("page-1".into()),
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 50.0,
        }],
        now,
        true,
    );

    assert_eq!(s.difficulty, Difficulty::Working, "default expand drops one step");
    assert_eq!(s.scope_history.len(), 1);
    assert_eq!(
        s.practice_history.len(),
        1,
        "practice history should travel with the segment across scope changes"
    );
}

#[test]
fn stalled_segment_in_realistic_three_week_history() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let mut s = make_segment("s", "piece", Difficulty::Struggling);

    // Eight attempts over 20 days, all rated Struggling.
    for i in 0..8 {
        s.record_attempt(PracticeAttempt {
            id: AttemptId(format!("a{}", i)),
            segment_id: SegmentId("s".into()),
            started_at: now - Duration::days(20 - i * 2),
            duration_seconds: 300,
            recording_ref: None,
            self_rating_after: Some(Difficulty::Struggling),
        });
    }

    assert!(is_stalled(&s, now, 6, 3));
}

#[test]
fn piece_with_segments_roundtrips_through_json() {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let piece = Piece {
        id: PieceId("p1".into()),
        title: "Test Piece".into(),
        composer: None,
        created_at: now,
        pages: vec![],
        segments: vec![
            make_segment("s1", "p1", Difficulty::Struggling),
            make_segment("s2", "p1", Difficulty::Solid),
        ],
    };
    let json = serde_json::to_string(&piece).unwrap();
    let back: Piece = serde_json::from_str(&json).unwrap();
    assert_eq!(back, piece);
}
