---
phase: 1
title: "Remove spike names and old modules"
status: done
priority: P2
effort: "1d"
dependencies: []
---

# Phase 1: Remove spike names and old modules

## Context

Checked against the tree at `df53e56` (2026-10-17). `desktop/` has 82 case-insensitive "spike" hits in 19 files (plans, target and node_modules excluded). Already gone: the `qualify_*`, `presentation_stress`, `stress_metrics` and `qualification::` identifiers, and the whole bounds-probe harness binary (M2 phase 7). `.github` has no "spike" text.

The spike UI is reachable only through the `spike-ui` route. Production replacements for its selection logic already exist: `studio-engine/src/canvas_selection.rs` and `studio-ui/src/canvas_view.rs`.

## Requirements

- No spike name, route, string or comment remains in `desktop/` outside `plans/`.
- `desktop/app/src` holds only `main.rs`, `assets.rs`, a `lib.rs` that exports assets, and the two bins.
- The CI step "Real worker frame, anchor, crash and restart" keeps its name and still runs a real-worker test, because the release gate's full-run check keys on that step name (`desktop-self-hosted.yml:110-112`, `:267-269`; `desktop-release.yml:200-201`).
- Production probes that M2 phase 7 kept stay: `StudioShell::telemetry`, `start_agent_telemetry` (`app/src/main.rs:206`, `studio-ui/src/shell/observation.rs:33`, the writer behind the hidden `--observe-*` flags that `app/tests/x11_shell.rs` uses), `bounds_probe` and `studio-ui/src/probe.rs`, `evidence_preview.rs`, and `qualification.json` as a package file.

## Files

Delete (no caller after the route goes):

| File | Lines | Note |
|------|-------|------|
| `desktop/app/src/app.rs` | 662 | `StudioSpikeApp`, `agent_spike` field, `"rev_spike"`, `"studio-spike"` id, spike window title |
| `desktop/app/src/agent_spike.rs` | 372 | `AgentSpike`, `CandidateReady` |
| `desktop/app/src/selection_spike.rs` | 405 | `SelectionSpike`, `DisplayedSourceFrame`, `SelectedElementInfo`, `SelectionError` |
| `desktop/app/src/preview_element.rs` | 108 | only user of `StudioSpikeApp` besides `main.rs` |
| `desktop/app/src/ui/mod.rs` | 4 | dead re-export of `studio_ui::probe`; nothing references it |

Edit:

- `desktop/app/src/lib.rs` (11 lines): keep `assets` only.
- `desktop/app/src/main.rs` (348 lines): drop `Command::SpikeUi`, `run_spike_ui()` (lines 225-250) and the test `no_command_opens_the_studio_and_the_legacy_spike_flag_still_works`; replace it with a test that no command opens the studio and that `--spike-ui` is now rejected.
- `desktop/app/tests/worker_roundtrip.rs` (83 lines): imports `fframes_studio::selection_spike::DisplayedSourceFrame`. Port it to `WorkerClient` and `studio_engine::canvas_selection` (see steps).
- `desktop/app/Cargo.toml`: remove dependencies only the spike used; check `async-channel`, `smallvec`, `image` and any others by grep.
- `desktop/scripts/package-phase-zero.py:2` (docstring) and `:196` (`CFBundleIdentifier "studio.fframes.spike"`); `desktop/scripts/test-packaging.py:59-61`, `:141` (asserts the packager has no `spike-ui`; reword to the new route state).
- Comments naming the spike: `crates/studio-harness/tests/{fake-codex.py:3,fake-claude.py:3,acp-peer.py:5}`, `crates/studio-engine/src/sdk_setup.rs:89`, `crates/studio-ui/src/setup_view.rs:73`.
- `docs/desktop/README.md:59` and `docs/desktop/design-system.md:3` belong to phase 2.

Keep unchanged: `desktop/app/src/assets.rs` (126, used by `main.rs:168` and `examples/ui_gallery.rs:640`), `desktop/app/src/bin/studio_mcp.rs`, `studio_tools.rs`, `desktop/app/tests/canvas_selection.rs` (uses the studio-ui view state, not the spike).

## Steps

1. Port `worker_roundtrip.rs` first, so the CI step never goes empty. Keep the ignored test `real_worker_frame_anchor_crash_and_restart` and its name. Replace the spike's displayed-frame type with the production frame metadata plus `canvas_selection::hit_test` and `point_to_video`. `WorkerClient::request_elements` (`studio-engine/src/services/worker_client.rs:550`) stays: `studio-engine/tests/managed_worker.rs:36` still uses it. Run it with `cargo test -p fframes-studio --test worker_roundtrip -- --ignored` where a worker exists.
2. Delete the five files, trim `lib.rs`, remove `Command::SpikeUi` and its test from `main.rs`.
3. Remove unused dependencies from `desktop/app/Cargo.toml`; confirm with `cargo build -p fframes-studio` and `cargo +nightly udeps` only if already installed, else by grep.
4. Reword the packaging script strings and the comments listed above. Do not rename the bundle identifier if a release already shipped under it: grep `desktop-release.yml` and the packager tests first; if any test pins it, keep it and say why in a comment.
5. Run the validation greps and tests below.

## Validation

- `grep -rniE "spike" desktop --include=*.rs --include=*.py --include=*.toml --include=*.yml` returns nothing; `grep -rni spike .github` returns nothing.
- `grep -rnE "qualify_|presentation_stress|stress_metrics|qualification::" desktop --include=*.rs` returns nothing.
- `grep -rnE "start_agent_telemetry|bounds_probe" desktop --include=*.rs` returns only the production sites listed under Requirements.
- `cargo test -p fframes-studio`, `cargo test -p studio-engine`, `just clippy`, `cargo fmt --all --check`.
- `python desktop/scripts/test-packaging.py`.
- CI: the self-hosted job's "Real worker frame, anchor, crash and restart" step runs at least one test (not "0 tests").

## Risk

- Medium: the ported worker test loses the `code_snippet` anchor check if production types do not carry it. Mitigation: keep one assertion that a hit-tested element resolves to a source anchor through `request_elements`; if that is impossible, the plan owner decides (see decisions in the readiness report).
- Low: a dependency removed from `app/Cargo.toml` is used by `ui_gallery.rs` or a bin. Mitigation: build with `--all-targets`.
- Low: renaming the bundle identifier breaks packaging tests or upgrade identity. Mitigation: step 4.

## Rollback

Revert the phase commit. Steps 1 and 2 are separate commits, so the port can stay while the deletion is reverted.

## Todo

- [x] Worker test rewritten on the live transport (frame, anchor, crash and restart)
- [x] Spike files, --spike-ui and the harness ACP session deleted
- [x] Unused app dependencies removed (built with `--all-targets`)
- [x] Bundle id is `studio.fframes.app`; packaging test passes
- [x] Stale wording fixed; spike grep empty
- [x] Verification recorded in [the report](./reports/phase-01-spike-removal.md)
