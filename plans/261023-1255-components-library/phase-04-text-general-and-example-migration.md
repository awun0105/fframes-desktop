---
phase: 4
title: "Phase 4: Text, general layer and example migration"
status: completed
priority: P1
effort: 5d
dependencies: [2]
---

# Phase 4: Text, general layer and example migration

## Goal

Ship the general motion-graphics components (text, overlays, data, audio, decoration,
backgrounds, media) by porting the high-reuse code the examples copy today, then migrate the
fork-only `examples/fframes-studio-intro` onto them and delete its duplicated helpers.
`examples/fframes-intro` is upstream-owned and keeps its own `beat.rs`/`ui.rs`; it stays a
porting source only.

## Context

- Candidate list with source lines: overview map sections 1 to 9 (`components-261023-0753-...md`).
  Port the "high" reuse rows in this phase; "medium" and "low" rows wait for a video that needs
  them (strategy roadmap phase 7).
- Duplicates to delete: `examples/fframes-studio-intro/src/{beat.rs,ui.rs}` only (fork-only
  example). Their twins in `examples/fframes-intro` are upstream and are not touched.
- The fork-only example lives in the root workspace and depends on the kit by path
  (`fframes-kit = { path = "../../components/fframes-kit" }`). Check that one `fframes` build
  is used (both resolve the same path crate inside the root workspace); if Cargo reports two
  `fframes` packages, stop the migration and record why.
- `fframes-studio-intro` and `vinuni-future-rises` also depend on the third-party
  `fframes-components` 0.1.2. Keep that dependency; only replace the duplicated in-repo helpers.
  Effects from 0.1.2 that overlap a kit component (typewriter, text reveal) are not migrated in
  this phase.

## Requirements

Components (high-reuse rows of the map):
- Text: `FitText` (shrink until it fits), `Letters` (per-glyph offset, opacity, scale),
  `TextReveal` (word or letter stagger), `Typewriter` (with cursor), `Counter`
  (tabular, formatted, `thousands`), `Paragraph` (wraps `text_break_lines`), `HudLabel`.
- Overlays: `Appear` (enter/exit wrapper; may be `Sequence` + `FadeIn`), `LowerThird`,
  `Captions` (current VTT cue), `CaptionStack`, `ChapterList`, `Callout`, `ProgressBar`,
  `Check`.
- Data: `Counter` (above), `BarChart`.
- Audio: `SpectrumBars`, `Waveform`.
- Decoration: `DrawOn` (alias of `Create` on any `Shape`), `CornerMarks`, `Glow` and
  `DropShadow` filter defs reading `shadow.glow`/`shadow.card`, `PhotoFrame`.
- Transitions: none here; `SceneFade` and the basic transitions are phase 3.
- Media: `KenBurns`.
- `motion` additions: beat grid (`Beat { bpm, downbeat }` with `time`, `frame`, `snap`,
  `soft`, `pulse`), `timecode(frames, fps)` returning `Result`.

Every component: themed defaults, `key`, text measured with `Ui::text_width`/`text_fit`
(phase 1 change) and a fallback to raw text when fonts do not resolve.

## Files

- Create: `components/fframes-kit/src/text/*.rs`, `components/fframes-kit/src/overlay/*.rs`,
  `components/fframes-kit/src/data/*.rs`, `components/fframes-kit/src/audio/*.rs`, `components/fframes-kit/src/decor/*.rs`,
  `components/fframes-kit/src/media/*.rs`, `components/fframes-kit/src/motion/beat.rs` (convert `motion.rs` into a
  module), gallery scene files per group.
- Modify: `examples/fframes-studio-intro/{Cargo.toml,src/**}` (fork-only, uses `fframes-kit`),
  `components.md` (recipes for lower third, counter, draw-on).
- Delete: `examples/fframes-studio-intro/src/beat.rs`, `examples/fframes-studio-intro/src/ui.rs`
  (whatever stays example-specific, such as branded `StatCard`, moves into that example's
  scene files).
- Do not modify: `examples/fframes-intro`, `skills/fframes-video/references/design.md`.

## Tasks & Steps

1. For each component: read the source lines from the map, port once with props for the
   hard-coded values, add a gallery scene named after it, a doctest row in `components.md`.
2. Order: motion additions, text, overlays, data, audio, decoration, media.
3. First run `git status --short examples/fframes-studio-intro`. The tree already has the
   user's uncommitted `src/scenes/keynote/speed.rs`; never stage it. If a file the migration
   must change has uncommitted user changes, mark the phase `Blocked` and ask. Then, before
   migrating, render key frames of `fframes-studio-intro` with `frame` (CPU renderer)
   as the reference; migrate; re-render; diff with odiff. Accept only differences explained by
   a deliberate fix (record them).
4. Delete the duplicated helper files; `cargo check -p fframes-studio-intro` at the root.
5. Add the lower third, counter and draw-on recipes to `components.md`.

## Verification

- `cargo test -p fframes-kit`, `just -f components/justfile e2e cpu` (gallery snapshots added),
  `just -f components/justfile check-wasm`, `just -f components/justfile clippy`.
- `fframes-studio-intro` key frames equal the reference (odiff threshold of the e2e harness);
  `inspect` on it: no new warnings; `cargo clippy -p fframes-studio-intro -- -D warnings`.
- `just -f components/justfile boundary` prints nothing.
- Bench pair within 10%; `BenchKit` extended with `LowerThird` and `Counter`.

## Risks

- **Pixel drift during migration.** The shared implementation may round differently from one
  of the copies. Keep the odiff threshold of `components/scripts/e2e.mjs`; investigate anything above it.
- **Scope creep.** Medium/low rows of the map stay out unless a migrated example needs them.

## Todo

- [x] Beat grid and `timecode` in `motion`
- [x] Text components
- [x] Overlay components
- [x] Data, audio, decoration and media components
- [x] Reference renders of `fframes-studio-intro` before migration
- [x] Migrate `fframes-studio-intro`; delete its `beat.rs`/`ui.rs` (kept as a facade and `brand.rs`, see the report)
- [x] Key frames unchanged or differences explained
- [x] `components.md` recipes
