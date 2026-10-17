# Platform checks

Windows hand checks need the owner's RDP session, because real input cannot be driven from an agent session. Until that session happens, every hand-check row is **pending**. The [hand-check guide](./hand-check-guide.md) walks through them with prepared projects. Rows that local commands can settle were run on 2026-10-17 on Windows Server 2022, on `feat/studio-m6` after phase 5 ([`8b1fcbf`](https://github.com/khanhthanhdev/fframes-desktop/commit/8b1fcbf)) plus the phase 6 changes.

## Roadmap Done-means items on Windows

| # | Done-means item ([roadmap](../../261013-0733-studio-rewrite-roadmap/plan.md#done-means)) | How checked | Result |
| --- | --- | --- | --- |
| 1 | `desktop/` builds on Cap's GPUI fork with Cap's components; `just clippy` and `cargo test` pass; no `.rs` file under `desktop/` exceeds 1,500 lines | `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` in `desktop/` (1,774 passed, 0 failed, 25 ignored real-CLI and platform tests, 104 test binaries, 650 s); 10 parity runs next to the `studio-engine` suite, all passing ([parity under load](./phase-05-flaky-tests.md#parity-under-load-found-during-phase-6)); a line count of every `.rs` file | Pass. `studio-ui/src/timeline/view.rs` had 1,863 lines; its tests moved to `timeline/view/tests.rs`, leaving 1,476. No other file is over 1,450. |
| 2 | Trim, split or reorder a scene in a studio-template project: preview updates without a Rust rebuild, `timeline.json` is written, Ctrl+Z restores it, an export matches the preview | Hand check in the real app | Pending |
| 3 | An agent turn renders markdown, a todo list and a diff with per-file accept and revert, with project tools, over native wires for Claude Code and Codex and ACP for other agents | Hand check in the real app with the installed Claude and Codex CLIs | Pending. See finding 1 below. |
| 4 | Durable writes, crash recovery and writer containment proven by tests | `cargo test --workspace`: the crash-boundary matrices and recovery tests are in `studio-engine/tests`; the Job Object containment and escape tests are in `studio-bootstrap` (`src/process/tests.rs`, `src/process/job_escape_tests.rs`) | Pass, by local tests (CI is not a gate for M6) |
| 5 | No bounds-probe, telemetry or qualification harness code remains; release gates on an eligibility list, package verification and a tag-to-CI-run check; no `*_spike` names | Text search for `spike` under `desktop/` and `.github` (0 files). The release gates are in `.github/workflows/desktop-release.yml`. | Pass for the names. The release dry run with `consumer_release_enabled` is the owner's to run: pending. |
| 6 | `docs/desktop/architecture.md` describes the real crate layout | Its crate table compared with the `members` of `desktop/Cargo.toml` | Pass (12 of 12) |

## Other Windows hand checks

| Check | Result |
| --- | --- |
| Windows title bar (open from M2) | Pending |
| Caption hit-test, title drag, Close hover colour (no automated test since the native harness was removed) | Pending |
| Native file dialogs (open project, choose attachment, export destination) | Pending |
| Ctrl+Enter in the composer | Pending |
| Review-banner layout in the real app | Pending |

## macOS

Not fully tested: CI only, as decided on 2026-10-16. The latest green Desktop CI run with the macOS job is [38965115683](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/38965115683) on `feat/studio-rewrite` (2026-10-16), which predates M6. M6 was not run on CI, by the owner's decision. Hand checks and the real-CLI tests (`cargo test -p studio-harness --test claude_real_cli --test codex_real_cli -- --ignored`) were not run.

## Compatibility-word allow-list

The plan's success criterion: `legacy|migrat|OLDEST_|format 2|dev:ino` (case-insensitive) in `desktop/**/src` hits only this list. Searched on 2026-10-17:

| Hit | Why it stays |
| --- | --- |
| `studio-theme` `gray_*_legacy` tokens and their doc comments | Token names, out of M6 scope |
| `answer_oldest_permission` (`studio-ui` `agent/commands.rs`, `shell/keymap.rs`) | Another sense: the oldest open permission row |
| `studio-sdk/src/manifest.rs` test with a `legacy` field | Refusal test: an unknown `legacy` field is rejected |
| `studio-engine/src/diagnostics.rs`, `store.rs` ("no migration") | Refusal wording: an old or newer file is opened read-only and never migrated |
| `studio-engine/src/edit_transaction/mod.rs` `dev:ino` | Crash recovery: the stage file's identity in the journal |
| `studio-engine/src/controller.rs`, `task_scope.rs` "legacy whole-project" task path | Another sense: a brief submitted with no compiled timeline scope, which the UI still sends (`studio-ui/src/agent/controls.rs`). It is a current entry point, not compatibility code. |

The one stale hit, a `studio-harness/src/driver.rs` comment about the removed handwritten ACP session, was corrected.

## Findings

1. **The built-in Claude and Codex profiles start on ACP, not the native wires.** All four built-in profiles in `studio-engine/src/providers/provider_profiles.rs` use `AgentWire::Acp`, and Install adapter installs the `@agentclientprotocol` packages. The Claude stream-json and Codex app-server wires run only after the user picks them in the Agent page's Protocol control and points the executable at the `claude` or `codex` CLI. The roadmap contract says Claude Code and Codex use native drivers. The docs describe the code as built. Whether the defaults change is an owner decision.

## Unresolved

- Every hand-check row waits for the owner's RDP session.
- The release dry run is for the owner to run.
- Finding 1 needs an owner decision.
