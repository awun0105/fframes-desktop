# Phase 1: Turn on runs the connection test

**Files (own):** `desktop/app/src/conversation_panel.rs` (`use_detected`, `save_adapter`, `apply_saved`, `select_provider`, `apply_selected_provider`, `install_adapter`, Detect row rendering), `desktop/app/src/conversation_panel/controls.rs` (row status text, the `AuthRequired` step), tests beside them.

## Context

- `use_detected` fills the fields and calls `save_adapter`. The save is asynchronous: resolve on a background thread, then `apply_saved` calls `workflow.set_adapter` (readiness goes to `Unchecked`), then the file is written on a background thread and `saves.finish(ticket)` reports completion.
- `select_provider` follows the same pattern through `apply_selected_provider` and `host::save_registry`.
- `workflow.check_adapter()` sends `Msg::CheckAdapter`. `set_adapter` changes the settings under the lock, then sends `Msg::SettingsChanged` on the same channel, so a check sent after it is processed after it.
- `controls::guidance(readiness)` already turns every `AdapterReadiness`/`AdapterStatus` into a headline and next steps.

## Requirements

1. Add `turn_on: Option<(u64, String)>` to the panel: the save ticket and the profile id that should be tested when that save completes. Only the turn-on paths set it.
2. Split `save_adapter(cx)` into `save_adapter_with(test: Option<String>, cx)`, keeping `save_adapter(cx)` for the Save button, which passes `None`. `use_detected` passes `Some(profile_id)`.
3. `select_provider` sets `turn_on` with its ticket. It does not when the profile is already selected or when a task is busy (the stopped-writer handoff path).
4. In both completions (`apply_saved` and `apply_selected_provider`), after `saves.finish(ticket)` returns true and the write succeeded:
   - take `turn_on` only if its ticket matches;
   - re-derive the controls from the current snapshot and fire `workflow.check_adapter()` only when `controls.check_adapter` is true;
   - otherwise say "Saved. The connection test waits until the running task finishes; use Check adapter then."
   - When the write failed, report the save error and do not test.
5. In the `install_adapter` success branch, insert the installed adapter, then call `use_detected(&id)`, which tests, when the settings are editable and no save is pending. Otherwise say "Adapter installed. Select it to turn it on."
6. Remember `turned_on: Option<String>`, the last profile id a turn-on targeted. In the Detect section, that row's status line shows `guidance(&snapshot.adapter.readiness, true).headline` and its first step instead of the detection label. Clear it on any plain Save or other selection.
7. Rewrite the `AuthRequired` steps so they lead with "Sign in with the agent's own tool (for example run `claude` or `codex` once and log in), then Check adapter again." Keep the environment variable names as an optional alternative.

## Steps

1. Tests first:
   - `controls.rs`: the turned-on row status for `Checking`, `Ready`, `AuthRequired` and `MissingRuntime`.
   - An `agent_workflow` test: `set_adapter(Some(a))` followed by `check_adapter()` probes `a`, not the previous adapter. Use the existing fake-adapter test helpers.
2. Implement items 1 to 7.
3. `cargo test -p fframes-studio controls`, the new workflow test, and `just clippy`.

## Risk

A save that is still pending when the user presses Select is refused by `saves.begin()`. The flag is set only after `begin` succeeds, so no stale test fires.
