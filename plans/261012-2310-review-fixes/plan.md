---
title: "Fix the findings of the uncommitted-work review"
status: completed
---

# Review fixes

Source: `plans/reports/code-review-261012-2303-uncommitted-parallel-review.md`.
Snapshot of the starting tree: scratchpad `snapshot-2310/` (tracked.patch, untracked.tar).

## Outcome

Every confirmed finding is fixed at its cause, the same problem is solved the same way
everywhere in `desktop/`, and all touched crates pass check, clippy and tests on this Windows
host (which has `py` but no `python`).

## Decisions

- Keep the out-of-plan features (config menu, New chat, SDK manifest rework).
- No adapter lockfile.
- Turn time: an idle deadline that only real agent activity extends (message chunks, tool
  calls and their updates, plan updates, permission requests; not `available_commands_update`
  or `session_info_update`), plus a hard per-turn cap, `max_turn`, default 2 hours. Docs updated.
- Example copy: fix only clear mistakes; do not invent new wording.
- `crash_adapter_for_qualification` stays public: the shipped `qualify-writer` needs it.

## Locked contracts

1. `studio-bootstrap` `ProcessTree::escaped_descendants(&self) -> EscapeScan` with
   `pub enum EscapeScan { Clean, Escaped(Vec<u32>), Unverifiable(String) }` (re-exported).
   The driver treats `Escaped` as today. `Unverifiable` stores the reason; ownership is demoted
   to `Detached` like an escape, and `DriverOutcome` gains `pub containment_unverified:
   Option<String>`.
2. `observed_descendants()` returns `None` when member recording was incomplete (a cap was hit,
   or a query failed).
3. `install_adapter` signature and `InstallError` are unchanged.
4. Test Python: new dev-only crate `desktop/crates/studio-test-support`, `pub fn python() ->
   &'static std::path::Path`, resolved once in this order: env `PYTHON`, then
   `$pythonLocation/python.exe` (Windows) or `$pythonLocation/bin/python3`, then the
   `sys.executable` reported by `py -3`, then `python3`, then `python`. Each must answer
   `-c "import sys"`. Used only as a dev-dependency. No production crate gets a test helper.

## Ownership

Wave 1 (libraries, parallel):
- containment: `desktop/crates/studio-bootstrap/**`
- driver: `studio-agent-spike/src/{driver/**,redact.rs,lib.rs,session.rs}`, its `tests/**`,
  new `desktop/crates/studio-test-support/**`, `desktop/Cargo.toml` (workspace members only)
- install: `studio-agent-spike/src/{detect.rs,adapter_install.rs,discovery.rs}`
- examples: `examples/fframes-studio-intro/**`

Wave 2 (app, after wave 1 compiles):
- panel: `desktop/app/src/conversation_panel.rs`, `conversation_panel/{controls,host,provider-profiles}.rs`,
  new panel submodules, `desktop/app/tests/provider_profiles.rs`
- app core: everything else under `desktop/app/**`

Docs (`docs/desktop/**`) are updated by the lead after both waves.

Rules for every agent: edit only owned files; run `cargo check`/`clippy`/`test` for your own
crate only; report a break in a file you do not own instead of fixing it.

## Gates

- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` in `desktop/`
- tests of every touched crate, using `studio-test-support::python()`
- `cargo clippy` for `fframes-studio-intro`
- diff audit against this plan, then ask the user about one commit per plan
