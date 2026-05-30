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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Struggling,
    Working,
    Solid,
    Mastered,
}

impl Difficulty {
    /// 0..=3 ranking, higher = closer to mastered. Used to decide whether
    /// a difficulty change was "upward" (improvement) or not.
    pub fn rank(self) -> u8 {
        match self {
            Difficulty::Struggling => 0,
            Difficulty::Working => 1,
            Difficulty::Solid => 2,
            Difficulty::Mastered => 3,
        }
    }

    /// Scheduler weight. Mastered is 1.0 (floor — never zero), Struggling 4.0.
    pub fn weight(self) -> f64 {
        match self {
            Difficulty::Struggling => 4.0,
            Difficulty::Working => 3.0,
            Difficulty::Solid => 2.0,
            Difficulty::Mastered => 1.0,
        }
    }

    /// Step one level down (toward Struggling). Saturates at Struggling.
    /// Used when a segment's scope is expanded — default behaviour is to
    /// re-rate the larger scope one step easier than the kernel.
    pub fn one_step_lower(self) -> Difficulty {
        match self {
            Difficulty::Struggling => Difficulty::Struggling,
            Difficulty::Working => Difficulty::Struggling,
            Difficulty::Solid => Difficulty::Working,
            Difficulty::Mastered => Difficulty::Solid,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorisationState {
    #[default]
    None,
    Learning,
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
        self.practice_history.iter().map(|a| a.duration_seconds as u64).sum()
    }

    /// Append a practice attempt to the segment's history.
    pub fn record_attempt(&mut self, attempt: PracticeAttempt) {
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub index: u32,
    pub image_ref: String,
    pub width: u32,
    pub height: u32,
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
        let r = Rect { page_id: PageId("p1".into()), x: 1.0, y: 2.0, w: 30.0, h: 40.0 };
        let json = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn difficulty_weights_match_spec() {
        assert_eq!(Difficulty::Struggling.weight(), 4.0);
        assert_eq!(Difficulty::Working.weight(), 3.0);
        assert_eq!(Difficulty::Solid.weight(), 2.0);
        assert_eq!(Difficulty::Mastered.weight(), 1.0);
    }

    #[test]
    fn difficulty_rank_is_ordered() {
        assert!(Difficulty::Struggling.rank() < Difficulty::Working.rank());
        assert!(Difficulty::Working.rank() < Difficulty::Solid.rank());
        assert!(Difficulty::Solid.rank() < Difficulty::Mastered.rank());
    }

    #[test]
    fn difficulty_one_step_lower_saturates_at_struggling() {
        assert_eq!(Difficulty::Mastered.one_step_lower(), Difficulty::Solid);
        assert_eq!(Difficulty::Solid.one_step_lower(), Difficulty::Working);
        assert_eq!(Difficulty::Working.one_step_lower(), Difficulty::Struggling);
        assert_eq!(Difficulty::Struggling.one_step_lower(), Difficulty::Struggling);
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
            started_at: DateTime::parse_from_rfc3339("2026-05-29T10:00:00Z").unwrap().with_timezone(&Utc),
            duration_seconds: 120,
            recording_ref: Some("rec://abc".into()),
            self_rating_after: Some(Difficulty::Working),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: PracticeAttempt = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn scope_stage_roundtrips() {
        let s = ScopeStage {
            rects: vec![Rect { page_id: PageId("p1".into()), x: 0.0, y: 0.0, w: 10.0, h: 10.0 }],
            difficulty_at_promotion: Difficulty::Solid,
            promoted_at: DateTime::parse_from_rfc3339("2026-05-29T10:00:00Z").unwrap().with_timezone(&Utc),
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
            created_at: DateTime::parse_from_rfc3339("2026-05-29T00:00:00Z").unwrap().with_timezone(&Utc),
            practice_history: vec![],
            memorisation_state: MemorisationState::None,
            goal: None,
        }
    }

    #[test]
    fn segment_roundtrips() {
        let s = make_segment("s1", "p1", Difficulty::Working);
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
            created_at: DateTime::parse_from_rfc3339("2026-05-29T00:00:00Z").unwrap().with_timezone(&Utc),
            pages: vec![Page {
                id: PageId("page-1".into()),
                index: 0,
                image_ref: "file://page-1.jpg".into(),
                width: 1500,
                height: 2000,
            }],
            segments: vec![make_segment("seg-1", "piece-1", Difficulty::Struggling)],
        };
        let json = serde_json::to_string(&piece).unwrap();
        let back: Piece = serde_json::from_str(&json).unwrap();
        assert_eq!(back, piece);
    }

    #[test]
    fn segment_helpers_summarise_practice_history() {
        let mut s = make_segment("s1", "p1", Difficulty::Working);
        assert_eq!(s.last_practiced_at(), None);
        assert_eq!(s.total_seconds_practiced(), 0);

        s.practice_history.push(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-20T10:00:00Z").unwrap().with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        s.practice_history.push(PracticeAttempt {
            id: AttemptId("a2".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-22T10:00:00Z").unwrap().with_timezone(&Utc),
            duration_seconds: 90,
            recording_ref: None,
            self_rating_after: None,
        });

        let end_of_second = DateTime::parse_from_rfc3339("2026-05-22T10:01:30Z").unwrap().with_timezone(&Utc);
        assert_eq!(s.last_practiced_at(), Some(end_of_second));
        assert_eq!(s.total_seconds_practiced(), 150);
    }

    #[test]
    fn record_attempt_appends_to_history() {
        let mut s = make_segment("s1", "p1", Difficulty::Working);
        assert!(s.practice_history.is_empty());
        s.record_attempt(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z").unwrap().with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        assert_eq!(s.practice_history.len(), 1);
    }

    #[test]
    fn self_rate_updates_difficulty_and_last_attempt() {
        let mut s = make_segment("s1", "p1", Difficulty::Struggling);
        s.record_attempt(PracticeAttempt {
            id: AttemptId("a1".into()),
            segment_id: SegmentId("s1".into()),
            started_at: DateTime::parse_from_rfc3339("2026-05-30T10:00:00Z").unwrap().with_timezone(&Utc),
            duration_seconds: 60,
            recording_ref: None,
            self_rating_after: None,
        });
        s.self_rate(Difficulty::Working);
        assert_eq!(s.difficulty, Difficulty::Working);
        assert_eq!(s.practice_history[0].self_rating_after, Some(Difficulty::Working));
    }

    #[test]
    fn self_rate_without_history_still_updates_difficulty() {
        let mut s = make_segment("s1", "p1", Difficulty::Struggling);
        s.self_rate(Difficulty::Solid);
        assert_eq!(s.difficulty, Difficulty::Solid);
    }
}
