---
phase: 8
title: "Phase 8: Studio props panel"
status: completed
priority: P2
effort: 5d
dependencies: [3, 4, 6, 7]
---

# Phase 8: Studio props panel

## Scope decision (2026-10-24)

The user first cancelled this phase (2026-10-23: "i do not want to touch the fframes core. or
fframes studio"). On 2026-10-24 they reopened it: "just keep the fframes core, can edit the
studio". So the old design is replaced. Nothing in `fframes*/` or any other upstream-owned path
changes. Instead the work is split between the kit (`components/`) and fframes Studio
(`desktop/`, `fframes-studio-protocol/`, `fframes-studio-runtime/`). The fork owns Studio; these
paths are not in `upstream/main`.

Facts checked before the redesign (2026-10-24):

- **Fresh frames.** The preview worker keeps no frames keyed by index
  (`fframes-studio-runtime/src/preview_worker.rs`). `Previewer` caches only content-keyed data:
  static subtrees by `static_hash`, text, and the converter. Props live in the video, so a
  changed value gives a new frame without `Previewer::set_props`.
- **Selection identity.** Canvas selection reads `EditorFrameMetadata.objects`, which core builds
  from `<g id={EditorObjectKey::render_id()}>`. `EditorObjectKey` is a fork API in
  `fframes/src/editor.rs`. Using it does not change core, but the default kit build must stay
  pure upstream, so it is behind the kit feature `studio`.
- **Studio projects** pin `fframes-studio-runtime`/`-protocol` from crates.io (`=0.2.0`,
  `desktop/crates/studio-project/src/lifecycle.rs`). The kit is not published. So the default
  project template stays unchanged, and the end-to-end check uses a fixture with path
  dependencies, as `desktop/fixtures/annotated-video-overlay` does.

## Goal

Select a kit component on the Studio canvas, see its props in a panel, change one, and see the
preview update without editing code. The change is saved in the project's `props.json`, the
way timeline edits are saved in `timeline.json`. Export uses the same document.

## Design

Crate graph: kit → protocol (feature `studio`), runtime → protocol, desktop → protocol. Nothing
depends on the kit except video projects. The project wires the kit's store into the worker
with closures, so neither Studio nor the runtime knows the kit.

- **Protocol (`fframes-studio-protocol/src/props.rs`):**
  - `PropsDocument`: version 1, a list of objects, each with the four identity parts and a map
    of field to JSON value. It has a size limit and refuses unknown fields; an invalid
    document is rejected whole.
  - The schema types: `PropsSchema` (component name and fields), `PropField` (name, kind,
    range, step, unit, options) and `PropKind` (number, integer, text, colour, toggle,
    choice).
  - `PropsInfo`: the schema, plus the code values of one object.
  - Requests:
    - `PreviewRequest::SetProps` (generation and the document as JSON text, like
      `SetTimingRequest`);
    - `PreviewRequest::PropsInfo` (an `EditorObjectIdentity`).
  - Capability `set_props_v1`. It is optional, and a worker offers it only when the project
    gave it props hooks.
- **Runtime:**
  - `PreviewWorkerConfig::with_props(PropsHooks { apply, info })` and `with_props_args`
    (`--props-file`, `--props-generation`, read and applied at startup).
  - The worker answers `SetProps` and `PropsInfo` and lists the capability in Hello.
  - Export: `run_export_with_props(&video, &options, &args, hooks)` applies `--props-file`
    before rendering. `run_export` keeps its signature.
- **Kit (feature `studio`):**
  - `#[derive(Props)]` in `fframes-kit-macros` emits `fframes_kit::props::Props`:
    - `schema()`;
    - `values()` (JSON of the editable fields);
    - `apply(&Map)`, which returns the fields it could not apply.

    Editable field types implement `PropValue`: floats, integers, `bool`, `String`,
    `Cow<str>`, `Color`, `Anim<f32>` holding `Fixed`, and kit enums through
    `#[derive(PropChoice)]`. Other fields are left out through autoref specialization, so
    children, closures, borrowed slices and keyframes need no attribute. `#[props(skip)]`,
    `#[props(range = a..b, step = s, unit = "px")]`.
  - `PropsStore`: `Arc<RwLock<Arc<PropsDocument>>>`, plus the code values of each shown
    object, recorded while a host listens.
    - `PropsStore::load(path)` serves CLI renders;
    - `store.hooks()` gives the runtime closures.
  - `Ui::with_props(&store)` snapshots the document once per `Ui`. Shifted UIs share it.
    `Ui::for_scene` keeps the scene's key: `editor_instance_key()`, else `name()`.
  - `Editable { key, component }`, also written `component.editable("title")`, and the tag
    attribute `editable="title"` in `ui!`. Its render:
    1. applies the matching entry;
    2. records the code values;
    3. renders the component;
    4. wraps it in `<g id={EditorObjectKey::new(scene, Type, key, "main").render_id()}>`.

    Without a store it renders the component unchanged.
- **Studio engine (`desktop/crates/studio-engine`):** a props module mirroring `timing/`:
  - document, atomic file and history;
  - external change reload;
  - a controller with gestures and undo;
  - sending `SetProps` when the worker offers `set_props_v1`;
  - `PropsInfo` for the selected object.

  `props.json` is left out of the build identity like `timeline.json`.
- **Studio UI (`desktop/crates/studio-ui`):** a Props section for the selected object when the
  worker returns a schema. It has a control per kind, "reset to code" per field, and gestures
  with undo.

## Requirements

- No edit under `fframes*/` or any other upstream-owned path (boundary check, both scopes).
- The kit's default build does not use fork-only APIs; `studio` is off by default.
- The worker without hooks behaves exactly as before, and old clients never send `SetProps`.
- Renders with the store, preview and export, draw the same values.
- No new dependency in root-workspace or desktop crates. The root and desktop `Cargo.lock`
  files carry the user's own changes and cannot be staged.
- Do not edit the user-modified `desktop/crates/studio-ui/src/kit/number_field.rs` or
  `desktop/crates/studio-engine/tests/materialization.rs`. Stage Studio files by explicit path.

## Files

- Create:
  - `fframes-studio-protocol/src/props.rs`;
  - `components/fframes-kit/src/props.rs`;
  - `components/fframes-kit-macros/src/props.rs`;
  - `desktop/crates/studio-engine/src/props/`;
  - `desktop/crates/studio-engine/src/controller/props.rs`;
  - `desktop/crates/studio-ui/src/props_panel.rs`;
  - a fixture `desktop/fixtures/kit-props/`;
  - tests next to each.
- Modify:
  - the protocol's `lib.rs` and `preview.rs`;
  - the runtime's `preview_worker.rs`, `export.rs` and `lib.rs`;
  - the kit's `ui.rs`, `lib.rs`, `prelude.rs` and `Cargo.toml`;
  - `fframes-kit-macros/src/{lib.rs,tags.rs}`;
  - the derive on existing components;
  - in the desktop engine and UI: `controller.rs`, `preview_coordinator.rs`, `revision.rs`, the
    shell layout and the canvas view;
  - `docs/components/components.md`, `docs/desktop/README.md`.

## Tasks & Steps

1. Protocol types, with tests: document parse, limits and round trip, and old-worker gating.
2. Kit `props` module and `#[derive(Props)]`, with tests: schema snapshot, `apply` for each
   type, unknown and invalid fields reported, non-editable fields left out.
3. `Editable`, `Ui::with_props`, and `editable=` in `ui!`. The gallery test checks that a doc
   entry changes the drawn tree and that the id decodes to the identity.
4. Runtime hooks, `SetProps`, `PropsInfo`, startup and export args, with runtime tests on a
   probe video.
5. Studio engine props module and controller, with tests mirroring `timing.rs`.
6. Props panel UI and wiring.
7. Derive on existing components; docs.
8. End to end with the `kit-props` fixture:
   - in the worker protocol, `PropsInfo` and `SetProps` change the frame and the export;
   - manually in the Studio app on this VM, if it runs: select, edit, undo, outside edit.

## Verification

- `cargo test -p fframes-studio-protocol -p fframes-studio-runtime` (root workspace).
- In `components/`: kit tests with `studio` and without it, macro tests, gallery tests, clippy
  for every feature set, wasm check, `e2e cpu`.
- In `desktop/`: `cargo test -p studio-engine` (new tests), `cargo build -p studio-ui`,
  `cargo clippy -- -D warnings` on the touched crates.
- Boundary: the kit check prints nothing, and no commit of this phase touches upstream paths.

## Risks

- **Many identity ids** in a frame: core caps them at `MAX_EDITOR_OBJECTS_PER_FRAME`. Only
  `Editable` components carry one.
- **Duplicate identities** (one key shown twice) make the frame metadata invalid, so the docs
  say keys must be unique per scene.
- **The desktop tree has the user's own changes.** If a needed file is one of them, stop and
  ask.

## Todo

- [x] Protocol props types and requests
- [x] Kit `props` module and `#[derive(Props)]`
- [x] `Editable`, `Ui::with_props`, `editable=` tag attribute
- [x] Runtime hooks, `SetProps`, `PropsInfo`, startup and export
- [x] Studio engine props module and controller
- [x] Props panel UI (the manual run in the app was skipped: see the report's Unresolved list)
- [x] Derive on existing components, docs
- [x] End-to-end check with the fixture (preview round trip test and a manual export)

Report: [kit-phase-08-261024-0805.md](../reports/kit-phase-08-261024-0805.md)
