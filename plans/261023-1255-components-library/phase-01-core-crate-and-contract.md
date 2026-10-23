---
phase: 1
title: "Phase 1: Core crate and contract"
status: completed
priority: P1
effort: 3d
dependencies: []
---

# Phase 1: Core crate and contract

## Goal

Create `fframes-kit` with the component contract (`Ui`, `Component`, time containers, `Anim`,
`Theme`, `FadeIn`/`FadeOut`), the `components-gallery` example with its editor bridge, the
doctested skill reference, and a cost baseline. Freeze the public names agents will copy.

## Context

- Contract: component model report section 3 (`research-261023-1700-...md`), sections 3.1 to
  3.7. Use those signatures unless a step below says otherwise.
- Verified facts (2026-10-23):
  - `Frame::text_width(&mut self, ctx, font_query, value: &'a str)` (`fframes/src/frame.rs:354`)
    and `Frame::text_fit(.., value: &'a str, ..) -> Option<Cow<'a, str>>` (`frame.rs:393`) tie
    the string to the context lifetime. `text_break_lines(_structure)` already take `&str`.
  - `Frame::clone_with_scene_offset(&Frame, usize)` is public (`frame.rs:76`); it computes
    `index - offset`, so callers must check `index >= offset`.
  - `FFramesContext.duration_in_frames` (`fframes/src/fframes_context.rs:48`) is the video's
    length, or the scene's own length inside a scene.
  - Not used (the kit is independent of fframes Studio): `EditorObjectKey`,
    `Svgr::with_editor_object`, `Scene::editor_instance_key` and `fframes::Styles` are fork
    additions for Studio; nothing under `desktop/` is read or referenced.
  - The root `Cargo.toml` has `[patch.crates-io] fframes = { path = "fframes" }` for the
    third-party `fframes-components` 0.1.2; leave it in place. Patches do not cross workspaces;
    the kit workspace needs no patch because it depends on `fframes` only by path.
  - `ctx.font_source: Option<&dyn FontSource>` is public upstream
    (`fframes/src/fframes_context.rs:55`, traits in `fframes/src/font_data.rs:76-104`):
    `resolve_font(..)` gives a `FontFace` with `resolve_char_width(size, char)`. That is how the
    kit measures strings it builds per frame, without kerning; strings borrowed for `'a` go
    through the core `Frame::text_width` unchanged.
  - `include_media_dir!` reads its path relative to the compiler's working directory, which is
    the workspace root, so inside the kit workspace the gallery writes
    `include_media_dir!(pub struct GalleryMedia, "gallery/media")`.
  - `just check-wasm` and `scripts/e2e.mjs` hard-code `examples/<name>`; the kit gets its own
    copies under `components/` (copied, never edited upstream).

## Requirements

Functional:
- `fframes-kit` is a member of the nested `components/` workspace (excluded from the root
  workspace like `desktop/`); `use fframes_kit::prelude::*;` is the only import a video needs.
- `Component<'a>` trait: `fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a>`, plus a boxed
  form (`self: Box<Self>`) so `Vec<Box<dyn Component>>` works.
- `Ui { frame, duration, ctx, theme }` with `Ui::new(frame, ctx, theme)`, `show`, `defaults::<T: Themed>()`, `seconds()`, `text_width`,
  `text_fit`, `text_break_lines` pass-throughs with the core signatures (`&'a str`), and
  `Ui::measure(font, &str) -> Option<f32>` for strings built per frame (through
  `ctx.font_source`, see Context).
- Only upstream fframes APIs; no fork-only or Studio API, so the kit builds against plain
  upstream fframes.
- Time containers: `Sequence { from, duration, children }`, `Series`, `Loop { duration, times,
  children }`, `Freeze { at, children }`, `Fill` (full-canvas group, Remotion's `AbsoluteFill`).
  Each checks `index >= offset` before building the child frame and returns `Svgr::empty()`
  outside its range.
- `Children<'a> = Box<dyn Fn(&mut Ui<'a, '_, '_>) -> Svgr<'a> + 'a>` with a `children(|ui| ..)`
  helper so call sites avoid writing `Box::new`.
- `Anim<'a, T> { Fixed(T), Keyframes(&'a KeyFramesAnimation<T>) }` with `From<T>` and
  `ui.sample(&anim)`.
- `Enter`/`Exit` traits and `FadeIn { shift, delay, run_time, rate_func }`, `FadeOut`. `exit`
  times are measured back from the end of `ui.duration`.
- `RateFunc` with Manim's names (`Smooth` default, `Linear`, `RushInto`, `RushFrom`,
  `ThereAndBack`, `DoubleSmooth`, plus `Easing(fframes::animation::Easing)`).
- `motion` module: `progress`, `clamp01`, `lerp`, `stagger(i, step)`, `hash`, `noise` (seeded,
  no `rand`), closed-form `spring`. Sources in the overview map section 1.
- `Theme` with typed groups (colors, typography, spacing, radius, stroke, shadow, motion),
  `Theme::default()` and struct update for overrides (`Theme { colors: .., ..Theme::default() }`).
- `key: Option<&'a str>` on components that create SVG ids (clip paths, gradients, filters):
  ids are derived from the key or the item index, never a global counter. It is a plain
  identifier with no editor meaning.

Non-functional:
- No panics, no I/O, no `Sync` bound on `Ui`; per frame only arithmetic and small strings.
- Decorative markup in components stays literal so `svgr!` caches it.
- The default feature set builds for `wasm32-unknown-unknown`.

## Architecture

```text
components/
  Cargo.toml          [workspace] members fframes-kit, gallery, gallery/editor/editor-bridge;
                      workspace deps fframes = { path = "../fframes" }, kurbo = "0.13";
                      lints copied from the root workspace
  Cargo.lock          committed
  justfile            check-wasm, e2e, clippy, fmt, test, boundary (see plan.md)
  scripts/e2e.mjs     copy of the root harness reduced to the gallery; odiff-bin resolves from
                      the root node_modules
components/fframes-kit/
  Cargo.toml          features: default = [], latex, math, physics, chemistry,
                      cs, biology, skia, 3d, lottie (empty until their phase)
  src/lib.rs          crate docs with the Remotion and Manim name tables, `pub mod`s, doctest hook
  src/prelude.rs      re-exports
  src/ui.rs           Ui, Component, Themed, Children, children()
  src/time.rs         Sequence, Series, Loop, Freeze, Fill
  src/anim.rs         Anim, RateFunc, Enter, Exit, FadeIn, FadeOut
  src/motion.rs       progress, lerp, stagger, spring, hash, noise
  src/theme.rs        Theme and its defaults
components/gallery/
  Cargo.toml          lib + bin, `renderer` feature like hello-world
  src/lib.rs          GalleryVideo: one Scene per component, scene name = component name
  src/main.rs         fframes::cli with `.renderers(SkiaRenderers::default())`
  src/scenes/core.rs  Sequence, Series, Loop, Freeze, FadeIn, FadeOut, Bench* scenes
  media/              DM Sans (copy from hello-world)
  snapshots/          e2e PNGs (committed)
  editor/editor-bridge/{Cargo.toml,lib.rs}
docs/components/
  README.md           index of the agent references
  components.md       the agent reference, doctested from fframes-kit
```

Gallery scenes are one generic `GalleryScene { name: &'static str, seconds, draw: fn(&mut Ui) ->
Svgr }` whose `name()` returns the component name, so
`frame Sequence@50%` and the e2e harness address them by component name.

## Files

- Modify (the only two edits outside `components/` and `docs/` in the whole plan): root
  `Cargo.toml` line 62, `exclude = ["desktop"]` becomes `exclude = ["desktop", "components"]`;
  `AGENTS.md`, one line under the project structure list:
  `/components  # fframes-kit component library, its own Cargo workspace (docs/components/)`.
- Create: everything under `components/` above, `docs/components/{README.md,components.md}`.
- Never modify: `fframes*/`, `svgr-macro/`, `media-dir-macro/`, `examples/`, `skills/`, `e2e/`,
  `scripts/`, `justfile`, `cargo-fframes/` (the boundary check in `plan.md`).

## Tasks & Steps

1. **Workspace.** Create `components/Cargo.toml` (`[workspace]`, `resolver = "2"` like the root, members,
   workspace deps, the root's `[workspace.lints]` copied), add `"components"` to the root
   `exclude`, and confirm `cargo metadata` at the root no longer lists anything under
   `components/`. Record `git rev-parse HEAD` as the baseline in `plan.md` if it is not set.
2. **Crate skeleton.** `components/fframes-kit/Cargo.toml` (`edition = "2024"`, `license = "MIT"`,
   `[lints] workspace = true`, deps `fframes.workspace = true`, `kurbo.workspace = true`). Build once from `components/` (`cargo build -p fframes-kit`); the first build
   compiles fframes into `components/target`.
3. **`motion`.** Port `progress`/`lerp`/`spring`/`hash`/`noise` from the sources in the overview
   map section 1, one implementation each, with unit tests (spring settles to 1, noise is
   deterministic for a seed, `progress` clamps).
4. **`Theme`.** Typed fields with neutral defaults documented in `theme.rs`. Tests: defaults
   are sane (contrast of text on background, positive durations), a partial override keeps
   the other defaults.
5. **`Ui`, `Component`, `Themed`, `Children`.** As in the contract. `Ui::new` fills `duration`
   from `ctx.duration_in_frames`.
6. **Time containers.** `Sequence` exactly as the report's section 3.4 sketch; `Series` places
   items back to back with optional negative overlap; `Loop` wraps the local index; `Freeze`
   clones the frame at a fixed local index. Unit tests on frame arithmetic: before `from`,
   at `from`, last frame, after the end, nested sequences add up.
7. **`Anim`, `RateFunc`, `FadeIn`/`FadeOut`.** `FadeIn` wraps content in
   `<g opacity={..} transform={..}>` with the content as an expression child.
8. **Ids.** A helper `ui.id(key, index, kind)` that builds a unique, deterministic SVG id;
   test that two components of the same kind in one frame get different ids.
9. **Gallery.** Write `components/gallery` by hand (package `components-gallery`, lib + bin),
   modelled on `examples/hello-world`; copy (do not move) its editor bridge and DM Sans font, and
   gate `cli` and `compile-time-svgtree` behind `renderer`. Do not run `just new`: it writes
   into `examples/` and the root members. Scenes: `Sequence`, `Series`, `Loop`, `Freeze`,
   `FadeIn`, `FadeOut`, `Theme` (swatches of every token), `BenchHandWritten`, `BenchKit`.
10. **Bench pair.** `BenchHandWritten` draws a lower-third-like card with three staggered fades
    written in plain `svgr!` + `timeline!`; `BenchKit` draws the same with `Sequence` and
    `FadeIn`. Time `render BenchHandWritten` and `render BenchKit` in release with
    `--renderer cpu` (`Measure-Command` in PowerShell, 3 runs each, median) and record the
    numbers in the phase report. This pair is the baseline for every later phase.
11. **Skill reference.** `docs/components/components.md`: the one import line,
    "time is local" with `Sequence`, the props rules (struct update with `..ui.defaults()`,
    props are plain data, `key` for SVG ids), a table of the phase-1 components, and a full
    copy-paste video. Hook it as doctests in `components/fframes-kit/src/lib.rs`:
    `#[cfg(doctest)] #[doc = include_str!("../../../docs/components/components.md")] struct ComponentsDocs;`
    Snippets needing fonts are `no_run`. `docs/components/README.md` lists the reference files.
12. **Wiring.** `components/justfile` recipes: `check-wasm` (`cd gallery/editor/editor-bridge &&
    cargo check --lib --target wasm32-unknown-unknown`), `e2e renderer *ARGS`
    (`node scripts/e2e.mjs`), `test`, `clippy` (`cargo clippy --all-targets -- -D warnings`),
    `fmt`, `boundary` (the check in `plan.md`). `components/scripts/e2e.mjs` is the root
    harness copied and reduced to one entry, with `root` = `components/`, snapshots in
    `gallery/snapshots`, binaries from `components/target/release`, run from `gallery/`
    (`{ dir: "gallery", pkg: "components-gallery", bin: "components-gallery", frames: "Sequence@50%,Series@50%,FadeIn@50%,Theme@50%" }`).
    Then
    `just -f components/justfile e2e cpu --update` to create snapshots, review the PNGs by eye, then `just -f components/justfile e2e cpu`.
13. **Pointers.** The one `AGENTS.md` line from Files, nothing else in upstream-owned files.
    Run `just -f components/justfile boundary`; it must print nothing.

## Verification

- From `components/`: `cargo test -p fframes-kit` (unit + doctests) passes.
- At the root: `cargo metadata --no-deps` lists no package under `components/`, and
  `cargo check -p fframes` still passes (the root build is untouched).
- `cargo run --release -p components-gallery -- --renderer cpu timeline` lists every scene by
  component name; `inspect` reports no errors.
- `just -f components/justfile check-wasm` passes.
- `just -f components/justfile e2e cpu` passes.
- `just -f components/justfile clippy` and `cargo fmt --all --check` (from `components/`) pass.
- `just -f components/justfile boundary` prints nothing.
- Bench numbers recorded; `BenchKit` within 10% of `BenchHandWritten`.

## Risks

- **Lifetimes on `Children`.** `Box<dyn Fn(&mut Ui<'a, '_, '_>) -> Svgr<'a> + 'a>` may need a
  higher-ranked bound (`for<'c, 'm>`). Prototype `Sequence` inside a `Sequence` first; if the
  bound fails, make `Ui` carry one lifetime for `ctx` and the media (`Ui<'a, 'c>`).
- **Per-character measurement.** `Ui::measure` sums `resolve_char_width` without kerning,
  so it can differ from `Frame::text_width` by a few pixels. Use it only for strings built
  per frame; test it against `text_width` on a borrowed string and record the difference.
- **Separate build.** The kit workspace has its own `target/` and lockfile, so the first
  build recompiles fframes and its FFmpeg bindings. Keep `components/Cargo.lock` committed.

## Rollback

All new code is in `components/` and `docs/components/`. Reverting the commit removes them
and the two pointer lines; nothing upstream-owned changes.

## Todo

- [x] Nested `components/` workspace, root `exclude` entry, baseline recorded
- [x] Crate skeleton, `Ui::measure`
- [x] `motion` module with tests
- [x] `Theme` with defaults and tests
- [x] `Ui`, `Component`, `Themed`, `Children`
- [x] `Sequence`, `Series`, `Loop`, `Freeze`, `Fill` with frame arithmetic tests
- [x] `Anim`, `RateFunc`, `FadeIn`, `FadeOut`
- [x] Deterministic SVG ids
- [x] Gallery example with editor bridge and scenes
- [x] Bench pair measured and recorded
- [x] `components.md` doctested, `docs/components/README.md`, the `AGENTS.md` line
- [x] `components/justfile`, `components/scripts/e2e.mjs`, snapshots; all verification commands green; boundary empty
