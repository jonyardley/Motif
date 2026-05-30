use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PieceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SegmentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttemptId(pub String);

/// A rectangle on a specific page in page-relative coordinates (0..page_width, 0..page_height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub page_id: PageId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A segment's current state on the practice journey from "just added" to
/// "concert-ready". See the design spec §2.2 for the rationale behind the
/// labels (positive framing, no "Struggling"; honest framing, no "Mastered").
///
/// `Fresh` is auto-assigned at construction and auto-transitions to `Learning`
/// when the first practice attempt is recorded (see [`Segment::record_attempt`]).
/// The user never picks `Fresh` from the UI — it's only ever an initial-state
/// marker. All other states are valid user-rated values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Fresh,
    Learning,
    Shaping,
    Confident,
    PerformanceReady,
}

impl Difficulty {
    /// 0..=4 ranking from "earliest in the journey" to "concert-ready". Used
    /// by stall detection to decide whether a self-rating change in a window
    /// represents upward movement.
    pub fn rank(self) -> u8 {
        match self {
            Difficulty::Fresh => 0,
            Difficulty::Learning => 1,
            Difficulty::Shaping => 2,
            Difficulty::Confident => 3,
            Difficulty::PerformanceReady => 4,
        }
    }

    /// Scheduler weight. `Fresh` and `Learning` share the highest weight (4.0)
    /// — both deserve attention; the difference between them is whether the
    /// segment has been worked yet, which the `under_invested_factor` surfaces
    /// separately. `PerformanceReady` is 1.0 (floor — never zero), so a
    /// concert-ready piece stays in long-interval rotation rather than being
    /// dropped from consideration.
    pub fn weight(self) -> f64 {
        match self {
            Difficulty::Fresh | Difficulty::Learning => 4.0,
            Difficulty::Shaping => 3.0,
            Difficulty::Confident => 2.0,
            Difficulty::PerformanceReady => 1.0,
        }
    }

    /// Step one level back toward the start of the journey. Used when a
    /// segment's scope is expanded — default behaviour is to re-rate the
    /// larger scope one step easier than the kernel.
    ///
    /// Saturates at `Learning` for any non-`Fresh` value (you don't go
    /// backwards into `Fresh`, which represents "never practised"). `Fresh`
    /// itself saturates at `Fresh` — expanding the scope of a never-practised
    /// segment leaves it never-practised on the bigger envelope.
    pub fn one_step_lower(self) -> Difficulty {
        match self {
            Difficulty::Fresh => Difficulty::Fresh,
            Difficulty::Learning => Difficulty::Learning,
            Difficulty::Shaping => Difficulty::Learning,
            Difficulty::Confident => Difficulty::Shaping,
            Difficulty::PerformanceReady => Difficulty::Confident,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorisationState {
    #[default]
    None,
    /// Actively committing the segment to memory. (Renamed from `Learning`
    /// to avoid collision with `Difficulty::Learning`, which describes a
    /// different concept on a different axis.)
    Practising,
    Memorised,
    Verified,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticeAttempt {
    pub id: AttemptId,
    pub segment_id: SegmentId,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: u32,
    pub recording_ref: Option<String>,
    pub self_rating_after: Option<Difficulty>,
}

/// A historical snapshot of a segment's scope before it was expanded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeStage {
    pub rects: Vec<Rect>,
    pub difficulty_at_promotion: Difficulty,
    pub promoted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub id: SegmentId,
    pub piece_id: PieceId,
    pub label: Option<String>,
    pub rects: Vec<Rect>,
    pub difficulty: Difficulty,
    pub tags: Vec<String>,
    pub notes: String,

    // Caption-strip metadata (driving the practice view's text caption)
    pub tempo_marking: Option<String>,
    pub dynamic_marking: Option<String>,
    pub expression_note: Option<String>,

    // Scope evolution
    pub scope_history: Vec<ScopeStage>,

    pub created_at: DateTime<Utc>,
    pub practice_history: Vec<PracticeAttempt>,

    // v2/v3 model hooks (unused in v1 scheduler)
    #[serde(default)]
    pub memorisation_state: MemorisationState,
    pub goal: Option<String>,
}

impl Segment {
    /// The end-time of the most recent attempt (start + duration). `None` if no attempts.
    pub fn last_practiced_at(&self) -> Option<DateTime<Utc>> {
        self.practice_history
            .iter()
            .map(|a| a.started_at + chrono::Duration::seconds(a.duration_seconds as i64))
            .max()
    }

    /// Total seconds practiced across all attempts.
    pub fn total_seconds_practiced(&self) -> u64 {
        self.practice_history
            .iter()
            .map(|a| a.duration_seconds as u64)
            .sum()
    }

    /// Append a practice attempt to the segment's history.
    ///
    /// If the segment is currently `Fresh` (never practised), this also
    /// auto-transitions it to `Learning`. After this point the user can
    /// rate it explicitly via `self_rate`. `Fresh` is only ever an
    /// initial-state marker and never reachable as a user-picked rating.
    pub fn record_attempt(&mut self, attempt: PracticeAttempt) {
        if self.difficulty == Difficulty::Fresh {
            self.difficulty = Difficulty::Learning;
        }
        self.practice_history.push(attempt);
    }

    /// Set the segment's current difficulty rating and annotate the most-recent
    /// attempt (if any) with the same rating. Calling this without any practice
    /// history still updates the segment's `difficulty` field — useful for the
    /// editor's "set initial difficulty" flow.
    pub fn self_rate(&mut self, rating: Difficulty) {
        self.difficulty = rating;
        if let Some(last) = self.practice_history.last_mut() {
            last.self_rating_after = Some(rating);
        }
    }

    /// Construct a new Segment with sensible defaults: difficulty `Fresh`
    /// (auto-transitions to `Learning` on the first recorded attempt),
    /// empty tags / notes / scope_history / practice_history, no caption
    /// metadata, `MemorisationState::None`, no goal. Caller supplies the
    /// identity, the piece it belongs to, its rect mask, and the creation
    /// timestamp.
    pub fn new(
        id: SegmentId,
        piece_id: PieceId,
        rects: Vec<Rect>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Segment {
            id,
            piece_id,
            label: None,
            rects,
            difficulty: Difficulty::Fresh,
            tags: Vec::new(),
            notes: String::new(),
            tempo_marking: None,
            dynamic_marking: None,
            expression_note: None,
            scope_history: Vec::new(),
            created_at,
            practice_history: Vec::new(),
            memorisation_state: MemorisationState::None,
            goal: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub index: u32,
    pub image_ref: String,
    pub width: u32,
    pub height: u32,
}

impl Page {
    /// Construct a new Page. `image_ref` is an opaque shell-supplied handle
    /// (e.g. a file URI) — the core never reads the actual image.
    pub fn new(id: PageId, index: u32, image_ref: String, width: u32, height: u32) -> Self {
        Page {
            id,
            index,
            image_ref,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub id: PieceId,
    pub title: String,
    pub composer: Option<String>,
    pub created_at: DateTime<Utc>,
    pub pages: Vec<Page>,
    pub segments: Vec<Segment>,
}

impl Piece {
    /// Construct a new Piece with no pages and no segments yet. Pages are added
    /// after capture; segments are added by the segment editor.
    pub fn new(
        id: PieceId,
        title: String,
        composer: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Piece {
            id,
            title,
            composer,
            created_at,
            pages: Vec::new(),
            segments: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_newtypes_roundtrip_serde() {
        let p = PieceId("abc".into());
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"abc\"");
        let back: PieceId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn rect_roundtrips() {
        let r = Rect {
            page_id: PageId("p1".into()),
            x: 1.0,
            y: 2.0,
            w: 30.0,
            h: 40.0,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn difficulty_weights_match_spec() {
        // Fresh and Learning share top weight — both need attention; the
        // under_invested_factor surfaces Fresh further via the no-time boost.
        assert_eq!(Difficulty::Fresh.weight(), 4.0);
        assert_eq!(Difficulty::Learning.weight(), 4.0);
        assert_eq!(Difficulty::Shaping.weight(), 3.0);
        assert_eq!(Difficulty::Confident.weight(), 2.0);
        assert_eq!(Difficulty::PerformanceReady.weight(), 1.0);
    }

    #[test]
    fn difficulty_rank_is_ordered() {
        assert!(Difficulty::Fresh.rank() < Difficulty::Learning.rank());
        assert!(Difficulty::Learning.rank() < Difficulty::Shaping.rank());
        assert!(Difficulty::Shaping.rank() < Difficulty::Confident.rank());
        assert!(Difficulty::Confident.rank() < Difficulty::PerformanceReady.rank());
    }

    #[test]
    fn difficulty_one_step_lower_saturates_at_learning_and_at_fresh() {
        // Normal walk down the chain.
        assert_eq!(
            Difficulty::PerformanceReady.one_step_lower(),
            Difficulty::Confident
        );
        assert_eq!(Difficulty::Confident.one_step_lower(), Difficulty::Shaping);
        assert_eq!(Difficulty::Shaping.one_step_lower(), Difficulty::Learning);

        // Saturation at Learning — you don't walk backwards into Fresh,
        // which represents "never practised". A Learning segment whose
        // scope is expanded stays Learning on the bigger envelope.
        assert_eq!(Difficulty::Learning.one_step_lower(), Difficulty::Learning);

        // Fresh itself saturates: expanding a never-practised segment
        // leaves it never-practised on the bigger envelope.
        assert_eq!(Difficulty::Fresh.one_step_lower(), Difficulty::Fresh);
    }

    #[test]
    fn memorisation_state_default_is_none() {
        let m: MemorisationState = Default::default();
        assert_eq!(m, MemorisationState::None);
    }

    #[test]
    fn practice_attempt_roundtrips() {
        let a = PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-29T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 120,
            recording_ref: Some("rec://abc".into()),
            self_rating_after: Some(Difficulty::Shaping),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: PracticeAttempt = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn scope_stage_roundtrips() {
        let s = ScopeStage {
            rects: vec![Rect {
                page_id: PageId("p1".into()),
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            }],
            difficulty_at_promotion: Difficulty::Confident,
            promoted_at: DateTime::parse_from_rfc3339("2026-05-29T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: ScopeStage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    fn make_segment(id: &str, piece: &str, difficulty: Difficulty) -> Segment {
        Segment {
            id: SegmentId(id.into()),
            piece_id: PieceId(piece.into()),
            label: None,
            rects: vec![],
            difficulty,
            tags: vec![],
            notes: String::new(),
            tempo_marking: None,
            dynamic_marking: None,
            expression_note: None,
            scope_history: vec![],
            created_at: DateTime::parse_from_rfc3339("2026-05-29T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            practice_history: vec![],
            memorisation_state: MemorisationState::None,
            goal: None,
        }
    }

    #[test]
    fn segment_roundtrips() {
        let s = make_segment("s1", "p1", Difficulty::Shaping);
        let json = serde_json::to_string(&s).unwrap();
        let back: Segment = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn piece_roundtrips_with_pages_and_segments() {
        let piece = Piece {
            id: PieceId("piece-1".into()),
            title: "Ballade No. 1 in G minor".into(),
            composer: Some("Chopin".into()),
            created_at: DateTime::parse_from_rfc3339("2026-05-29T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            pages: vec![Page {
                id: PageId("page-1".into()),
                index: 0,
                image_ref: "file://page-1.jpg".into(),
                width: 1500,
                height: 2000,
            }],
            segments: vec![make_segment("seg-1", "piece-1", Difficulty::Learning)],
        };
        let json = serde_json::to_string(&piece).unwrap();
        let back: Piece = serde_json::from_str(&json).unwrap();
        assert_eq!(back, piece);
    }

    #[test]
    fn segment_helpers_summarise_practice_history() {
        let mut s = make_segment("s1", "p1", Difficulty::Shaping);
        assert_eq!(s.last_practiced_at(), None);
        assert_eq!(s.total_seconds_practiced(), 0);

        s.practice_history.push(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-20T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        s.practice_history.push(PracticeAttempt {
            id: AttemptId("a2".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-22T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 90,
            recording_ref: None,
            self_rating_after: None,
        });

        let end_of_second = DateTime::parse_from_rfc3339("2026-05-22T10:01:30Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(s.last_practiced_at(), Some(end_of_second));
        assert_eq!(s.total_seconds_practiced(), 150);
    }

    #[test]
    fn record_attempt_appends_to_history() {
        let mut s = make_segment("s1", "p1", Difficulty::Shaping);
        assert!(s.practice_history.is_empty());
        s.record_attempt(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        assert_eq!(s.practice_history.len(), 1);
    }

    #[test]
    fn self_rate_updates_difficulty_and_last_attempt() {
        let mut s = make_segment("s1", "p1", Difficulty::Learning);
        s.record_attempt(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        s.self_rate(Difficulty::Shaping);
        assert_eq!(s.difficulty, Difficulty::Shaping);
        assert_eq!(
            s.practice_history[0].self_rating_after,
            Some(Difficulty::Shaping)
        );
    }

    #[test]
    fn self_rate_without_history_still_updates_difficulty() {
        let mut s = make_segment("s1", "p1", Difficulty::Learning);
        s.self_rate(Difficulty::Confident);
        assert_eq!(s.difficulty, Difficulty::Confident);
    }

    #[test]
    fn segment_new_uses_fresh_difficulty_and_empty_collections() {
        let now = DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let s = Segment::new(
            SegmentId("s1".into()),
            PieceId("p1".into()),
            vec![Rect {
                page_id: PageId("page-1".into()),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            }],
            now,
        );
        assert_eq!(s.difficulty, Difficulty::Fresh);
        assert!(s.tags.is_empty());
        assert!(s.notes.is_empty());
        assert!(s.scope_history.is_empty());
        assert!(s.practice_history.is_empty());
        assert_eq!(s.memorisation_state, MemorisationState::None);
        assert_eq!(s.goal, None);
        assert_eq!(s.rects.len(), 1);
        assert_eq!(s.created_at, now);
    }

    #[test]
    fn record_attempt_auto_transitions_fresh_to_learning() {
        let now = DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut s = Segment::new(
            SegmentId("s1".into()),
            PieceId("p1".into()),
            vec![Rect {
                page_id: PageId("page-1".into()),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            }],
            now,
        );
        assert_eq!(s.difficulty, Difficulty::Fresh);

        s.record_attempt(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: now,
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });

        assert_eq!(s.difficulty, Difficulty::Learning);
        assert_eq!(s.practice_history.len(), 1);
    }

    #[test]
    fn record_attempt_does_not_overwrite_non_fresh_difficulty() {
        let mut s = make_segment("s1", "p1", Difficulty::Confident);
        s.record_attempt(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        // Should still be Confident, not silently re-classified.
        assert_eq!(s.difficulty, Difficulty::Confident);
    }

    #[test]
    fn page_new_constructs_directly() {
        let p = Page::new(PageId("pg".into()), 0, "file://x.jpg".into(), 1500, 2000);
        assert_eq!(p.index, 0);
        assert_eq!(p.width, 1500);
        assert_eq!(p.height, 2000);
        assert_eq!(p.image_ref, "file://x.jpg");
    }

    #[test]
    fn piece_new_is_empty_until_pages_or_segments_added() {
        let now = DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let p = Piece::new(PieceId("pc".into()), "Test".into(), None, now);
        assert!(p.pages.is_empty());
        assert!(p.segments.is_empty());
        assert_eq!(p.composer, None);
        assert_eq!(p.created_at, now);
    }
}
