# Phase 2: Install guide links

**Files (own):** `desktop/app/src/conversation_panel/provider-profiles.rs` (`install_guide_url`, `DetectRowAction`), the Detect row actions in `desktop/app/src/conversation_panel.rs`, `desktop/app/tests/provider_profiles.rs`.

## Requirements

1. Add `pub fn install_guide_url(profile_id: &str) -> Option<&'static str>` next to `builtin_cli`, built-in ids only. Candidate pages, found on 2026-10-12. Re-fetch each one before shipping and record the check date in a comment:
   - claude: `https://code.claude.com/docs/en/setup`
   - codex: `https://developers.openai.com/codex/cli`
   - pi: `https://github.com/svkozak/pi-acp` (the README lists pi and Node.js as prerequisites)
   - antigravity: ship only if an official Google page documents the ACP binary. Otherwise return `None`; do not use a third-party registry page.
   - Node.js, for "Install adapter needs Node.js and npm": `https://nodejs.org/en/download`
2. Add a pure `fn row_action(state, installable, tools_present, guide) -> DetectRowAction`, with `DetectRowAction` = `TurnOn | InstallAdapter | GetNode | InstallGuide(url) | None`:
   - `Ready`: TurnOn (the existing Select)
   - `AdapterMissing` with a package and tools: InstallAdapter
   - `AdapterMissing` with a package but no Node or npm: GetNode
   - `AdapterMissing` without a package: InstallGuide when a URL exists
   - `NotFound`: InstallGuide when a URL exists, otherwise None
3. Add `fn open_link(url: &str, cx)`. It opens the URL with `cx.open_url` only when the URL starts with `https://`, and otherwise refuses and says so. Put the check in a pure `is_openable(url) -> bool` so it can be tested.
4. After a guide opens, the row hint reads "After installing, press Detect agents again." Detection is never re-run by itself.

## Steps

1. Tests in `tests/provider_profiles.rs`:
   - `row_action` covers every state combination.
   - `is_openable` refuses `http:`, `file:`, `ms-settings:`, `javascript:`, an empty string and an `https` lookalike with leading whitespace.
   - Every built-in URL passes `is_openable`.
2. Implement items 1 to 4 and render the buttons with the existing `self.button` and `Tone::Normal`.
3. `cargo test -p fframes-studio --test provider_profiles` and `just clippy`.
