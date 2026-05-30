//! End-to-end behavioural test: simulate a few weeks of practice and assert
//! that the scheduler picks reasonable segments, scope expansion archives
//! correctly, and stall detection fires when expected.

use chrono::{Duration, TimeZone, Utc};
use motif_core::model::{
    AttemptId, Difficulty, MemorisationState, PageId, Piece, PieceId, PracticeAttempt, Rect,
    Segment, SegmentId,
};
use motif_core::scheduler::{pick_session, score};
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
fn scheduler_prioritises_learning_and_neglected_over_performance_ready_and_recent() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

    let mut learning_segment = make_segment("s-hard", "piece", Difficulty::Learning);
    // 4 days since last practice
    learning_segment.record_attempt(PracticeAttempt {
        id: AttemptId("a1".into()),
        segment_id: SegmentId("s-hard".into()),
        started_at: now - Duration::days(4),
        duration_seconds: 300,
        recording_ref: None,
        self_rating_after: None,
    });

    let mut recent_performance_ready =
        make_segment("s-easy-recent", "piece", Difficulty::PerformanceReady);
    recent_performance_ready.record_attempt(PracticeAttempt {
        id: AttemptId("a2".into()),
        segment_id: SegmentId("s-easy-recent".into()),
        started_at: now - Duration::hours(2),
        duration_seconds: 300,
        recording_ref: None,
        self_rating_after: None,
    });

    let picked = pick_session(&[learning_segment, recent_performance_ready], 1, now);
    assert_eq!(picked, vec![SegmentId("s-hard".into())]);
}

#[test]
fn scope_expansion_preserves_practice_history_across_stages() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let mut s = make_segment("s", "piece", Difficulty::Confident);

    s.record_attempt(PracticeAttempt {
        id: AttemptId("a1".into()),
        segment_id: SegmentId("s".into()),
        started_at: now - Duration::days(3),
        duration_seconds: 600,
        recording_ref: None,
        self_rating_after: Some(Difficulty::Confident),
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

    assert_eq!(
        s.difficulty,
        Difficulty::Shaping,
        "default expand drops one step"
    );
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
    let mut s = make_segment("s", "piece", Difficulty::Learning);

    // Eight attempts over 20 days, all rated Learning.
    for i in 0..8 {
        s.record_attempt(PracticeAttempt {
            id: AttemptId(format!("a{}", i)),
            segment_id: SegmentId("s".into()),
            started_at: now - Duration::days(20 - i * 2),
            duration_seconds: 300,
            recording_ref: None,
            self_rating_after: Some(Difficulty::Learning),
        });
    }

    assert!(is_stalled(&s, now, 6, 3));
}

#[test]
fn piece_with_pages_and_evolved_segments_roundtrips_through_json() {
    // Behavioural roundtrip: a real-world-shaped Piece (with a Page and a Segment
    // whose scope has evolved) survives serde JSON serialisation. Complements the
    // unit test in model.rs by exercising the scope_history field across the
    // crate boundary.
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();

    let mut evolved_segment = make_segment("seg-evolved", "p1", Difficulty::Confident);
    // Simulate a chunk-up: kernel grew to a wider rect, with the original archived.
    evolved_segment.expand_scope(
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

    let piece = Piece {
        id: PieceId("p1".into()),
        title: "Test Piece".into(),
        composer: Some("J. S. Bach".into()),
        created_at: now,
        pages: vec![motif_core::model::Page {
            id: PageId("page-1".into()),
            index: 0,
            image_ref: "file://page-1.jpg".into(),
            width: 1500,
            height: 2000,
        }],
        segments: vec![
            make_segment("s-learning", "p1", Difficulty::Learning),
            evolved_segment,
        ],
    };

    let json = serde_json::to_string(&piece).unwrap();
    let back: Piece = serde_json::from_str(&json).unwrap();
    assert_eq!(back, piece);
    // Specifically confirm scope_history made it through.
    assert_eq!(back.segments[1].scope_history.len(), 1);
}

#[test]
fn under_invested_boost_does_not_crowd_out_stale_performance_ready() {
    // Operational invariant: a Performance-Ready segment that has been kept in
    // long-interval rotation should outscore a Performance-Ready segment that
    // was rated Performance-Ready just yesterday with little time invested.
    // The under-invested boost must not let recent Performance-Ready segments
    // crowd out stale Performance-Ready ones.
    //
    // Asserts the *ratio* (stale dominates by at least 4x) rather than the
    // exact numbers, so the invariant survives plausible future scoring
    // refinements (e.g. tweaking the under-invested curve or extending the
    // Performance-Ready staleness cap) as long as the qualitative property
    // still holds.
    //
    // Note for v2: a Performance-Ready segment with no practice history at
    // all is a semantically odd state (the user marked it Performance-Ready
    // without ever practising) and the current scoring treats it as fully
    // stale via the 10_000-day sentinel. That's defensible in v1 —
    // Performance-Ready + zero history is unlikely outside an editor "set
    // initial difficulty" edge case — but worth revisiting if it becomes
    // common.
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

    let mut recent_performance_ready =
        make_segment("recent", "piece", Difficulty::PerformanceReady);
    recent_performance_ready.record_attempt(PracticeAttempt {
        id: AttemptId("a-recent".into()),
        segment_id: SegmentId("recent".into()),
        started_at: now - Duration::days(1),
        duration_seconds: 60 * 5, // 5 minutes — lands in the 1.2x under-invested bucket
        recording_ref: None,
        self_rating_after: None,
    });

    let mut stale_performance_ready = make_segment("stale", "piece", Difficulty::PerformanceReady);
    stale_performance_ready.record_attempt(PracticeAttempt {
        id: AttemptId("a-stale".into()),
        segment_id: SegmentId("stale".into()),
        started_at: now - Duration::days(90), // deep in the long-rotation regime
        duration_seconds: 60 * 35,            // 35 minutes — above the under-invested threshold
        recording_ref: None,
        self_rating_after: None,
    });

    let recent_score = score(&recent_performance_ready, now);
    let stale_score = score(&stale_performance_ready, now);

    assert!(
        stale_score >= recent_score * 4.0,
        "stale Performance-Ready should dominate recent Performance-Ready by at least 4x; got stale={stale_score}, recent={recent_score}",
    );

    // Also assert the operational consequence: the picker picks the stale one.
    let picked = pick_session(&[recent_performance_ready, stale_performance_ready], 1, now);
    assert_eq!(picked, vec![SegmentId("stale".into())]);
}
