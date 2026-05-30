# Motif Core Foundations Implementation Plan (Phase 1 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the pure-Rust core library `motif-core` that owns the entire Motif data model, scoring function, scheduler with interleaving, scope evolution, and stall detection — fully unit-tested, with no iOS or Crux dependencies yet.

**Architecture:** Single Cargo workspace with one crate (`motif-core`). All types serialise via serde. All functions that depend on the current time take `now: DateTime<Utc>` as an explicit argument — no wall-clock access, so everything is deterministic and testable. IDs are passed in as `String` newtypes; the core never generates UUIDs (that's the shell's job).

**Tech Stack:** Rust (edition 2021), `serde` + `serde_json` for serialisation, `chrono` for time, `uuid` for tests only.

**Files this plan will create:**

- `Cargo.toml` — workspace root
- `crates/motif-core/Cargo.toml` — crate manifest
- `crates/motif-core/src/lib.rs` — public re-exports
- `crates/motif-core/src/model.rs` — `Rect`, `Difficulty`, `MemorisationState`, `PracticeAttempt`, `ScopeStage`, `Segment`, `Page`, `Piece`, ID newtypes, segment mutation methods
- `crates/motif-core/src/scheduler.rs` — `difficulty_weight`, `staleness_factor`, `staleness_factor_mastered`, `under_invested_factor`, `score`, `pick_session`
- `crates/motif-core/src/stall.rs` — `is_stalled`
- `crates/motif-core/src/scope.rs` — `expand_scope` helper (operates on `&mut Segment`)
- `crates/motif-core/tests/integration.rs` — end-to-end behaviour test

**Files this plan will NOT touch (deferred to later phases):**

- Anything related to Crux App trait, Capabilities, Effects, ViewModels
- iOS Swift code
- Image storage, audio recording, Vision integration
- Persistence beyond `serde_json` roundtripping

---

## Task 1: Initialise the Cargo workspace

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Create the workspace manifest**

Write `Cargo.toml` at the repository root:

```toml
[workspace]
resolver = "2"
members = ["crates/motif-core"]
```

- [ ] **Step 2: Verify Cargo reads it (the crate doesn't exist yet so this should fail informatively)**

Run: `cargo metadata --no-deps --format-version 1 2>&1 | head -3`
Expected: an error referencing missing `crates/motif-core/Cargo.toml`. This confirms the workspace declaration is parsed.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: initialise cargo workspace"
```

---

## Task 2: Create the `motif-core` crate skeleton

**Files:**
- Create: `crates/motif-core/Cargo.toml`
- Create: `crates/motif-core/src/lib.rs`

- [ ] **Step 1: Create the crate manifest**

Write `crates/motif-core/Cargo.toml`:

```toml
[package]
name = "motif-core"
version = "0.1.0"
edition = "2021"
description = "Pure-Rust core for the Motif practice companion app"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Create an empty library root**

Write `crates/motif-core/src/lib.rs`:

```rust
//! Motif core: pure-Rust data model, scoring function, scheduler,
//! scope evolution, and stall detection. No platform dependencies.
```

- [ ] **Step 3: Verify the workspace builds**

Run: `cargo build -p motif-core`
Expected: `Compiling motif-core v0.1.0 ...` and `Finished` with no errors.

- [ ] **Step 4: Verify tests run (zero tests is fine)**

Run: `cargo test -p motif-core`
Expected: `running 0 tests` and `test result: ok. 0 passed`.

- [ ] **Step 5: Commit**

```bash
git add crates/motif-core/Cargo.toml crates/motif-core/src/lib.rs Cargo.lock
git commit -m "feat(core): scaffold motif-core crate"
```

---

## Task 3: Add ID newtypes

**Files:**
- Create: `crates/motif-core/src/model.rs`
- Modify: `crates/motif-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/motif-core/src/model.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PieceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttemptId(pub String);

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
}
```

- [ ] **Step 2: Wire the module into the library**

Replace `crates/motif-core/src/lib.rs` contents:

```rust
//! Motif core: pure-Rust data model, scoring function, scheduler,
//! scope evolution, and stall detection. No platform dependencies.

pub mod model;
```

- [ ] **Step 3: Run the test, expect it to fail**

Run: `cargo test -p motif-core id_newtypes_roundtrip_serde -- --exact`
Expected: FAIL — the inner `String` is not serialised transparently, so the JSON is `{"0":"abc"}` not `"abc"`.

- [ ] **Step 4: Add `#[serde(transparent)]` to fix**

Edit each of the four newtypes in `crates/motif-core/src/model.rs` to add `#[serde(transparent)]` above the `pub struct` line:

```rust
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
```

- [ ] **Step 5: Run the test, expect PASS**

Run: `cargo test -p motif-core id_newtypes_roundtrip_serde -- --exact`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 6: Commit**

```bash
git add crates/motif-core/src
git commit -m "feat(core): add typed ID newtypes (PieceId, PageId, SegmentId, AttemptId)"
```

---

## Task 4: Add `Rect` type

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs` (before the `#[cfg(test)]` block):

```rust
/// A rectangle on a specific page in page-relative coordinates (0..page_width, 0..page_height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub page_id: PageId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
```

Append to the existing `tests` module:

```rust
    #[test]
    fn rect_roundtrips() {
        let r = Rect { page_id: PageId("p1".into()), x: 1.0, y: 2.0, w: 30.0, h: 40.0 };
        let json = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p motif-core rect_roundtrips -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add Rect type"
```

---

## Task 5: Add `Difficulty` enum with weight and ordering

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs` (before the `#[cfg(test)]` block):

```rust
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
```

Append to the existing `tests` module:

```rust
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
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p motif-core difficulty -- --nocapture`
Expected: 3 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add Difficulty enum with weight, rank, and one_step_lower"
```

---

## Task 6: Add `MemorisationState` enum (v2 hook, present in v1 model)

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs`:

```rust
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
```

Append to `tests`:

```rust
    #[test]
    fn memorisation_state_default_is_none() {
        let m: MemorisationState = Default::default();
        assert_eq!(m, MemorisationState::None);
    }
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core memorisation_state_default_is_none -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add MemorisationState enum"
```

---

## Task 7: Add `PracticeAttempt`

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs`:

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticeAttempt {
    pub id: AttemptId,
    pub segment_id: SegmentId,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: u32,
    pub recording_ref: Option<String>,
    pub self_rating_after: Option<Difficulty>,
}
```

Append to `tests`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core practice_attempt_roundtrips -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs crates/motif-core/Cargo.toml
git commit -m "feat(core): add PracticeAttempt"
```

---

## Task 8: Add `ScopeStage`

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs`:

```rust
/// A historical snapshot of a segment's scope before it was expanded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeStage {
    pub rects: Vec<Rect>,
    pub difficulty_at_promotion: Difficulty,
    pub promoted_at: DateTime<Utc>,
}
```

Append to `tests`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core scope_stage_roundtrips -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add ScopeStage"
```

---

## Task 9: Add `Segment`

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs`:

```rust
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
```

Append to `tests`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core segment_roundtrips -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add Segment type with all fields including v2 hooks"
```

---

## Task 10: Add `Page` and `Piece`

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/model.rs`:

```rust
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
```

Append to `tests`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core piece_roundtrips_with_pages_and_segments -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add Page and Piece types"
```

---

## Task 11: Segment helpers — `last_practiced_at` and `total_seconds_practiced`

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append the new impl block to `crates/motif-core/src/model.rs`, after the `Segment` struct:

```rust
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
}
```

Append to `tests`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core segment_helpers_summarise_practice_history -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add Segment::last_practiced_at and total_seconds_practiced"
```

---

## Task 12: Scheduler — `difficulty_weight` and `staleness_factor` (regular curve)

**Files:**
- Create: `crates/motif-core/src/scheduler.rs`
- Modify: `crates/motif-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/motif-core/src/scheduler.rs`:

```rust
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
```

Modify `crates/motif-core/src/lib.rs` to add the module:

```rust
//! Motif core: pure-Rust data model, scoring function, scheduler,
//! scope evolution, and stall detection. No platform dependencies.

pub mod model;
pub mod scheduler;
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core scheduler::tests`
Expected: 5 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src
git commit -m "feat(core): add difficulty_weight and regular staleness_factor"
```

---

## Task 13: Scheduler — Mastered staleness curve

**Files:**
- Modify: `crates/motif-core/src/scheduler.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/scheduler.rs` (above the `#[cfg(test)]` block):

```rust
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
```

Append to the existing `tests` module in `scheduler.rs`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core mastered_staleness`
Expected: 2 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/scheduler.rs
git commit -m "feat(core): add Mastered staleness curve with 14/30/60 day targets"
```

---

## Task 14: Scheduler — `under_invested_factor`

**Files:**
- Modify: `crates/motif-core/src/scheduler.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/motif-core/src/scheduler.rs` **above the `#[cfg(test)]` block** (function definitions must go in the module body, not inside the tests module):

```rust
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
```

Append to the `tests` module:

```rust
    #[test]
    fn under_invested_boost_decays_with_time() {
        assert_eq!(under_invested_factor(0), 1.5);
        assert_eq!(under_invested_factor(60 * 4), 1.5);
        assert_eq!(under_invested_factor(60 * 10), 1.2);
        assert_eq!(under_invested_factor(60 * 20), 1.1);
        assert_eq!(under_invested_factor(60 * 60), 1.0);
    }
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core under_invested_boost_decays_with_time -- --exact`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/scheduler.rs
git commit -m "feat(core): add under_invested_factor"
```

---

## Task 15: Scheduler — combined `score` function

**Files:**
- Modify: `crates/motif-core/src/scheduler.rs`

- [ ] **Step 1: Write the failing test**

Add the two `use` lines at the **top** of `crates/motif-core/src/scheduler.rs` (immediately after the existing `use crate::model::Difficulty;` line):

```rust
use crate::model::Segment;
use chrono::{DateTime, Utc};
```

Then append the two functions to `crates/motif-core/src/scheduler.rs` **above the `#[cfg(test)]` block**:

```rust
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
```

Append to the `tests` module:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core scheduler::tests`
Expected: all tests in `scheduler::tests` pass (including the earlier ones).

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/scheduler.rs
git commit -m "feat(core): add combined score function and days_since_last_practice"
```

---

## Task 16: Scheduler — `pick_session` with interleaving

**Files:**
- Modify: `crates/motif-core/src/scheduler.rs`

- [ ] **Step 1: Write the failing test**

Add the `use` line at the **top** of `crates/motif-core/src/scheduler.rs` (with the other `use` declarations):

```rust
use crate::model::{PieceId, SegmentId};
```

Then append the function to `crates/motif-core/src/scheduler.rs` **above the `#[cfg(test)]` block**:

```rust
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
```

Append to the `tests` module:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core pick_session`
Expected: 5 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/scheduler.rs
git commit -m "feat(core): add pick_session with cross-piece interleaving"
```

---

## Task 17: Stall detection

**Files:**
- Create: `crates/motif-core/src/stall.rs`
- Modify: `crates/motif-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/motif-core/src/stall.rs`:

```rust
//! Stall detection: a derived signal that surfaces a nudge in the practice view
//! when a segment has been worked many times over multiple weeks with no
//! improvement. Triggers the v1 jumping-off feature (tag → suggestion table).

use crate::model::Segment;
use chrono::{DateTime, Duration, Utc};

/// True iff the segment has at least `attempt_threshold` practice attempts within
/// the last `week_threshold` weeks AND the highest self-rating recorded in that
/// window is no better than the earliest self-rating recorded in that window
/// (i.e. no upward movement toward Mastered).
///
/// Attempts with no self-rating are ignored for the rating-comparison check
/// but still count toward the attempt threshold.
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

    let rated: Vec<_> = recent
        .iter()
        .filter_map(|a| a.self_rating_after)
        .collect();

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
            difficulty: Difficulty::Working,
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
        s.practice_history.push(att("2026-05-30T10:00:00Z", Some(Difficulty::Struggling)));
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
            s.practice_history.push(att(date, Some(Difficulty::Struggling)));
        }
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        assert!(is_stalled(&s, now, 6, 2));
    }

    #[test]
    fn not_stalled_when_upward_rating_change_present() {
        let mut s = seg();
        s.practice_history.push(att("2026-05-18T10:00:00Z", Some(Difficulty::Struggling)));
        s.practice_history.push(att("2026-05-20T10:00:00Z", Some(Difficulty::Struggling)));
        s.practice_history.push(att("2026-05-22T10:00:00Z", Some(Difficulty::Struggling)));
        s.practice_history.push(att("2026-05-24T10:00:00Z", Some(Difficulty::Struggling)));
        s.practice_history.push(att("2026-05-26T10:00:00Z", Some(Difficulty::Working)));
        s.practice_history.push(att("2026-05-28T10:00:00Z", Some(Difficulty::Working)));
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
            s.practice_history.push(att(date, Some(Difficulty::Struggling)));
        }
        let now = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        // Only one attempt within the last 2 weeks → below threshold of 6.
        assert!(!is_stalled(&s, now, 6, 2));
    }
}
```

Modify `crates/motif-core/src/lib.rs`:

```rust
//! Motif core: pure-Rust data model, scoring function, scheduler,
//! scope evolution, and stall detection. No platform dependencies.

pub mod model;
pub mod scheduler;
pub mod stall;
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core stall::tests`
Expected: 5 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src
git commit -m "feat(core): add stall detection"
```

---

## Task 18: Scope evolution — `expand_scope`

**Files:**
- Create: `crates/motif-core/src/scope.rs`
- Modify: `crates/motif-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/motif-core/src/scope.rs`:

```rust
//! Scope evolution: growing a segment outward from a kernel to a larger unit of
//! practice. The previous rects are archived in `scope_history`; the user is
//! prompted to re-rate difficulty (defaulting to one step lower, per the spec).

use crate::model::{Rect, ScopeStage, Segment};
use chrono::{DateTime, Utc};

impl Segment {
    /// Replace the segment's current scope with new rects. The previous rects
    /// become a `ScopeStage` history entry along with the difficulty rating at
    /// promotion. If `drop_difficulty` is true (the spec default), the segment's
    /// difficulty drops one step (e.g. Solid → Working). Otherwise difficulty
    /// stays where it was, which is appropriate when the user has explicitly
    /// rated the larger scope before expanding.
    pub fn expand_scope(
        &mut self,
        new_rects: Vec<Rect>,
        now: DateTime<Utc>,
        drop_difficulty: bool,
    ) {
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
    use crate::model::{
        Difficulty, MemorisationState, PageId, PieceId, SegmentId,
    };
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
            difficulty: Difficulty::Solid,
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
        assert_eq!(s.scope_history[0].difficulty_at_promotion, Difficulty::Solid);
        assert_eq!(s.scope_history[0].promoted_at, now);
        assert_eq!(s.difficulty, Difficulty::Working);
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
        assert_eq!(s.difficulty, Difficulty::Solid);
    }

    #[test]
    fn expand_scope_chains_multiple_stages() {
        let mut s = seg();
        let t1 = Utc.with_ymd_and_hms(2026, 5, 30, 12, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap();
        s.expand_scope(
            vec![Rect { page_id: PageId("p".into()), x: 0.0, y: 0.0, w: 100.0, h: 100.0 }],
            t1,
            true,
        );
        s.expand_scope(
            vec![Rect { page_id: PageId("p".into()), x: 0.0, y: 0.0, w: 300.0, h: 300.0 }],
            t2,
            true,
        );
        assert_eq!(s.scope_history.len(), 2);
        assert_eq!(s.scope_history[0].promoted_at, t1);
        assert_eq!(s.scope_history[1].promoted_at, t2);
        // Started Solid → Working (after first expand) → Struggling (after second)
        assert_eq!(s.difficulty, Difficulty::Struggling);
    }
}
```

Modify `crates/motif-core/src/lib.rs`:

```rust
//! Motif core: pure-Rust data model, scoring function, scheduler,
//! scope evolution, and stall detection. No platform dependencies.

pub mod model;
pub mod scheduler;
pub mod scope;
pub mod stall;
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core scope::tests`
Expected: 3 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src
git commit -m "feat(core): add Segment::expand_scope with history archival"
```

---

## Task 19: Segment practice operations — `record_attempt` and `self_rate`

**Files:**
- Modify: `crates/motif-core/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to the `impl Segment` block in `crates/motif-core/src/model.rs`:

```rust
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
```

Append to the `tests` module in `model.rs`:

```rust
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p motif-core record_attempt_appends_to_history self_rate_updates_difficulty_and_last_attempt self_rate_without_history_still_updates_difficulty`
Expected: 3 passing tests.

- [ ] **Step 3: Commit**

```bash
git add crates/motif-core/src/model.rs
git commit -m "feat(core): add Segment::record_attempt and self_rate"
```

---

## Task 20: End-to-end integration test

**Files:**
- Create: `crates/motif-core/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/motif-core/tests/integration.rs`:

```rust
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
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test -p motif-core --test integration`
Expected: 4 passing tests.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test -p motif-core`
Expected: all unit and integration tests pass. No warnings except possibly unused imports — fix any that appear.

- [ ] **Step 4: Commit**

```bash
git add crates/motif-core/tests/integration.rs
git commit -m "test(core): end-to-end integration tests for scheduler, scope, stall, persistence"
```

---

## Task 21: Add `clippy` and `fmt` checks, fix warnings

**Files:**
- Touch any file with warnings

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -p motif-core --all-targets -- -D warnings`
Expected: may surface warnings (unused imports, suboptimal patterns). Note each one.

- [ ] **Step 2: Fix each warning inline**

For each warning, edit the offending file and resolve it. Do not silence with `#[allow(...)]` unless there's a real reason.

- [ ] **Step 3: Run clippy again**

Run: `cargo clippy -p motif-core --all-targets -- -D warnings`
Expected: clean exit, no warnings.

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt -p motif-core --check`
Expected: either clean exit, or a diff. If a diff, run `cargo fmt -p motif-core` to apply.

- [ ] **Step 5: Run full test suite one more time to confirm nothing broke**

Run: `cargo test -p motif-core`
Expected: all tests pass.

- [ ] **Step 6: Commit any fixes**

```bash
git add -A
git commit -m "chore(core): clippy and fmt clean-up"
```

If there's nothing to commit, skip this step.

---

## Self-Review

After all tasks are complete, the following spec sections are covered:

| Spec section | Covered by |
|---|---|
| §2 data model — Piece, Page, Segment, Rect, Difficulty, MemorisationState, ScopeStage, PracticeAttempt | Tasks 3–10 |
| §2.1 scope evolution with `scope_history` and stage difficulty | Tasks 8, 18 |
| §3 scheduler — `difficulty × staleness × under_invested` | Tasks 12, 13, 14, 15 |
| §3 Mastered floor and re-exposure curve | Task 13 |
| §3 stall detection | Task 17 |
| §3 cross-piece interleaving in session picker | Task 16 |
| §2 segment self-rate updates difficulty + history | Task 19 |
| Persistence (serde JSON roundtrip as the contract) | Tasks 3–10, 20 |

What this plan does **not** cover (correctly — these are future phases):

- Crux App trait, capabilities, effects (Phase 2)
- iOS Swift shell (Phase 2+)
- Capture flow / image storage (Phase 3)
- Segment editor UI and Vision-based automation (Phase 4, Phase 7)
- Practice view, audio recording (Phase 5)
- Heatmap rendering (Phase 6)
- Stalled-segment suggestion library content (Phase 8)
- Onboarding (Phase 9)

The data model has hooks for everything later phases need: `recording_ref` on `PracticeAttempt` so the audio layer can attach files, `image_ref` on `Page` so the capture layer can attach pages, `memorisation_state` and `goal` reserved for v2 features, `scope_history` for the score-map visualisation.

No placeholders, no TODOs, no "similar to Task N" references. Every code block is self-contained and compiles in the file context shown.
