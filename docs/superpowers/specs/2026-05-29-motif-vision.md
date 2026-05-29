# Motif — Product Vision (draft)

**Date:** 2026-05-29
**Status:** Draft

## The problem

Most people learning an instrument waste most of their practice time. Not through laziness — through the wrong defaults. They open their score, start at bar 1, play the opening they already know, fumble through the middle they haven't solved, and stop when it gets hard. Tomorrow they do the same thing. Weeks pass. The opening gets gradually more polished. The hard passages don't.

There are two specific failure modes underneath this:

- **Avoidance.** Practice gravitates toward what feels comfortable. The intro gets a hundred repetitions; the difficult bars on page 6 get five. The piece never improves where it actually needs to improve.
- **Obsession.** The opposite trap — drilling one bar for forty minutes, fixating on perfection at the cost of forward progress. Both failure modes look like "practising" but neither moves the player forward.

Underneath both lies a deeper problem: **the player has no map of their own piece.** They don't know where they are, where they're stuck, or where to spend the next hour. There's no honest mirror for their own practice — and without one, they're guessing.

## The vision

Motif gives every player a map of their piece and a plan for today. It breaks a score into small, practiced units; it remembers what's hard, what's stale, what's mastered, and what's been neglected; and it tells you what to work on right now — biased toward the bits you've been avoiding. It locks your focus on one chunk at a time so you can't drift, and it nudges you when you've drilled it long enough. It records you so you can hear yourself improve over weeks. And when something has been stuck for too long, it doesn't pretend — it tells you, and it points you toward an exercise that might help.

You don't have to know how to practise. The app practices for you.

## Who it's for

**Adult amateur and intermediate instrumentalists** — primarily pianists at first — who are self-directed (no teacher, or only occasional lessons), motivated, and frustrated by their own progress. They have pieces they care about, time they can give, and the persistent feeling that they're not getting better as fast as they should. They don't need a teacher's eye on every bar; they need a structure that prevents them from sabotaging themselves.

Motif is **not** for:

- Sight-readers who learn a piece in three sessions
- Beginners who don't yet have pieces of their own
- People looking for entertainment, gamification, or social practice
- Professionals who already have a refined practice discipline

## What Motif is and isn't

**Motif is a practice companion.** It sits next to your existing score (paper, PDF, ForScore) and helps you decide and execute what to do with the time you've allocated to practising.

**Motif is not:**

- A score reader. It doesn't replace how you read music.
- A performance app. It's used in the practice room, not on stage.
- A metronome, tuner, or note-trainer. Those are commodities.
- A gamification layer. No streaks, no leagues, no badges.
- A teacher. It doesn't tell you what to do with your hands — it tells you what to spend time on.

## Five principles

1. **Tortoise, not hare.** Every default pushes toward slow, careful, locked-in work on small chunks before larger ones. Mastery of fragments compounds; performance-tempo runs through unmastered material erodes.

2. **Force focus.** The practice view shows one segment, full screen, with everything else hidden. You cannot drift into the next bar because there is no next bar visible. The opposite failure — obsessing on one bar — is caught by a gentle timer-nudge.

3. **The map matters.** A bird's-eye heatmap of the whole piece, with mastery, staleness, and time-invested visible on one image. The player can always see *where they are* and *where the work is*.

4. **Honest tools.** No streaks, no progress theatre. The heatmap shows you what's actually solid and what isn't. When a passage has stalled for weeks, the app says so — and points you elsewhere. Self-assessment is treated as a noisy signal, not ground truth, and supplemented with objective data the app already has.

5. **Low admin.** Setting up a segment is two or three taps and no typing on a clean page photo. The app uses on-device vision to detect systems, snap to bar lines, propose context strips, and pre-fill tempo and dynamic markings. The user's time goes into practising, not configuring.

## Roadmap arc

### v1 — the loop

Manual segment creation with heavy on-device automation. Hybrid difficulty × staleness × time-invested scheduler. Strict-isolation practice view with focus timer. Heatmap as a planning view across the whole piece. Per-segment audio history. Stalled-segment detection with hard-coded exercise suggestions by tag. Local-first, no account, no server.

This is the smallest version that delivers the core promise: a map, a plan, and a focused practice loop.

### v2 — the depth

Adds the techniques the research strongly supports but that aren't load-bearing for v1:

- Mental-practice mode (motor imagery away from the instrument).
- Tempo gradation with alternating-tempo presets.
- Curated exercise library by tag — expanding the v1 hard-coded table into real content.
- Memorisation drills with random-start practice.
- Performance-rehearsal mode (play-through with simulated pressure).
- Hands-separate mode for piano.
- Per-segment SMART goals.
- CloudKit sync across devices.

### v3 — the magic

The features that require ML or substantial new effort:

- On-device glyph recognition (semantic clef, key sig, time sig — searchable, transposable).
- Full optical music recognition: pre-populated segments at phrase boundaries from a recognised score.
- Recording-based objective signals (pitch and timing analysis, used as ground-truth alongside self-rating).
- Other instruments and ensemble practice models.

## How we'll know it's working

Not vanity metrics. Specific behavioural signals that the product is solving the real problem:

- **Practice distribution shifts.** Users spend a measurably higher proportion of time on Struggling/stale segments after a few weeks than they did at start. The avoidance problem is being broken.
- **Sessions per piece compound.** Users return to the same piece week after week and the heatmap visibly greens up — not just for the opening, but evenly across pages.
- **Stalled segments resolve.** When the stall card surfaces and a user follows a suggestion, the segment subsequently improves. The "jumping-off point" actually jumps off.
- **Audio history is listened to.** Users replay their own old takes — a sign the long-term progress visualisation is emotionally meaningful, not just data.
- **Pieces get finished.** Users report performing or finishing pieces they'd been stuck on. The lagging indicator that matters.

What we are *not* trying to maximise: daily active use, time-in-app, streaks. A user who opens Motif twice a week and practises better is the win. A user who opens it daily because of a streak nudge is the failure.

## The strategic bet

Most practice apps offer the wrong thing — usually a metronome, a tuner, a library of lessons, or a gamified daily-task scaffold. None of them solve the actual problem self-directed adult learners face, which is *how to spend the next hour*.

The bet is that a small, opinionated tool that answers that single question — by giving the player a map, a queue, and a focused practice loop — will earn its place next to the score and stay there. Because the player keeps getting better, and they can feel it, and the heatmap shows it.

That's the product. Everything else is decoration.
