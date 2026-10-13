# Phase 2: Profiles and Setup UI

**Files (own):** `desktop/app/src/conversation_panel/provider-profiles.rs`, the Setup tab view in `desktop/app/src/conversation_panel/`, `host.rs` (detect call off the UI thread), `desktop/app/tests/provider_profiles.rs`.

## Requirements

- Profiles gain `cli_executable` and optional native ACP `args`. Bump the registry schema version and keep forward-compat handling.
- `auth_env_names` is empty by default; Claude and Codex may keep their names as optional hints only.
- Add profiles only for agents whose ACP launch was verified locally (candidates: gemini, opencode, cursor, grok, devin, hermes).
- Setup shows one row per agent: status (Ready, CLI found / adapter missing, Not found), path, **Select**, **Check adapter**. Detect runs on a background thread, never the UI thread.
- Select writes the absolute resolved path into the saved adapter file (owner-only, existing writer).

## Steps

1. Tests: registry round trip, migration from the current schema version, detection result to row state.
2. Implement profile fields and migration.
3. Implement the Detect action, rows and Select, keeping the existing tab order and IME rules.
4. `cargo test -p fframes-studio --test provider_profiles` and clippy.

## Risk

A saved absolute path goes stale after an upgrade. The launch closure is already re-checked at task launch; surface that failure with a next step.
