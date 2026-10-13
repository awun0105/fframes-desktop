# Phase 3: Docs, contract text and verification

**Files (own):** `docs/desktop/README.md` (Setup paragraph), `docs/desktop/architecture.md` if it states when an agent may launch, `NO_AGENT_LAUNCHED` in `desktop/app/src/conversation_panel/controls.rs:24`.

## Steps

1. In `NO_AGENT_LAUNCHED`, replace "only Check adapter and tasks do" with "only Select, Install adapter, Check adapter and tasks do; each starts at most one connection test." In `docs/desktop/README.md`, update the Setup paragraph ("Opening a project never installs or launches an agent", "**Select** saves the absolute path it found") and the M6 paragraph ("selecting a profile does not launch it"). Also update `architecture.md` if it states the same rule.
2. Document the following:
   - Select turns an agent on and tests it once.
   - Save alone does not test.
   - Install guide opens the official page, and the user runs Detect agents again afterwards.
   - Sign-in failures keep the selection.
3. On this Windows host, run the manual check:
   - Detect, then Select Claude: the test runs and Status shows the result.
   - Pi and Codex rows show Install guide, which opens the browser.
   - Save alone does not start the adapter.
   Record the results honestly. Signing in to Claude needs the user's own login.
4. Run `just clippy`, `cargo test -p fframes-studio --test provider_profiles`, and the `controls` and workflow tests. Known host-only failures from the previous plan (no `python`, no built worker) are reported as such, not hidden.
5. Before committing, audit the diff against this plan's constraints with the advisor.
