# Motif — Phase Roadmap

This document lists the v1 implementation phases. Each phase is its own spec→plan→implementation cycle producing working, testable software. Phases are sequenced because each builds on the previous, but each phase's plan should be written *after* the prior phase has landed so it can incorporate what we learned.

Cross-references: see `specs/2026-05-29-motif-design.md` for the full design and `specs/2026-05-29-motif-vision.md` for the why.

---

## Phase 1 — Pure-Rust core ✅ shipped

Crate: `crates/motif-core/`.

Data model (Piece, Page, Segment with practice ops, Rect, Difficulty with auto-assigned Fresh, MemorisationState, PracticeAttempt, ScopeStage), scoring function with cross-piece interleaving, stall detection, scope evolution, end-to-end behavioural tests. No Crux scaffolding, no iOS, no platform code. Pure functions; every time-dependent function takes `now: DateTime<Utc>` as an explicit argument.

48 tests pass; clippy and rustfmt clean in CI.

Plan: `plans/2026-05-29-motif-core-foundations.md`.

---

## Phase 2 — Crux scaffold + minimal iOS shell

Wrap `motif-core` in a Crux App. Wire up the Crux capabilities the design depends on: `KeyValue` (persistence), `Render` (view-model updates), `Time` (timestamps), and a small custom capability for audio recording and playback. Build the iOS skeleton in Swift that connects to the Rust core, persists the app state to disk via the KeyValue capability, and renders a placeholder view that proves the effect plumbing works end-to-end.

**In scope:** Crux App trait implementation, Effect/Event enum design, capability wiring, iOS shell skeleton, persistence round-trip, a single "hello world" view rendered from the core's view model.

**Out of scope:** Any of the user-facing features (capture, segment editor, practice view, heatmap). Phase 2 proves the architecture works; the features follow in Phases 3-7.

---

## Phase 3 — Capture flow

PDF import and multi-page camera capture. User can add a piece, capture or import its pages, and see them in a Library list. Page images stored to disk via the shell; the core holds opaque `image_ref` strings.

**In scope:** PDF picker, AVFoundation camera flow, `VNDetectDocumentSegmentationRequest` for page rectification, page persistence, Library list view.

**Out of scope:** Segment creation (Phase 4), any image-analysis beyond rectification.

---

## Phase 4 — Segment editor (manual primitives)

Draw rectangles by drag, multi-rect segments, edit handles, set difficulty / tags / notes / label, fill in tempo / dynamic / expression caption metadata, delete. Scope expansion as an explicit user action.

**In scope:** All manual primitives from spec §7.1. Heatmap stub showing segments coloured by current Difficulty (without the time/staleness lenses yet — those land in Phase 6).

**Out of scope:** The Vision-based automation from spec §7.2 — those land in Phase 7. Phase 4 ships the editor that the automation will later augment.

---

## Phase 5 — Practice view + audio

Strict-isolation practice view (segment alone on black background, caption strip above), focus timer with nudge-to-move-on, audio recording with auto-stop on silence, per-segment recording history with playback, self-rating chips. Guided session flow (the app picks N segments by score, walks the user through them, summary at the end). The graduation prompt at first-Confident rating (context pass + expand-scope options).

**In scope:** All of spec §4 except §4.3 (stalled-segment nudge card content — that's Phase 8).

**Out of scope:** The exercise-suggestion content library (Phase 8). Phase 5 ships the practice loop with a placeholder when stalled detection fires; the real content lands later.

---

## Phase 6 — Heatmap (score map view)

Bird's-eye view of the whole piece. Pages rendered as a scrollable strip with segment masks overlaid on the score, coloured by the scoring function. Chip toggles for the four lenses (Mastery / Staleness / Time / Trend). Tag filter. Tap-to-practice from any segment.

This is also when **spec §3 rule 2** (within-piece adjacency-aware shuffling) becomes implementable, because rect geometry is finally first-class in the UI layer. The `pick_session` interleaver should be extended at this point.

**In scope:** All of spec §5. Adjacency-aware shuffling in `pick_session`.

**Out of scope:** Any new visualisation modes (scope-evolution outlines, concert prep view) — explicitly deferred to v2.

---

## Phase 7 — Vision-based automation

Plug Apple's Vision framework into the segment editor. System detection via horizontal projection profile; bar-line detection via vertical projection; auto-suggested context strip rect on segment creation; OCR for tempo / dynamic / expression markings via `VNRecognizeTextRequest` with an Italian musical-vocabulary hint; snap-to-bar on drawn rect edges; expand-scope candidate proposals.

Every suggestion is one-tap-confirm over the manual primitives from Phase 4. Detection accuracy of ~70% is already valuable; ~90% feels like magic.

**In scope:** Everything in spec §7.2 and §7.3. Stays on-device; no ML model training, no model downloads, no server-side OMR.

**Out of scope:** Glyph recognition (clef / key sig / time sig as semantic data) — that's a v2 ML feature noted in spec §11. Full OMR (notes, pitches, MusicXML) is v3+.

---

## Phase 8 — Stalled-segment suggestion content library

Hardcoded tag → suggestion table for the v1 stalled-segment nudge card (spec §4.3). Initial content covers the common tags: octaves, trill, leaps, chromatic, polyrhythm, voicing, fingering, rhythm, pedalling, plus generic fallbacks (slow practice, hands separate for piano, isolate the hardest beat).

Wires the suggestion display into the practice view shipped in Phase 5.

**In scope:** Content writing, tag→suggestion lookup logic, the nudge card UI surfaced when `is_stalled` fires.

**Out of scope:** Adaptive ML-driven suggestions, embedded score snippets, the curated etude library — all v2.

---

## Phase 9 — Onboarding polish

First-run experience: add-your-first-piece prompt, capture walkthrough, segment editor tutorial overlay on first use, practice view first-time hints. The minimum to get a never-before-user from install to a focused, recorded practice attempt with a meaningful heatmap.

**In scope:** Empty-state flows, one-time-shown hints, error recovery (what if Vision fails on the user's first photo?), accessibility audit pass.

**Out of scope:** Tutorial videos, interactive onboarding content, anything requiring back-end services. The first-run experience must work entirely offline like the rest of the v1 product.

---

## After v1

v2 and v3 features are captured in spec §11. The data model already has hooks for all of them (`memorisation_state`, `goal`, scope evolution data) so adopting them is a UI + content effort, not a re-architecture.

The behavioural success signals (vision doc §"How we'll know it's working") apply to v1 in aggregate, not to any individual phase. Phase-level success is just "the feature works as specified and the test suite stays green in CI."
