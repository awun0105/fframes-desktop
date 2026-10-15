---
phase: 2
title: "SDK release, template and project migration"
status: completed
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
- Projects never fetch from crates.io: `build_materialization.rs` (around `:186-193`) replaces the crates-io source with the bundled SDK's `framework/vendor` directory, which `desktop/scripts/assemble-phase-zero-sdk.py` fills with `cargo vendor`. The fframes crates themselves are not vendored: the script copies them as source to `framework/framework/<crate>` and materialization rewrites the project's dependencies to those paths (`:315-327`). A new SDK is a new bundle, not a crates.io publish.

## Design (verified against the code)

- **One active SDK.** `SdkInstaller` keeps a single `sdk_home/active`; installing a
  new bundle replaces it. Projects pin the exact SDK (`sdk_id` and the SHA-256 of
  `compatibility.json`; checked in `build_materialization.rs:90`, `controller.rs:481`,
  `engine/backend.rs:441`). A new bundle therefore needs to keep serving old pins, or
  every existing project stops building.
- **Two framework trees in one bundle.** `framework/framework` holds the new crates
  (fframes family 1.3.0, runtime and protocol 0.2.0); `framework/framework-legacy`
  holds the seven crates of `CRATES` exported from the last pre-timing commit
  (`96ab9fc`, fframes 1.2.0, runtime and protocol 0.1.0), each tree its own workspace.
  One `cargo vendor --versioned-dirs --sync <legacy worker>` run vendors the
  third-party crates of both; both qualification workers build offline and pass the
  preview probe.
- **Manifest.** `CompatibilityManifest` gains `runtime_version` (runtime and protocol)
  and a `legacy` set: its versions, its tree path, and the `sdk_id`s of the bundles it
  continues. Acceptance matches the release id, not the digest: every assembly of a
  release has different artifact hashes. Schema, `validate()` and the
  hard-coded `"0.1.0"` callers move to these fields.
- **Enforcement.** A project builds when its SDK pin is the current one and its pins
  are the new set with `generated_instruction_version` 2, or its SDK pin is the
  current one or a listed legacy pin and its pins are the legacy set with
  `generated_instruction_version` 1. All three dependencies must carry an exact
  version; mixed, unknown or path-only pins are rejected. The path rewrite points at
  the matching tree.
- **Migration** rewrites the SDK pin with the crate pins, the worker, `main.rs` and
  `generated_instruction_version`, as one undoable edit transaction.
- **Migration record.** A distinct `TransactionKind::Migration` journal record, never
  task history. `studio.json` stays out of the file set (a briefly absent manifest makes
  the project unopenable); the record keeps its raw before and after bytes, the
  controller writes them atomically after the commit, and the next open completes the
  write when the app stopped in between. Undo inverts the file set and restores the
  before bytes, refusing when a touched file or `studio.json` changed since.
- **Fixture pins.** The `pre_m4_project` fixture is pinned to fframes 1.2.0: projects of
  SDK v1 and early v2 (fframes 1.1.0) cannot build against the legacy tree and need the
  migration.

## Tasks

1. Scout the version enforcement, SDK manifest and `assemble-phase-zero-sdk.py` / `build-phase-zero-sdk.*`. Required finding: how unpublished workspace crates enter `framework/vendor`. `cargo vendor` skips path dependencies, and a directory source needs a `.cargo-checksum.json` per crate, so name the mechanism (for example `cargo package`, unpack and generate checksums, or a local registry) before task 2.
2. Bump `fframes`, `fframes-studio-runtime` and `fframes-studio-protocol` versions in the workspace, add them to the bundled vendor set by the mechanism from task 1, and bump the SDK compatibility manifest. The legacy versions (`fframes 1.2.0`, runtime and protocol `0.1.0`) stay in the vendor set. Enforcement in `build_materialization.rs` accepts exactly two pin sets, legacy and new, chosen from the project's `generated_instruction_version` (cross-checked against its `Cargo.toml` pins), and the hard-coded `"0.1.0"` is replaced by values from the manifest. Any other pins are still rejected.
3. New template: three scenes with `editor_instance_key`, `Duration::Auto`, `define_scenes`, `render_scenes`; worker reading `timeline.json` and supporting export options; `main.rs` using `cli::new(..).timing_file(..)`; template agent guide from phase 1.
4. Project migration as a durable transaction with an undo checkpoint: rewrite `Cargo.toml` pins and `src/bin/studio_worker.rs`, add `src/main.rs` timing opt-in, leave the user's `src/lib.rs` scenes untouched, bump `generated_instruction_version`. Offered from a notice when an old project opens; never automatic.
5. Old projects: preview keeps working (the worker lacks the `SetTiming` capability); editing and export options are disabled with "Migrate this project to edit timing".
6. Tests: migrate the `pre_m4_project` fixture (moved with the engine tests in M2 phase 1) and build it; undo restores it; an unmigrated project builds against the legacy pin set, previews and shows the disabled reason; a project with mixed or unknown pins is rejected; a new project has three editable scenes.

## Todo

- [x] Scout enforcement and SDK assembly scripts
- [x] Bundled SDK with new and old vendored versions; manifest bump
- [x] New multi-scene template
- [x] Migration transaction with undo
- [x] Unmigrated projects degrade gracefully
- [x] Tests

## Delivery notes

- Tests, by bullet of task 6:
  - migrate and build `pre_m4_project`: `native-export.rs` (legacy section: builds against
    `framework-legacy`, refuses settings, migrates, builds against the current tree and
    exports at half size with a timing document);
  - undo restores it: `tests/project_migration.rs` (every byte, `studio.json` included;
    refusal after a touched file or the manifest changed; completion on open after an
    interrupted manifest write);
  - an unmigrated project builds and previews: `real_sdk_presets.rs` (legacy project) and
    `native-export.rs`; the disabled reason is `Controller::timing_editing`;
  - mixed, unknown or path-only pins: `materialization.rs`;
  - three editable scenes: `native-export.rs` trims `intro`, `story` and `outro` by key
    and checks the 5 s result.
- Unmigrated projects show a "Migrate project" notice; More actions has "Undo
  migration". The timeline and export controls that read `timing_editing` arrive in
  phases 4, 5 and 7.
- The `pre_m4_project` worker predates export altogether, so the legacy section checks
  only the refusal of settings; no default export of an unmigrated project is tested.
- Windows fixes found on the way: the export flush now opens the MP4 with write access
  (`FlushFileBuffers` refuses read-only handles), and two real-SDK tests build in short
  temporary roots (build-script paths passed the 260-character linker limit).
  `test-preset-relocation.py` assumes Unix tool names; on Windows the Rust half ran
  directly with `SDK_ACTIVE` and `PRESET_RELOCATION_OUT`.
- The SDK assembler exports the legacy tree with `git archive`, so its CI jobs now check
  out full history.
- Verified here with the assembled v3 bundle: `native-export`, `real_sdk_selection`,
  `agent_workflow` (real SDK) and `real_sdk_presets`, plus every non-ignored desktop suite.

## Success criteria

A project created today, migrated, builds and edits timing on this VM; undo of the migration restores the original files byte for byte.
