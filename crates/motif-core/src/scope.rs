//! Scope evolution: growing a segment outward from a kernel to a larger unit of
//! practice. The previous rects are archived in `scope_history`; the user is
//! prompted to re-rate difficulty (defaulting to one step lower, per the spec).

use crate::model::{Rect, ScopeStage, Segment};
use chrono::{DateTime, Utc};

impl Segment {
    /// Replace the segment's current scope with new rects. The previous rects
    /// become a `ScopeStage` history entry along with the difficulty rating at
    /// promotion. If `drop_difficulty` is true (the spec default), the segment's
    /// difficulty drops one step (e.g. Confident → Shaping). Otherwise difficulty
    /// stays where it was, which is appropriate when the user has explicitly
    /// rated the larger scope before expanding.
    pub fn expand_scope(
        &mut self,
        new_rects: Vec<Rect>,
        now: DateTime<Utc>,
        drop_difficulty: bool,
    ) {
        debug_assert!(
            !new_rects.is_empty(),
            "expand_scope requires at least one rect; a segment with no rects is unrenderable",
        );
        let previous_rects = std::mem::replace(&mut self.rects, new_rects);
        self.scope_history.push(ScopeStage {
            rects: previous_rects,
            difficulty_at_promotion: self.difficulty,
            promoted_at: now,
        });
        if drop_difficulty {
            self.difficulty = self.difficulty.one_step_lower();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Difficulty, MemorisationState, PageId, PieceId, SegmentId};
    use chrono::TimeZone;

    fn seg() -> Segment {
        Segment {
            id: SegmentId("s".into()),
            piece_id: PieceId("p".into()),
            label: None,
            rects: vec![Rect {
                page_id: PageId("page-1".into()),
                x: 10.0,
                y: 10.0,
                w: 50.0,
                h: 50.0,
            }],
            difficulty: Difficulty::Confident,
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
    fn expand_scope_archives_old_rects_and_drops_difficulty_by_default() {
        let mut s = seg();
        let original_rects = s.rects.clone();
        let new_rects = vec![Rect {
            page_id: PageId("page-1".into()),
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        }];
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();

        s.expand_scope(new_rects.clone(), now, true);

        assert_eq!(s.rects, new_rects);
        assert_eq!(s.scope_history.len(), 1);
        assert_eq!(s.scope_history[0].rects, original_rects);
        assert_eq!(
            s.scope_history[0].difficulty_at_promotion,
            Difficulty::Confident
        );
        assert_eq!(s.scope_history[0].promoted_at, now);
        assert_eq!(s.difficulty, Difficulty::Shaping);
    }

    #[test]
    fn expand_scope_preserves_difficulty_when_requested() {
        let mut s = seg();
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        let new_rects = vec![Rect {
            page_id: PageId("page-1".into()),
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        }];
        s.expand_scope(new_rects, now, false);
        assert_eq!(s.difficulty, Difficulty::Confident);
    }

    #[test]
    fn expand_scope_chains_multiple_stages() {
        let mut s = seg();
        let t1 = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap();
        s.expand_scope(
            vec![Rect {
                page_id: PageId("p".into()),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            }],
            t1,
            true,
        );
        s.expand_scope(
            vec![Rect {
                page_id: PageId("p".into()),
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 300.0,
            }],
            t2,
            true,
        );
        assert_eq!(s.scope_history.len(), 2);
        assert_eq!(s.scope_history[0].promoted_at, t1);
        assert_eq!(s.scope_history[1].promoted_at, t2);
        // Started Confident → Shaping (after first expand) → Learning (after second)
        assert_eq!(s.difficulty, Difficulty::Learning);
    }
}
