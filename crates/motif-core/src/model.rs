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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorisationState {
    None,
    Learning,
    Memorised,
    Verified,
}

impl Default for MemorisationState {
    fn default() -> Self { MemorisationState::None }
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
}
