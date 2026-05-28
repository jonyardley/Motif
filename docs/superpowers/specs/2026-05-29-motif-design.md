# Motif — Design

**Date:** 2026-05-29
**Status:** Draft for review
**Target:** Native iOS app, built on Crux (Rust core + Swift shell)

## 1. Positioning

Motif is a **practice companion**, not a score reader and not a performance app. The user already has a way to read music (paper score, PDF, ForScore). Motif's job is to:

1. Help the user break a piece into focused practice **segments**.
2. Show a bird's-eye **heatmap** of progress across the whole piece — where time has been spent, where the hard bits are, what's gone stale.
3. **Schedule** practice so the hardest and most neglected segments rise to the top, eliminating the "play the intro over and over" and "play the easy bits and avoid the hard bits" failure modes.
4. **Force focus** during practice by showing only the current segment, with an optional timer that nudges the user to move on when they're getting obsessed with a single passage.

Out of scope for v1: OMR/auto-segmentation, performance-mode page turner, multi-instrument-specific features, social/sharing, memorisation drills, adaptive exercise suggestions. All of these are noted as future and have data-model hooks where appropriate.

## 2. Core concepts and data model

### Piece

A piece is what the user is learning. It owns its pages and its segments.

```
Piece {
  id
  title
  composer
  created_at
  pages: [Page]         // ordered
  segments: [Segment]
}
```

### Page

A page is a single image (photo or PDF page). Pages are owned by a piece and ordered.

```
Page {
  id
  index           // 0-based order within the piece
  image_ref       // opaque handle; Swift shell resolves to a file
  width, height   // in pixels, for coordinate normalisation
}
```

### Segment

A segment is a **practice unit**. It is a **mask** — one or more axis-aligned rectangles on one or more pages — together with practice state.

```
Segment {
  id
  piece_id
  label              // optional human name, e.g. "Largo opening"
  rects: [Rect]      // each rect is { page_id, x, y, w, h } in page-relative coords
  difficulty         // Struggling | Working | Solid | Mastered
  tags: [String]     // e.g. "octaves", "trill", "fingering", "rhythm"
  notes              // free text
  created_at
  practice_history: [PracticeAttempt]

  // v2/v3 hooks (present but unused in v1):
  memorisation_state // None | Learning | Memorised | Verified
}
```

Notes on the mask model:

- **Rectangles, not freeform polygons.** Rectangle unions handle every case observed in scoping (cross-stave, cross-system, cross-page, overlapping, key-sig context strip). Freeform lasso is overkill on a touchscreen and dramatically complicates the editor.
- **Cross-page segments** are allowed (a segment's rects may live on different pages). Rare; the model costs nothing to support.
- **Overlapping segments** are allowed. Segments are independent objects whose rects may happen to intersect.

### PracticeAttempt

Each time the user practices a segment, an attempt is recorded.

```
PracticeAttempt {
  id
  segment_id
  started_at
  duration_seconds
  recording_ref        // optional; opaque handle to audio file
  self_rating_after    // optional: Struggling | Working | Solid | Mastered
                       // — updates the segment's difficulty when present
}
```

All attempts are retained by default so the user can scroll their audio history per segment over weeks/months.

## 3. The scheduler

A single scoring function drives both the heatmap visualisation and the guided session queue. They must share the same scoring so what the user *sees* matches what the app *picks*.

```
score(segment, now) =
    difficulty_weight(segment.difficulty)
  × staleness_factor(now - segment.last_practiced_at)
  × under_invested_factor(total_time_spent(segment))
```

- `difficulty_weight`: Struggling 4, Working 3, Solid 2, Mastered 1.
- `staleness_factor`: monotonically increasing in days-since-last-practice, with a soft floor of 1 (a segment practiced today still has some score) and a cap to prevent ancient untouched segments from dominating forever.
- `under_invested_factor`: a small boost for segments with low total practice time, so the scheduler doesn't keep cycling the same five segments. Decays toward 1 once meaningful time has been spent.

The exact curves are tunable constants; the design only requires the *shape*. v1 ships sensible defaults; we may expose them in settings later.

### Guided session

The default flow when the user taps **Practice now**:

1. Scheduler picks the top N segments by score (default N = 5, configurable).
2. User is walked through them in order: masked segment shown, optional metronome/count-in, optional record button, optional focus timer (default 5 min).
3. When the user moves on (or the timer nudges them and they accept), they get an optional self-rating prompt: *"Still hard / Getting easier / Solid / Mastered"*. Choosing one updates `segment.difficulty`. Skipping is fine.
4. Session summary at the end: which segments worked, total time, difficulty changes.

### Free browse

A parallel flow. From the heatmap / score map, the user can tap any segment and practice it directly. Same practice UI, same recording and self-rating loop. The score is updated regardless of how the segment was reached.

## 4. Practice view

The practice view is the most opinionated screen in the app. Its design is shaped by the "force focus" goal:

- **Isolated, not in-context.** The view shows the segment alone — the rectangles of the mask, composited tightly, on a black background. The rest of the page is not visible. The user *cannot* drift into the next bar by reading ahead, because there is no next bar visible.
- **Big.** The mask is scaled to fill the screen.
- **Recording controls:** one tap to start, auto-stop on silence (configurable threshold) or manual stop. The take is saved with a timestamp.
- **Playback:** the most recent take is one tap away; older takes available via a per-segment history panel.
- **Focus timer (optional, default on at 5 min):** counts up while the segment is active. On expiry, a gentle non-blocking nudge: *"You've been on this for 5 minutes. One more pass, or move on?"* The user can extend, or accept the nudge and move to the next segment in the queue.
- **No score navigation.** No swipe-to-next-page, no jump-to-bar. This is not a score reader.

## 5. The heatmap (score map)

The other opinionated screen. This is the planning and motivation view.

- **Layout:** the whole piece rendered as a long scrollable strip (or grid) of page thumbnails, with each segment shown as its actual mask region overlaid on the real notes. The user recognises "the bit with the scary chromatic run" by sight, not by abstract symbols.
- **Colour axis (default): mastery.** Struggling red → Working amber → Solid green → Mastered deep green.
- **Opacity / desaturation: staleness.** A Solid segment untouched for three weeks fades toward grey-green — visual cue that "you nailed this once, but is it still there?"
- **Subtle density indicator: time invested.** Small dots, a thickness, or similar showing total practice minutes. Lets the user spot "I've poured an hour into this and it's still red" and "I've barely touched this and it's already green."
- **Toggle overlays:** chips at the top *promote* a secondary axis to be the primary colour. In the default view, mastery is the colour and staleness rides on opacity; switching to "Staleness" makes staleness the colour (red = haven't touched in weeks) and drops the opacity encoding. Same logic for Time invested and Recent trend (improving / flat / regressing). Same layout, four lenses on the same underlying data.
- **Untagged regions** (areas of the page with no segment drawn) display as the plain score — natural contrast between practiced material and the rest.

Tapping a segment opens its detail (notes, tags, audio history, difficulty, "Practice now").

## 6. Capture flow

Adding a piece supports two paths into the same internal model:

- **PDF import:** user picks a PDF; each page becomes a `Page`.
- **Multi-page camera flow:** guided "Page 1… next… Page 2… next…" capture. Pages are stored as images.

After capture, the user enters the segment editor for the new piece. Page management (delete, reorder, replace) is available but minimal.

## 7. Segment editor

- View a page.
- Draw a rectangle by drag.
- Add another rectangle to the same segment (for multi-rect segments — cross-system phrases, key-sig context strips).
- Edit rectangle bounds (drag handles).
- Set difficulty, tags, notes, label.
- Delete segment.

## 8. Architecture (Crux split)

This is the natural fit for Crux. The Rust core is pure, deterministic, and owns all reasoning about state. The Swift shell handles everything that requires platform APIs.

### Rust core owns

- The full data model: Piece, Page (as ids + image refs), Segment, PracticeAttempt.
- The scheduler scoring function and session queue selection.
- The heatmap scoring (same function as the scheduler).
- Self-rating updates and history mutations.
- Capabilities (Crux side-effect requests): `KeyValue` for persistence, `Render` for view models, `Time` for timestamps, plus a small custom capability for requesting audio recording and audio playback.

The core never reads a file, never sees a pixel, never opens an audio stream. It emits effects describing *what* it wants done, and processes the results.

### Swift shell owns

- Camera capture, PDF import, image file storage.
- Audio recording (AVAudioEngine) and playback.
- Rendering: masking the page image to draw segments, drawing the heatmap, drawing the practice view.
- CloudKit sync (v2).
- Local persistence: serialising the core's KeyValue requests to disk.

### Why this split works

- All "interesting" logic (scheduler, scoring, mastery updates, history reasoning) is testable in Rust with zero iOS dependencies.
- The Swift shell is a thin renderer + I/O layer that can be replaced (Android, macOS) without touching the model.
- The heatmap and the practice queue share scoring by construction — they call the same function in the core.

## 9. Storage and sync

- **v1:** local-only. Crux core requests `KeyValue` puts/gets; Swift shell persists to disk (app container). Audio and image files stored alongside; the core holds opaque refs.
- **v2:** CloudKit sync for multi-device. The data model is already serialisable Rust structs; the shell handles the sync mechanics.

No server, no account, no backend in v1.

## 10. Tagging and notes

- Tags are free-form strings, but the app suggests a curated set (`octaves`, `trill`, `leaps`, `chromatic`, `polyrhythm`, `voicing`, `fingering`, `rhythm`, `pedalling`, `memorise`).
- Tags are filterable in the heatmap ("show only segments tagged `octaves`").
- Notes are free text per segment.

## 11. Future features (not in v1, but model-ready)

### Memorisation (v2)

Per-segment `memorisation_state` field already in the model. Adds a Memory Check practice mode: segment shown briefly, blanked, user plays from memory, self-rates *got it / hesitated / lost it*. Scheduler gets a second axis: Memorised segments earn periodic memory-check visits even when their difficulty is Solid. A "random start" drill picks a random bar within the segment.

### Adaptive exercise suggestions (v2/v3)

Driven by the existing tag system, not OMR. A curated tag → exercise-suggestion library: when a segment has been Struggling for >N sessions and is tagged `octaves`, surface a card suggesting hands-separate slow practice and a relevant etude. Pure content curation, no ML.

### OMR auto-segmentation (v3+)

Pre-populate segments from a recognised score. The mask model already supports the output (rectangles per phrase), so OMR just changes the *input* to the segment editor. The rest of the app is unchanged.

### Concert prep mode (deferred indefinitely)

Considered and rejected for v1 — pulled the product toward performance, away from practice. The natural pre-concert behaviour falls out of the existing heatmap and scheduler.

## 12. Open questions for review

None blocking. The following are tunable constants that v1 will ship with sensible defaults and may expose later:

- Focus timer default (5 min).
- Session size N (5 segments).
- Auto-stop-on-silence threshold.
- Scoring function constants (difficulty weights, staleness curve, under-invested boost shape).
