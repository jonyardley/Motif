# CLAUDE.md

Guidance for working in the **Motif** repository.

## What Motif is

Motif is a **practice companion** for self-directed adult instrumentalists (pianists
first). It is **not** a score reader, a performance app, a metronome/tuner, or a
gamification layer. It sits next to the user's existing score and answers one question:
*how should I spend the next hour of practice?*

It does this by breaking a piece into small **segments** (masked rectangles on page
images), scheduling the hardest/stalest/most-neglected segments to the top, forcing
focus on one segment at a time, recording per-segment audio history, and surfacing a
heatmap of the whole piece.

Two opinionated principles drive every default (see `docs/.../motif-design.md` §1a/§1b):

- **Tortoise, not hare** — bias toward slow, locked-in work on small chunks before
  larger ones. The scheduler weights toward Struggling/stale segments; the practice
  view shows one segment full-screen with everything else blacked out.
- **Low-overhead, magic-minimum-taps** — segment setup should be 2–3 taps and no
  typing, using on-device Vision + classical CV (never deep-learning OMR in v1).

Read the specs before making design decisions:

- `docs/superpowers/specs/2026-05-29-motif-vision.md` — product vision, who it's for, roadmap (v1/v2/v3).
- `docs/superpowers/specs/2026-05-29-motif-design.md` — **the authoritative design spec**: data model, scheduler, practice view, heatmap, capture/editor, Crux architecture. Section numbers (§2, §3, §7.3, §13…) are referenced throughout the code and plans.
- `docs/superpowers/plans/2026-05-29-motif-core-foundations.md` — the task-by-task implementation plan for the current phase (Phase 1: `motif-core`).

## Architecture

Target is a **native iOS app built on [Crux](https://github.com/redbadger/crux)**: a pure
Rust core + a thin Swift shell.

- **Rust core (`motif-core`)** owns the entire data model, the scheduler/scoring
  function, heatmap scoring (same function as the scheduler — by construction what the
  user *sees* matches what the app *picks*), self-rating/history mutations, scope
  evolution, and stall detection. It is **pure and deterministic**: it never reads a
  file, sees a pixel, opens an audio stream, or reads the wall clock.
- **Swift shell** (not in this repo yet) owns camera/PDF capture, image+audio file
  storage, audio recording/playback, rendering (masking, heatmap, practice view),
  persistence, and CloudKit sync (v2).

The current repository is **Phase 1 only**: the pure-Rust core. No Crux `App`/Capability
traits, no Swift, no image/audio/Vision code yet — those are deferred to later phases.

## Repository layout

```
Cargo.toml                       # workspace root (resolver = "2")
rust-toolchain.toml              # pinned toolchain (single source of truth)
clippy.toml                      # core-purity lint bans
.github/workflows/ci.yml         # fmt + clippy + test on every PR
crates/
  motif-core/                    # the only crate; pure Rust, no platform deps
    Cargo.toml
    src/
      lib.rs                     # public re-exports / module declarations
      model.rs                   # data model + ID newtypes + Segment methods
      scheduler.rs               # scoring curves, score(), pick_session()
      stall.rs                   # is_stalled()
      scope.rs                   # Segment::expand_scope()
    tests/
      integration.rs             # end-to-end behaviour test
docs/superpowers/
  specs/                         # vision + design
  plans/                         # implementation plans
```

## Tech stack & conventions

- **Rust, edition 2021.** Cargo workspace, single crate `motif-core` v0.1.0.
- Dependencies: `serde` + `serde_json` (serialisation), `chrono` (time, `serde` feature).
  `uuid` is a **dev-dependency only** (tests).
- **Determinism is a hard rule.** Every function that depends on the current time takes
  `now: DateTime<Utc>` as an explicit argument. No `Utc::now()` in the core (enforced by
  `clippy.toml`).
- **The core never generates IDs.** IDs are `String` newtypes (`PieceId`, `PageId`,
  `SegmentId`, `AttemptId`) passed in by the shell. Newtypes use
  `#[serde(transparent)]` so they serialise as bare strings.
- All model types derive `Serialize`/`Deserialize` and roundtrip through JSON.
- v2/v3 fields (`memorisation_state`, `goal`, `scope_history`) live in the v1 model as
  hooks but are unused by the v1 scheduler. Use `#[serde(default)]` for fields added as
  forward-compat hooks.

### Core domain concepts (from design §2–§3)

- **Piece** owns ordered `Page`s and `Segment`s.
- **Segment** = a practice unit: a *mask* of one or more `Rect`s (page-relative coords)
  plus practice state (`difficulty`, `tags`, caption metadata, `practice_history`,
  `scope_history`).
- **Difficulty**: `Struggling | Working | Solid | Mastered`. Scheduler weights are
  4 / 3 / 2 / **1 (floor — never 0)**, so Mastered stays in long-interval rotation.
- **Scoring**: `score = difficulty_weight × staleness_factor × under_invested_factor`.
  Mastered segments use a separate staleness curve targeting ~14/30/60-day re-exposure.
- **pick_session** applies cross-piece interleaving (avoid consecutive segments from the
  same piece when a score-equivalent alternative exists).
- **Scope evolution** (`expand_scope`): growing a segment's kernel outward archives the
  old rects into `scope_history` and (by default) drops difficulty one step.
- **Stall detection** (`is_stalled`): ≥N attempts over ≥M weeks with no upward rating
  change → triggers the v1 "jumping-off point" exercise-suggestion card.

When tuning scheduler curves, the **shape** is what the design fixes; exact constants are
tunable defaults (design §12). Keep heatmap and scheduler sharing the same `score`.

## Development workflow

This project follows **TDD via the superpowers plan**. Each task in the foundations plan
is: write a failing test → run it (confirm it fails for the stated reason) → implement →
confirm it passes → commit. Follow the plan's task order and exact test cases unless you
have a reason to deviate.

Commands (run from the repo root):

```bash
cargo build -p motif-core            # build the core
cargo test  -p motif-core            # run all tests
cargo test  -p motif-core <name> -- --exact   # run a single test by name
cargo test  -p motif-core scheduler::tests    # run a module's tests
cargo fmt                            # format
cargo clippy -p motif-core           # lint
```

### Branch, PR & review conventions

All changes go through a pull request — **never commit directly to `main`**.

1. **Branch.** Start every piece of work on a fresh branch off `main`. Name it by
   scope and intent, e.g. `feat/scheduler-interleaving`, `fix/staleness-cap`,
   `chore/ci-setup`.
2. **Build green before opening.** `cargo build`, `cargo test`, `cargo fmt --check`,
   and `cargo clippy` must all pass locally before a PR goes up.
3. **Open a PR** with a description of what changed and why, and which design-spec
   section(s) or plan task(s) it implements (e.g. "implements §3 stall detection").
4. **Review before merge.** Every PR gets a review — request `/code-review` (or have a
   reviewer look it over) and address findings before merging. Don't self-merge
   unreviewed work.
5. **Keep PRs small.** Prefer one plan task (or a tightly related group) per PR so the
   diff stays reviewable. Squash-merge to keep `main` history linear and one-commit-
   per-task.

### Quality gates (CI)

Correctness is enforced mechanically, not by convention alone. CI
(`.github/workflows/ci.yml`) runs on every PR and push to `main` and **must be green
to merge**:

- `cargo fmt --all -- --check` — formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` — lint; **warnings are errors**.
- `cargo test --workspace` (unit + integration) and `cargo test --workspace --doc`.

Two project-specific guards back the "pure, deterministic core" rule:

- **`rust-toolchain.toml`** pins the compiler + `rustfmt`/`clippy` and is the single
  source of truth — CI installs from it (`rustup show`), so local and CI builds can't
  drift. Bump it in its own PR.
- **`clippy.toml`** bans the methods/types that would break determinism or do I/O in the
  core (`Utc::now`, `SystemTime::now`, `std::fs::File`, …). A determinism violation fails
  clippy instead of slipping through review. If the core ever legitimately needs one,
  add a narrowly-scoped `#[allow(...)]` with a comment — don't loosen the global ban.

Run the same gates locally before opening a PR: `cargo fmt --all -- --check && cargo
clippy --all-targets -- -D warnings && cargo test`.

### Commit conventions

Conventional Commits, scoped to the area touched:

- `feat(core): add Difficulty enum with weight, rank, and one_step_lower`
- `chore: initialise cargo workspace`

Commit each completed plan task separately, including `Cargo.lock` when dependencies
change. Only commit/push when asked.

## Guardrails

- **Keep the core pure.** No filesystem, network, audio, image, or wall-clock access.
  If logic needs the time, take `now` as a parameter.
- **Don't pull v2/v3 features forward.** OMR, glyph recognition, CloudKit, mental-practice
  mode, etc. are explicitly out of scope for v1 (design §11). Add model hooks, not
  implementations.
- **Honest tools, no gamification.** No streaks/badges/leaderboards — this is a product
  values stance (vision §"Five principles"), not just a missing feature.
- When in doubt about behaviour, the **design spec is authoritative**; cite the section.
