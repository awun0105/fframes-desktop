---
phase: 2
title: "SDK release, template and project migration"
status: pending
priority: P1
effort: "4-6d"
dependencies: [1]
---

# Phase 2: SDK release, template and project migration

## Goal

New projects and migrated existing projects use an SDK with the timing hook and export options, start from a multi-scene template, and keep working if not migrated.

## Context

- Template pins: `desktop/crates/studio-project/templates/Cargo.toml` (`fframes =1.2.0`, runtime and protocol `=0.1.0`); copied at creation by `studio-project/src/lifecycle.rs`.
- Enforcement: `build_materialization.rs` rejects other versions and hard-codes `"0.1.0"` for runtime and protocol.
- SDK compatibility: `studio-sdk/src/manifest.rs` (`fframes_version`), `generated_instruction_version` in the project manifest.
- Projects never fetch from crates.io: `build_materialization.rs` (around `:186-193`) replaces the crates-io source with the bundled SDK's `framework/vendor` directory, which `desktop/scripts/assemble-phase-zero-sdk.py` fills with `cargo vendor`. A new SDK is a new bundled vendor set, not a crates.io publish.

## Tasks

1. Scout the version enforcement, SDK manifest and `assemble-phase-zero-sdk.py` / `build-phase-zero-sdk.*`. Required finding: how unpublished workspace crates enter `framework/vendor`. `cargo vendor` skips path dependencies, and a directory source needs a `.cargo-checksum.json` per crate, so name the mechanism (for example `cargo package`, unpack and generate checksums, or a local registry) before task 2.
2. Bump `fframes`, `fframes-studio-runtime` and `fframes-studio-protocol` versions in the workspace, add them to the bundled vendor set by the mechanism from task 1, and bump the SDK compatibility manifest. The legacy versions (`fframes 1.2.0`, runtime and protocol `0.1.0`) stay in the vendor set. Enforcement in `build_materialization.rs` accepts exactly two pin sets, legacy and new, chosen from the project's `generated_instruction_version` (cross-checked against its `Cargo.toml` pins), and the hard-coded `"0.1.0"` is replaced by values from the manifest. Any other pins are still rejected.
3. New template: three scenes with `editor_instance_key`, `Duration::Auto`, `define_scenes`, `render_scenes`; worker reading `timeline.json` and supporting export options; `main.rs` using `cli::new(..).timing_file(..)`; template agent guide from phase 1.
4. Project migration as a durable transaction with an undo checkpoint: rewrite `Cargo.toml` pins and `src/bin/studio_worker.rs`, add `src/main.rs` timing opt-in, leave the user's `src/lib.rs` scenes untouched, bump `generated_instruction_version`. Offered from a notice when an old project opens; never automatic.
5. Old projects: preview keeps working (the worker lacks the `SetTiming` capability); editing and export options are disabled with "Migrate this project to edit timing".
6. Tests: migrate the `pre_m4_project` fixture (moved with the engine tests in M2 phase 1) and build it; undo restores it; an unmigrated project builds against the legacy pin set, previews and shows the disabled reason; a project with mixed or unknown pins is rejected; a new project has three editable scenes.

## Todo

- [ ] Scout enforcement and SDK assembly scripts
- [ ] Bundled SDK with new and old vendored versions; manifest bump
- [ ] New multi-scene template
- [ ] Migration transaction with undo
- [ ] Unmigrated projects degrade gracefully
- [ ] Tests

## Success criteria

A project created today, migrated, builds and edits timing on this VM; undo of the migration restores the original files byte for byte.
