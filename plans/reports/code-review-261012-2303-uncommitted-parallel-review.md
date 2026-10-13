# Parallel review: uncommitted changes on feat/cap-style-studio-ui

Date: 2026-10-12. Scope: about 3,800 uncommitted lines across 45 tracked files plus untracked
`detect.rs`, `adapter_install.rs` and `qualify_writer.rs`. Six read-only `code-reviewer` agents
each verified a disjoint list of edge cases. Spot-checked by the lead: the image size limit and
the `--out` flag handling.

## Verdict

The features work and compile, clippy is clean, and the tests pass once a real `python` is on
PATH. The work is not ready to commit. Two defects block it, several changes hide a problem
instead of fixing its cause, and three plans plus unrelated example edits are mixed in one tree.

## Blockers

1. **Images can kill the driver.** `runtime.rs:59` allows 4 MiB of PNG data, but a message is
   capped at 1 MiB (`runtime.rs:89`, `transport.rs:259`), and base64 adds a third. About 768 KiB
   of image passes validation and then `OversizedOutbound` shuts the whole driver down. Fix: check
   the encoded size against `max_message_bytes` in `prompt_with_images`.
2. **A broken install folder cannot be repaired.** A `<version>/` folder with no marker
   (for example after antivirus quarantine) makes the rename at `adapter_install.rs:218` fail on
   every retry. Fix: move the broken folder aside, and retry briefly on `PermissionDenied`.

## Changes that hide the real problem

- `runtime.rs:1555-1571` turns an adapter error into `EndTurn` by matching English error text.
  Match `errorKind == "incomplete_tool_call"`, apply it to the Claude adapter only, and stop
  treating a turn the adapter marked failed as a clean finish.
- Any update pushes the prompt deadline back, so a turn has no hard upper limit. Use an idle
  timeout plus a cap on the whole turn.
- The Windows escape scan reports "contained" when its own Windows calls fail, gives up without
  a message at its size caps and on ERROR_MORE_DATA, and can count an escape that did not happen
  when a process id is reused. It also records members only when an event arrives, so a long
  silent tool call is never sampled.
- `9da2b55` re-syncs a cached menu list on every render. Remove the cache instead.
- `3fbe519` patches over `failed_step` being derived from a state transition instead of stored in
  `SetupState::Failed`.
- The Vulkan fallback is copied into three example mains instead of one shared helper.

## Correctness

| Area | Defect | Where |
|---|---|---|
| Detection | A deleted adapter shows Ready: the built-in's executable is saved as `node.exe`, then any Node on PATH matches it | `host.rs:389-399`, `provider-profiles.rs:525-531,620-636` |
| Install | Cancelling after npm has already finished still turns the adapter on; leaving the project keeps the pending install | `conversation_panel.rs:1085` |
| Install | npm is not stopped when the app shuts down (it uses its own process tracker) | `conversation_panel.rs:1077` |
| Install | A folder whose modified time is unreadable or in the future counts as stale, so an active install's folder can be deleted | `adapter_install.rs:186-188` |
| Install | Proxy and CA variables are dropped from npm's environment | `process.rs` allowed keys |
| Detection | Only npm's `%dp0%\` shim is understood; pnpm and yarn shims fail with "node is not recognized" | `detect.rs:188` |
| Adapter check | A check still running when the adapter changes reports the old adapter's result as the new one's | `actor.rs:913,1158,1278` |
| Panel | `turned_on` is not cleared on a switch or a failed save, so one row shows another agent's readiness | `conversation_panel.rs:755,2985` |
| Panel | The config menu commits by index, so options that change between paint and click set the wrong value | `conversation_panel.rs:2225` |
| Redaction | A turn that ends in the middle of a secret prefix redacts the next turn's first word | `redact.rs` `finish()` |
| Sessions | `let _ = SessionStore::clear(...)`: if the clear fails, the old chat is offered again on restart | `actor.rs:3217` |
| qualify-writer | `--out` without `--development` is ignored and the installed ledger is written; values that look like flags are taken as values; unknown flags are ignored; a huge `--timeout` panics | `main.rs:69-90`, `qualify_writer.rs:488` |
| qualify-writer | The baseline is sampled after the prompt, so a shell started early fails the scenario | `qualify_writer.rs:465,476` |
| qualify-writer | `check_record` never checks `contained` | `qualification.rs:628-637` |
| qualify-writer | Fixed temp and scratch names with no lock, so two runs at once lose an update | `qualify_writer.rs:183,242,307` |
| Security (out of plan) | A release build reads the SDK manifest from `./target/...` relative to the working folder | `setup_view.rs` `resolve_manifest` |
| Build | `lib.rs` declares `detect` and `adapter_install`, which are untracked; committing it alone breaks the build | `studio-agent-spike/src/lib.rs` |

## Tests

- The test helpers hard-code `python`. On this py-only host 39 of 41 `acp_v1` tests, 7
  `session` tests and `agent_workflow_apply_gate` fail. Resolve `py -3` / `sys.executable` instead.
- `tests/agent_workflow.rs` only builds on Linux, so its two new tests have not run here.
- The 300 ms sleeps before "nothing happened" checks can pass when they should fail. No test covers
  exactly 3 images or 4 MiB, the srvtoolu path end to end, or install over a broken folder.
- Many assertions match UI wording instead of state.

## Scope and hygiene

- Not in any of the three plans: the config menu, New chat, the SDK manifest rework, the
  intro-video copy rewrite (portrait `SECTIONS` half edited, "WHY IT'S FAST" twice, never
  re-rendered).
- `conversation_panel.rs` is now 3,725 lines. Detect, install and turn-on belong in their own
  module, with turn-on as one state enum.
- Keep `Capture*.PNG` and the two example mp4s (about 330 MB) out of git.

## Sound parts

Save tickets, the Node launch path, pinned and marked installs, the atomic and tamper-checked
ledger, the merged end-of-turn stream flush, New chat log archiving, and `BREAKAWAY_OK` limited
to tests.

## Unresolved

- Should installs ship a lockfile so the adapters' dependencies stay fixed?
- Should the out-of-plan features (config menu, New chat, manifest rework) stay on this branch?
