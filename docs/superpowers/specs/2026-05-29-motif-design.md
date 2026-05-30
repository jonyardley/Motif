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
5. **Act as a jumping-off point** when the user is stuck — detect stalled segments and surface targeted exercise suggestions, so practicing this app prompts richer practice off-app.

Out of scope for v1: OMR/auto-segmentation, performance-mode page turner, multi-instrument-specific features, social/sharing, memorisation drills, curated exercise content library, mental-practice mode, tempo gradation tools, performance-rehearsal pressure simulation. All have data-model hooks where appropriate.

## 1a. Philosophy: tortoise, not hare

The product has an opinionated philosophy that shapes every default: **you make faster overall progress by refusing to run before you can walk.** The common failure mode in self-directed instrumental practice is playing the whole piece at near-tempo from day one, getting through the easy bits, stumbling at the hard bits, and reinforcing exactly the wrong motor patterns. Motif's defaults all push in the opposite direction:

- The guided session picks segments, not whole pieces.
- The practice view shows one segment, full-screen, with the rest of the page blacked out — you literally cannot drift into the next bar.
- The scheduler weights toward Struggling and stale segments, biasing time away from the bits that already feel comfortable.
- Audio is recorded per-segment so the user can hear *progress on this fragment* across weeks — feedback that's invisible when you only ever play the whole piece.
- The "context pass" (see §4) is offered only once a segment is Solid, so wider musical context is re-introduced *after* the bit is locked, not before.

This is a values stance, not a neutral piece of UX. The honest research caveat is that this stance is more strongly supported by pedagogical tradition (Neuhaus, Whiteside, generations of teachers) than by controlled experimental studies — see §13.

## 1b. Philosophy: low-overhead, magic-minimum-taps

Equally opinionated: **the app must not feel like admin.** Every segment setup step that requires typing or precise touch input is a tax on actually practicing. The product target is: photograph a page, draw boxes by drag, get usable segments with key/time/tempo/dynamic context attached, with as few taps as possible. Manual override is always available but rarely needed. See §6 (capture) and §7 (segment editor) for the v1 automation; the honest scope of what's automatable on-device without a trained OMR model is in §11.

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
  rects: [Rect]      // current scope: each rect is { page_id, x, y, w, h } in page-relative coords
  difficulty         // Struggling | Working | Solid | Mastered
  tags: [String]     // e.g. "octaves", "trill", "fingering", "rhythm"
  notes              // free text

  // Musical-context metadata (displayed as a caption strip during practice,
  // so info that lives elsewhere on the page is still present without
  // needing the user to draw extra rects across distant parts of the score)
  tempo_marking      // optional, e.g. "Largo", "Moderato"
  dynamic_marking    // optional, e.g. "f pesante", "p"
  expression_note    // optional, e.g. "dim.", "agitato"

  // Scope evolution — see §2.1
  scope_history: [ScopeStage]

  created_at
  practice_history: [PracticeAttempt]

  // v2/v3 hooks (present but unused in v1):
  memorisation_state // None | Learning | Memorised | Verified
  goal               // optional free-text micro-goal, e.g. "hands together at 80bpm no stops"
}

ScopeStage {
  rects: [Rect]                  // what the scope was at this stage
  difficulty_at_promotion        // what the user rated it before expanding
  promoted_at                    // when this stage ended and the next began
}
```

Notes on the mask model:

- **Rectangles, not freeform polygons.** Rectangle unions handle every case observed in scoping (cross-stave, cross-system, cross-page, overlapping, key-sig context strip). Freeform lasso is overkill on a touchscreen and dramatically complicates the editor.
- **Cross-page segments** are allowed (a segment's rects may live on different pages). Rare; the model costs nothing to support.
- **Overlapping segments** are allowed. Segments are independent objects whose rects may happen to intersect.

### 2.1 Segment scope evolution

A segment is not fixed in size for its lifetime. A common and pedagogically valid practice pattern is:

1. Start with a tiny scope — a single beat with a complex figure, a single bar with an awkward leap.
2. Practice it until it's Solid.
3. **Expand** the scope outward — to two beats, to a bar, to two bars, to a phrase.
4. Repeat.

This is "chunking up": the kernel is locked, then it's re-contextualised by enlarging the unit of practice. The data model represents this explicitly:

- The Segment's `rects` field is its **current scope**.
- `scope_history` records every previous scope state, the difficulty rating at the moment of promotion, and the timestamp.
- The Segment's identity is stable across scope changes — practice_history travels with it, so the user sees the chunking-up story per segment: "started as beat 3, became bar 1, became bars 1-2; took 12 / 8 / 5 sessions per stage."
- On expansion, the user is prompted to re-rate difficulty (default: drop one step — Solid → Working). The previous rating is preserved in the `ScopeStage`.

The heatmap can visualise scope evolution as concentric outlines (defer to v2 if visually noisy) — the "kernel" you started from is still visible inside the current envelope.

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

- `difficulty_weight`: Struggling 4, Working 3, Solid 2, **Mastered 1 (floor — never 0)**. Mastered segments stay in long-interval rotation per the overlearning literature; they are never zeroed out of consideration.
- `staleness_factor`: monotonically increasing in days-since-last-practice, with a soft floor of 1 (a segment practiced today still has some score) and a cap to prevent ancient untouched segments from dominating forever. For Mastered segments specifically, the curve targets re-exposure at roughly 14 / 30 / 60-day intervals.
- `under_invested_factor`: a small boost for segments with low total practice time, so the scheduler doesn't keep cycling the same five segments. Decays toward 1 once meaningful time has been spent.

The exact curves are tunable constants; the design only requires the *shape*. v1 ships sensible defaults; we may expose them in settings later.

### Interleaving (default on)

The scheduler does not produce serial runs of segments from the same passage or piece. Two rules:

1. **Across pieces:** when the user has more than one active piece, the queue mixes them — a session is typically not 5 segments from one piece but 3 + 2, 2 + 2 + 1, etc., subject to the top-scoring segments.
2. **Within a piece:** when consecutive top-scoring segments are spatially adjacent on the page, the queue shuffles non-adjacent segments between them where score-equivalent options exist.

This is grounded in the contextual-interference literature (Shea & Morgan 1979; Carter & Grahn 2016): blocked single-passage practice feels better in the moment but yields worse retention. The behaviour is configurable — power users can switch to blocked.

### Stall detection

The scheduler maintains a derived signal `stalled` on each segment, true when:

- The segment has ≥ N practice attempts (default 6) over ≥ M weeks (default 2), **and**
- Its difficulty has not improved in that window.

Stall is a trigger, not a scoring input — it surfaces a card (see §4.3) suggesting targeted exercises drawn from a hard-coded tag → suggestion table. This is the v1 form of the "jumping-off point" feature.

### Guided session

The default flow when the user taps **Practice now**:

1. Scheduler picks the top N segments by score (default N = 5, configurable), with interleaving rules applied.
2. User is walked through them in order: masked segment shown, optional metronome/count-in, optional record button, optional focus timer (default 5 min).
3. When the user moves on (or the timer nudges them and they accept), they get an optional self-rating prompt: *"Still hard / Getting easier / Solid / Mastered"*. Choosing one updates `segment.difficulty`. Skipping is fine.
4. Session summary at the end: which segments worked, total time, difficulty changes.

### Overrides (scaffolding fade)

From day one the user can override the scheduler — build a custom session, pin specific segments, exclude a piece for today, switch interleaving off. This addresses the expertise-reversal concern (Kalyuga et al. 2003): heavy prescription helps beginners and hurts advanced users.

### Free browse

A parallel flow. From the heatmap / score map, the user can tap any segment and practice it directly. Same practice UI, same recording and self-rating loop. The score is updated regardless of how the segment was reached.

## 4. Practice view

The practice view is the most opinionated screen in the app. Its design is shaped by the "force focus" goal.

### 4.1 Strict isolation

- **Isolated, not in-context.** The view shows the segment alone — the rectangles of the mask, composited tightly, on a black background. The rest of the page is not visible. The user *cannot* drift into the next bar by reading ahead, because there is no next bar visible.
- **Big.** The mask is scaled to fill the screen.
- **Recording controls:** one tap to start, auto-stop on silence (configurable threshold) or manual stop. The take is saved with a timestamp.
- **Playback:** the most recent take is one tap away; older takes available via a per-segment history panel.
- **Focus timer (optional, default on at 5 min):** counts up while the segment is active. On expiry, a gentle non-blocking nudge: *"You've been on this for 5 minutes. One more pass, or move on?"* The user can extend, or accept the nudge and move to the next segment in the queue.
- **No score navigation.** No swipe-to-next-page, no jump-to-bar. This is not a score reader.

### 4.2 Musical context without breaking isolation

Strict isolation creates a real problem: key signatures, time signatures, tempo markings, and dynamics often live elsewhere on the page. Two-part solution:

- **Auto-suggested context strip.** When the user draws a segment in the editor, the app proposes a small additional rect at the start of the system covering the clef, key sig, and time sig. One tap to accept; user can adjust or skip. Uses the existing multi-rect mask model — no new structural concept.
- **Caption strip.** A thin row *above* the masked segment shows the segment's metadata: `tempo_marking · dynamic_marking · expression_note` (e.g. "Largo · f pesante · dim."). These are typed in once during editing; they appear during every practice attempt. Decouples musical information from spatial position on the page — useful when the relevant marking is 14 inches away from the segment.

Between the auto-context-strip rect and the caption strip, the segment-in-isolation view carries enough musical instruction to be practiced honestly without needing to flip back to the score.

### 4.3 Stalled-segment nudge card

When the segment's `stalled` signal is true (§3 Stall detection), the practice view shows a dismissible card on entry:

> *"You've worked this segment 8 times over 3 weeks with no change. Try one of these:"*

…followed by 2-3 short suggestions drawn from a hard-coded `tag → suggestions` table. For tag `octaves`: "Hands separate, half tempo, watch wrist height." For `trill`: "Slow trill from each note, gradually increase speed." For untagged segments: generic fallbacks (slow practice, hands separate for piano, isolate the hardest beat). The card links to a longer "Suggested exercises" view with more detail.

The card is the v1 form of the "jumping-off point" — explicitly designed so the app is not a closed loop but a prompt back into richer off-app practice.

### 4.4 Graduation prompt — context pass and scope expansion

When the user rates a segment **Solid** for the first time, the app offers two complementary actions, either, both, or neither of which the user can choose:

1. **Context pass.** A one-shot practice mode that shows the segment plus surrounding bars (no mask), inviting the user to play the segment back inside its musical neighbourhood. Addresses the phrase-level encoding concern at exactly the moment it becomes relevant — once the bit is locked, re-embed it in the piece.
2. **Expand scope.** Grow this segment outward to include adjacent material. The app proposes a natural candidate expansion using bar detection (next beat, next bar, next phrase). On accept, the previous rects become a `ScopeStage` history entry; the user re-rates difficulty (defaulting to one step back).

These map to the two pedagogically valid moves at this point: re-contextualise within the larger phrase, or chunk up to a larger unit of practice. Both are first-class — the design does not force a choice.

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

The editor is the place where the "low-overhead, magic-minimum-taps" principle (§1b) lives or dies. Every step that requires manual precision or typing has a tap-saving suggestion sitting underneath.

### 7.1 Manual primitives (always available)

- View a page.
- Draw a rectangle by drag.
- Add another rectangle to the same segment (for multi-rect segments — cross-system phrases, key-sig context strips).
- Edit rectangle bounds (drag handles).
- Set difficulty, tags, notes, label.
- Fill in optional tempo marking, dynamic, expression note.
- **Expand scope:** add rects to grow the segment outward (records a `ScopeStage` history entry).
- Delete segment.

### 7.2 Automation (suggestion + one-tap-confirm)

All v1 automation runs on-device using Vision and classical image processing. No model training, no cloud.

- **System detection.** On page import, the app finds staff lines (five evenly-spaced horizontal lines per stave, grouped into systems). Used silently throughout: segments snap to system boundaries, the context strip is placed at the correct system's left edge.
- **Bar detection.** Bar lines (tall vertical strokes between staves) are detected per system. When the user finishes drawing a segment rect, the app proposes snapping its left/right edges to the nearest bar lines. One tap to accept.
- **Auto-context-strip.** When a segment is first drawn, the app automatically proposes a small additional rect at the left edge of that system covering clef, key sig, and time sig. One tap to accept; user can adjust handles or dismiss.
- **Text OCR for markings.** Vision text recognition reads italic Italian words and dynamic letters near the segment, pre-filling the tempo / dynamic / expression caption fields. User confirms with a tap. (The OCR identifies *text*, not glyphs — we recognise the word "Largo" but not a treble clef symbol.)
- **Expand-scope candidates.** When the user accepts "expand scope" on a Solid segment (§4.4), the app proposes a candidate larger rect using bar detection: typically "next bar" or "this bar + next bar". One tap to accept.

Everything above is a suggestion overlaid on the manual primitive. If detection fails or the user disagrees, the manual flow is unchanged. The aim is that on a clean page photo, creating a usable segment with full musical context is two-or-three taps and no typing.

### 7.3 Implementation notes — v1 automation stack

The "magic" in §7.2 deliberately avoids deep-learning OMR. The full OMR pipeline (oemer, Audiveris, etc.) gives us *semantic note recognition* — which v1 does not need. v1 needs *layout detection* (where are the systems, where are the bar lines) and *text recognition* (Largo, p, ff, dim., rehearsal marks). Both are achievable on-device with Apple's Vision framework plus a small classical computer-vision pass — no ML model to train, ship, or maintain.

**Decision: build on Apple Vision + classical CV. Defer OMR.**

#### Pipeline

1. **Page rectification.** `VNDetectDocumentSegmentationRequest` (iOS 15+) on the captured photo or PDF page render. Handles phone-photo perspective for free.
2. **Binarisation.** Adaptive threshold via Accelerate/vImage or Metal Performance Shaders to produce a clean black-on-white image for the projection passes below.
3. **Staff/system detection.** Horizontal projection profile across the binarised image; staff lines appear as five evenly-spaced peaks. Peak-group detection finds staves; pairing into systems (two staves per piano grand staff) is geometric. Implementation lives in the Rust core via the `imageproc` crate so the same logic ships on Android later if we want.
4. **Bar-line detection.** Within each system's bounding box, vertical projection of dark pixels. Bar lines are tall, thin, periodic peaks — filter by aspect ratio. Yields bar segmentation per system.
5. **Text recognition.** `VNRecognizeTextRequest` restricted by region-of-interest to areas above/below staves, with a custom vocabulary hint covering Italian musical terms (Largo, Moderato, Allegro, Andante, etc.) and dynamic letters (p, mp, mf, f, ff, sf, fp, etc.). Recognised text near a segment populates the tempo / dynamic / expression caption fields.
6. **Snap-to-bar.** When the user finishes drawing a segment rect, snap its left/right edges to the nearest detected bar lines from step 4.
7. **Auto-context-strip.** Use the system bounding boxes from step 3 to propose a rect covering the leftmost portion of the system that contains the new segment (clef + key sig + time sig region).

#### What we explicitly do not do in v1

- **Semantic note recognition** (pitch, rhythm, voicing, MusicXML output). Different problem, orders of magnitude harder, not needed for the v1 feature set.
- **Glyph classification of clef, key signature, time signature.** v1 sidesteps by showing the *image* of these glyphs in the context strip. The user reads them; the app doesn't need to parse them. v2 candidate (see §11).
- **Server-side OMR.** Considered and rejected — kills the local-first design, adds 10–60 s per page latency, requires network for a feature users will try at the piano with bad wifi.
- **Bundled ML models.** No model weights shipped, no model weights downloaded on first run. Everything runs through Apple's system frameworks or hand-written CV in the Rust core.

#### Why this is enough

The user always has the manual primitive available (§7.1). The automation is *suggestion + one-tap-confirm* over those primitives — so a 70% detection accuracy is already valuable, and 90% feels like magic. A genuine OMR pipeline would only matter if we promised semantic features (transpose, MIDI export, auto-fingering) which v1 does not.

#### Effort estimate

- Vision integrations (rectification, text recognition): days, mostly glue code.
- Projection-profile staff and bar-line detection in the Rust core: 1–2 weeks of focused work to reach "good enough", measured against a small held-out set of test page photos.
- The user-correction UI for the cases the heuristic misses is the same UI as the manual primitive — no extra work, and it doubles as training data if we ever do add ML.

Total: 2–3 weeks of focused work for the v1 automation, versus 3–6 months for a deep-learning OMR port. The cost/value ratio is decisive.

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

### Mental-practice mode (v2)

Brown & Palmer (2013); Pascual-Leone's plasticity work. Low-build-cost, strong evidence base. App prompts the user to mentally rehearse a segment without the instrument — segment shown, no recording, timed silent imagery, light self-rating after. Especially valuable away from the instrument (commute, bed).

### Tempo gradation (v2)

Allingham & Wöllner (2022) — *alternating* fast/slow tempo passes outperform monotonic ramps for piano scales. Per-segment metronome with a target tempo and a "tempo plan" (e.g. 60 → 80 → 60 → 100) executed across attempts.

### Curated exercise library (v2)

The v1 stalled-segment nudge ships with a small hard-coded tag → suggestion table. v2 expands this into a curated library — Hanon for octaves, Cortot for trills, Brahms 51 for polyrhythms, Czerny for runs — keyed by tag, optionally with embedded score snippets. Pure content curation, no engineering risk.

### Memorisation drills (v2)

Per-segment `memorisation_state` field already in the model. Adds a Memory Check practice mode: segment shown briefly, blanked, user plays from memory, self-rates *got it / hesitated / lost it*. Scheduler gets a second axis: Memorised segments earn periodic memory-check visits even when their difficulty is Solid. A "random start" drill picks a random bar within the segment.

### Performance-rehearsal mode (v2)

Aufegger et al. (2017) and the pressure-simulation literature. Different from the (rejected) Concert Mode dashboard — this is a "play through the whole piece" mode that disables masks and timers, records continuously, optionally plays simulated audience audio. Fits the practice-companion positioning: you are rehearsing a performance, not performing one.

### Per-segment SMART goals (v2)

Locke & Latham's specific-difficult-goal theory. The `goal` field is already in the model; v2 wires it into the practice view as a small goal line ("today: hands together at 80bpm, no stops") and tracks whether the user reports the goal met.

### Hands-separate mode (v2, piano-specific)

Furuya et al. Adds a per-segment "hand focus" toggle in piano mode that masks half the grand staff. Practice attempts can record which hand was practiced.

### Glyph recognition (v2)

Recognising clef, key signature, and time signature as *semantic* data rather than just images. Requires a trained ML model (CoreML, on-device). Unlocks: searchable pieces by key, transposition-aware suggestions, and richer context-strip display when the strip is too small to read. v1 sidesteps by always showing the *image* of those glyphs in the context strip — adequate, but not parseable.

### Full OMR auto-segmentation (v3+)

Pre-populate segments from a recognised score (notes, beats, phrase boundaries). The mask model already supports the output (rectangles per phrase), so OMR just changes the *input* to the segment editor. The rest of the app is unchanged.

### Concert-prep dashboard (rejected)

Considered and rejected. Pulled the product toward performance, away from practice. The natural pre-concert behaviour falls out of the existing heatmap and scheduler.

## 12. Open questions for review

None blocking. The following are tunable constants that v1 will ship with sensible defaults and may expose later:

- Focus timer default (5 min).
- Session size N (5 segments).
- Auto-stop-on-silence threshold.
- Scoring function constants (difficulty weights, staleness curve, under-invested boost shape).
- Stall detection thresholds (default ≥ 6 attempts over ≥ 2 weeks, no difficulty change).
- Mastered re-exposure intervals (default 14 / 30 / 60 days).

## 13. Research notes and known tradeoffs

The design has been reviewed against published music-education and motor-learning research. Most of the design is supported. Three places are worth flagging explicitly because the evidence is mixed, contradicting, or absent.

### 13.1 What the evidence supports

- **Segmenting and chunking.** Chaffin & Imreh (2002); Williamon & Valentine (2002). Expert pianists structure practice around bar-level chunks tied to retrieval cues. Motif's segment-first model is well-aligned.
- **Slow practice.** Allingham & Wöllner (2022, *Psychology of Music*). Slow practice is near-ubiquitous among advanced musicians and aids motor accuracy. Supports the tortoise philosophy and the per-segment focus timer (which discourages racing through).
- **Spaced re-exposure for motor skills.** Shea, Lai, Black & Park (2000); Walker et al. (2002). Cross-day spacing benefits motor learning; sleep consolidates it. Supports the staleness factor in the scheduler and the maintenance intervals for Mastered segments.
- **Self-recording and audio feedback.** Daniel (2001); Hewitt (2001/2011). Improves self-assessment accuracy and performance. Supports the per-segment recording history.
- **Targeted work over total time.** Ericsson, Krampe & Tesch-Römer (1993). Supports the scheduler's emphasis on Struggling segments — but see 13.3.

### 13.2 Where the design is opinionated against the research

- **Strict-isolation practice view vs phrase-level encoding.** Williamon & Valentine (2002) found pianists who segmented at structural/phrase boundaries gave the highest-rated performances. Blacking out surrounding music removes the visual frame the brain uses to embed a chunk in its musical context. The design accepts this tradeoff in service of the "force focus" goal (preventing drift into the next bar) and mitigates it with (a) the auto-context-strip (clef/key/time visible), (b) the caption strip (tempo/dynamic visible), and (c) the context pass on graduation to Solid (re-embed in the phrase once the bit is locked). This is a deliberate values choice, not an oversight.
- **The "tortoise" philosophy.** Well supported by pedagogical tradition (Neuhaus, Whiteside, generations of teachers); less well supported by controlled experimental studies. The argument that errors practised at speed "ingrain" is more clinical observation than RCT. Motif builds the philosophy in as the default but allows full override (custom sessions, disabled timer, blocked practice mode).

### 13.3 What the evidence complicates or undermines

- **Deliberate practice as a single explanation is overstated.** Macnamara, Hambrick & Oswald (2014, *Psychological Science*) meta-analysis found deliberate practice accounts for ~21% of variance in music performance — meaningful but not deterministic. The app's framing should not promise mastery from time-on-app alone.
- **Self-rating accuracy is suspect.** Dunning–Kruger (1999) and Hewitt (2002) — students who struggle most are least accurate at self-assessing. The scheduler's primary input layer (the Struggling/Working/Solid/Mastered label) is noisiest precisely where it matters most. v1 mitigation: the under-invested boost and the stall detector both use *objective* signals (time, attempt count, difficulty-change history) so the scheduler is not 100% reliant on self-labels. v2 candidates: recording-based pitch/timing analysis, scheduled re-tests.
- **Blocked single-piece practice is worse for retention** than interleaved practice (Shea & Morgan 1979; Carter & Grahn 2016, *Frontiers in Psychology*). Addressed by the default interleaving rules in §3.
- **Mastered ≠ done.** Driskell, Willis & Copper (1992) on overlearning and Bahrick's long-term retention work — maintenance practice is required. Addressed by the Mastered floor in the scoring function (never zero) and the long-interval re-exposure curve.
- **Autonomy and intrinsic motivation matter.** Evans (2015, *Psychology of Music*); Deci, Koestner & Ryan (1999). Prescriptive scheduling and gamification can shift locus-of-causality to external and reduce long-term engagement. Mitigations: (a) overrides surfaced from day one (§3), (b) the heatmap is a progress visualisation, *not* gamification — no streaks, no badges, no loss-aversion mechanics, (c) the scaffolding-fade principle.

### 13.4 What's missing from v1 and why

These are all evidence-supported features deferred to v2, listed in §11 with their evidence base: mental practice, tempo gradation, performance-rehearsal mode, hands-separate mode, per-segment SMART goals. The v1 data model accommodates each so adoption is a UI and content effort, not a re-architecture.
