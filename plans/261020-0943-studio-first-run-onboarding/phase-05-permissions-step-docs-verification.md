---
phase: 5
title: "Permissions step, settings, panel indicator, docs and verification"
status: pending
effort: "1 day"
depends_on: [2, 4]
---

# Phase 5: Permissions step, settings, panel indicator, docs and verification

## Permissions step

Three choices, each with one plain sentence (use the wording below):

- **Ask every time.** "The agent stops and asks before every action."
- **Allow in the working copy** *(preselected, pending the owner's answer in plan.md)*. "The
  agent reads and edits files in its private copy of your project without asking. Running
  commands or using the network still asks. You review or undo every change."
- **Allow everything.** "The agent also runs commands, such as `cargo check`, and uses the
  network without asking. Commands run on this computer and are not sandboxed."

Next saves `permission_policy`. The step also says: "Change this anytime in Settings → Agent."

## Elsewhere

- `shell/settings.rs`: an Agent → Permissions row with the same three options (shared component).
  It is disabled while a task runs, with the reason, because the policy is frozen per task (phase 2).
- Agent column header (`agent/mod.rs` header next to the provider line): a small badge showing the
  active policy ("Asks every time" / "Auto: working copy" / "Auto: everything", warning tone for
  everything). Clicking it opens the Settings row.
- `docs/desktop/README.md`: replace "Studio never answers one for you" with the policy rule and
  the table from plan.md. `docs/desktop/architecture.md`: one paragraph on `auto_answer`, where it
  runs and why `allow_always` is never chosen. `docs/desktop/research.md:51` stays as history,
  with a dated note that the user-chosen policy supersedes it.

## Verification

- Full suites: `cargo fmt --all`, `cargo clippy --manifest-path desktop/Cargo.toml --all-targets -- -D warnings`,
  `cargo test --manifest-path desktop/Cargo.toml -p studio-proto -p studio-harness -p studio-engine -p studio-ui -p fframes-studio`.
- Real app, clean app data: launch `fframes-studio studio` with `LOCALAPPDATA` pointed at an empty
  folder and walk every step in light and dark. Capture each state with `PrintWindow` (screen
  capture is unavailable from the agent's session on the Windows runner) and inspect it.
- Real agent, end to end: with Claude via `claude-agent-acp`, choose "Allow in the working copy"
  and ask for a title change. Expected: no card for the edit, a card for the `cargo check` the
  agent runs, and an "Allowed automatically" row in the transcript.

## Done when

- All acceptance criteria in plan.md hold. The captures are attached to the PR or delivery
  message, and the docs match the shipped behaviour.
