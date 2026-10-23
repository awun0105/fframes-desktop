---
phase: 8
title: "Phase 8: Studio props panel"
status: cancelled
priority: P2
effort: 5d
dependencies: [3, 4, 6]
---

# Phase 8: Studio props panel

## Cancelled (2026-10-23)

The user decided: "stop before phase 8, i do not want to touch the fframes core. or fframes
studio. It must be independence from the fframes-kit component library". This phase needs
changes to fframes core and to fframes Studio, so it is not done. Nothing below is executed;
it stays only as a record of the old design. No other phase depends on it.

## Goal

Select a kit component on the Studio canvas, see its props in a panel, change one, and see the
preview update without editing code. The change is saved in a project document, the same way
timeline edits are saved in `timeline.json`.

## Context

- Design: component model report section 5 (`#[derive(Props)]`, schema keyed by
  `component_key`, identity through `EditorObjectKey`).
- Precedent to mirror (verified 2026-10-23): timeline edits.
  - Document type `fframes::TimingDoc`; the CLI opts in with `.timing_file("timeline.json")`.
  - Studio writes it at the end of a gesture: `desktop/crates/studio-engine/src/controller/timing.rs`
    (undo history, "changed outside the studio" reload, agent-published reload).
  - The worker receives it with `PreviewRequest::SetTiming`, gated by the
    `set_timing_v1` capability (`fframes-studio-protocol/src/preview.rs:30`), handled in
    `fframes-studio-runtime/src/preview_worker.rs:784` and applied with
    `Previewer::set_timing`.
  - Preview coordination: `desktop/crates/studio-engine/src/services/preview_coordinator.rs:408`.
- Selection already carries identity and style tokens:
  `desktop/crates/studio-engine/src/canvas_selection.rs` (`CanvasSelection`, `hit_test`),
  shown in `desktop/crates/studio-ui/src/canvas_view.rs`.
- Desktop is its own Cargo workspace (`desktop/`); its tests run there.

## Requirements

- `#[derive(Props)]` (proc macro crate `fframes-kit-macros`) on component structs emits
  `fn schema() -> PropsSchema` (field, type, default, range, unit, theme token) and
  `fn apply(&mut self, values: &PropsValues)` for the editable subset: numbers, strings,
  `Color`, enums, `bool`, `Anim::Fixed` values. Borrowed data, closures and `Anim::Keyframes`
  are not editable and are left out of the schema.
- A props document `props.json` (`fframes::PropsDoc`, versioned like `TimingDoc`, size-limited,
  rejected whole when invalid): entries keyed by the four `EditorObjectKey` parts, each a map of
  field to JSON value.
- `Ui::show` applies the matching entry before `render` when the component has a `key`, and
  attaches the schema and current values to the object's editor metadata so Studio can build
  the panel from the frame without a separate registry.
- Opt-in like timing: `cli::new(..).props_file("props.json")`, `RenderOptions::props`,
  `Previewer::set_props`; a missing file changes nothing. Renders use the same document, so
  export matches preview.
- Protocol: `PreviewRequest::SetProps` with capability `set_props_v1`; old workers keep working.
- Studio: a "Props" section for the selected object when it has a schema; controls per type
  (number with range and unit, text, colour, enum, toggle); a "reset to code" per field;
  edits are gestures with undo, written at gesture end, reloaded when changed outside Studio.
- Components built in phases 1 to 7 get the derive; phases 9 to 11 use it from the start.

## Files

- Create: the `Props` derive in `components/fframes-kit-macros/src/` (crate made in phase 7), `fframes/src/props_doc.rs`,
  `desktop/crates/studio-engine/src/controller/props.rs`,
  `desktop/crates/studio-ui/src/props_panel.rs`, tests next to each.
- Modify: `fframes/src/{lib.rs,editor.rs,fframes_context.rs}` (doc type export, editor
  metadata field for schema and values, access from the context),
  `fframes/src/renderer/{cli.rs,preview.rs}` (`props_file` next to `timing_file` at
  `cli.rs:485`, `set_props` next to `set_timing` at `preview.rs:365`), `fframes/src/timing.rs`
  (read only, the `TimingDoc` pattern to mirror), `fframes-studio-protocol/src/preview.rs` (request and
  capability), `fframes-studio-runtime/src/preview_worker.rs` (handle it),
  `desktop/crates/studio-engine/src/{controller.rs,services/preview_coordinator.rs}`,
  `desktop/crates/studio-ui/src/{canvas_view.rs,shell/layout.rs}` (place the panel),
  `components/fframes-kit/src/**` (derive on existing components), `components.md` (how props become
  editable: give the component a `key`), `docs/desktop/README.md` (the props panel and
  `props.json`).

## Tasks & Steps

1. Read `controller/timing.rs`, `preview_worker.rs` around `SetTiming`, `canvas_selection.rs`,
   and the editor metadata path in `fframes/src/editor.rs`; write a half-page design note in
   `plans/reports/` confirming where schema and values travel. Stop and ask if it contradicts
   this phase.
2. `PropsDoc` with parse, validate, size limit and tests (mirror `TimingDoc` tests).
3. `fframes-kit-macros` derive with tests on a sample struct (schema output snapshot, `apply`
   on each field type, unknown field ignored with a warning).
4. Core opt-in API and editor metadata field; `cargo test -p fframes`.
5. Protocol request and worker handling with capability gating; runtime tests.
6. Studio controller (gesture, undo, external change reload) and panel UI.
7. Add the derive to existing components; `components.md` section.
8. End to end: in a Studio project using the kit (`components/gallery` copy via
   `prepare_examples` or a fixture), change `LowerThird.name`, see the preview change, export,
   and confirm the export shows the change.

## Verification

- `cargo test -p fframes`, `cargo test -p fframes-kit`, `cargo test -p fframes-studio-runtime`.
- In `desktop/`: `cargo test -p studio-engine` and `cargo test -p studio-ui` for the new tests.
- Manual check on this VM with the Studio app (CPU preview): select, edit, undo, reload after an
  outside edit. Record screenshots in the phase report.
- `just -f components/justfile clippy`, and `cargo clippy -- -D warnings` in `desktop/`.

## Risks

- **Metadata size.** Schemas on every frame could bloat frame metadata; send the schema once
  per `component_key` per preview session and only values per frame if the note in step 1
  shows a cost.
- **Old workers.** Capability gating must keep projects built before this phase working.
- **Scope.** Writing values back into Rust source is out of scope; `props.json` is the store.

## Todo

- [ ] Design note confirming where schema and values travel
- [ ] `PropsDoc` with tests
- [ ] `#[derive(Props)]` in `fframes-kit-macros`
- [ ] Core opt-in API (`props_file`, `set_props`, `RenderOptions::props`)
- [ ] `SetProps` protocol request with `set_props_v1` capability
- [ ] Studio controller with undo and external reload
- [ ] Props panel UI
- [ ] Derive added to existing components
- [ ] End-to-end check in Studio including export
