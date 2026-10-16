# Wiring the agent UX components into the agent panel

Branch `feat/studio-m5-ux`, worktree `fframes-m5`. The composer, todo panel, question card, history list and
keymap are now live in `ConversationPanel`, driven by the engine API and proven against the fake agents.

## What was wired

- Composer (the ported `ComposerInput`): send, stop, send-now, attach file/image, slash menu fed by the
  agent's advertised commands, held and steer messages, refused-message "Put back", staged attachments with the
  engine's refusal reasons and the outside-project confirmation. Code: `agent/composer_host.rs`,
  `agent/attachment_flow.rs`.
- Todo panel: shows the agent's plan (`PlanEntry` mapped by `From` impls in `todo_panel.rs`).
- Question card: one card per open question, per-item options plus free text, "Send answers" and "Cancel"
  (`agent/question_card.rs`). The permission card for a question reference now says "Answer in the question card".
- History tab: stored sessions listed off the UI thread, search, reopen, new session (`agent/history_host.rs`).
  Opening or starting a session returns to Chat and focuses the composer.
- Keymap: `composer::keymap::init` is bound from `shell/keymap.rs`; workspace shortcuts stay out of the
  composer (`Workspace && !TextInput && !AgentComposer`).
- Telemetry: panel JSON gained `tab`, `composer.commands`, and bounds for every new control, through a
  `BoundsSink` in `probe.rs` and probes in the composer and history components.

## API mismatches found and fixes

- Engine send operations consume the attachment `Vec`; a refused send lost the chips. `PreparedAttachment`
  (and its inner enum) now derive `Clone`, so the panel keeps its copy and restores it on refusal.
- Clicking a history row or "New session" removes the clicked element, leaving focus dangling, so typing after
  it was lost. The panel focuses the composer when the history event succeeds.
- The slash menu opened inside backticks; it no longer does. Attach kinds the agent cannot take no longer
  leave disabled chips; a blocked attach reports `AttachBlocked` instead.
- Found in the real window: a question card with several items plus the composer was taller than a small
  window and pushed the composer out of view with the card clipped and unreachable. Each card now sits in a
  shrinkable scroll box (the dock may shrink, the composer keeps its height), so the composer stays visible and the card scrolls only when the window is too short.

## Tests

- `tests/parity/composer.rs`, 11 tests that drive the real shell, engine and fake ACP / Claude agents:
  slash commands listed and inserted; a JPEG reaches the agent as PNG; a secret file is refused with the engine's
  reason; a file outside the project needs confirmation; a message is held until the open tool call ends and can be
  withdrawn; a steer reaches the running turn; send-now cancels and resends; the plan shows as a todo list; history
  lists, searches and reopens; a two-item question is answered from its card.
- `studio-ui` lib tests updated and extended in `composer/tests.rs` (473 pass).

## Real-click visual check (real app, fake agents)

The RDP session was disconnected, so `CopyFromScreen` was unavailable: screenshots use `PrintWindow`, and clicks,
characters and wheel turns are posted window messages at the bounds the panel reports. A throwaway example (not
committed, deleted) opened the real studio window on a fixture project. The window is 1275x800, so the agent panel is
short (about 250px of chat); the layout fix above was found there.

| Screenshot | Shows |
| --- | --- |
| `assets/01-idle-composer.png` | typed brief, attach buttons, Send |
| `assets/02-todo-and-review.png` | todo 1/3 after Send, stop note, queue composer while the candidate waits for review |
| `assets/03-slash-menu.png` | typing `/` lists `/compact` and `/review` |
| `assets/04-history.png` | History tab: search, New session, the session marked Current / In review |
| `assets/05-question-card.png` | question card shrunk above a fully visible composer (Claude agent) |
| `assets/07-question-answered.png` | card scrolled to the sizes item, answers chosen, "Send answers" enabled |
| `assets/08-after-answer.png` | agent resumed, edit accepted, preview rebuilt |
| `assets/09-maximized-chat-with-candidate.png` | maximized window: candidate card (Apply / Discard / Review changes), validation, todo, composer |
| `assets/10-review-viewer.png` | "Review changes" opened the viewer with the candidate diff |
| `assets/11-history-two-sessions.png` | History after New session: two stored sessions |

Results (each confirmed from telemetry, not only from the picture):

- Review policy set from the Project tab (`ManualReview`); the turn stopped at "Waiting for your review".
- Slash menu listed the agent's commands; clicking `/review` inserted it (8 chars).
- Question card: `selected` went `0,0` -> `1,0` (Blue) -> `1,1` (S) -> `1,2` (M); the agent's `question.json` received
  `Blue` and `["S","M"]`; phase ended Accepted with no open question.
- Review viewer opened from the banner; Apply ended with Accepted.
- "New session" while a candidate waits is refused with "Apply or Discard the candidate before starting a new chat" (engine
  rule, shown as a notice). After Apply, New session opened an empty chat with the composer focused (5 typed chars
  landed), History listed both sessions, and clicking the older row reopened its 6 rows with the draft intact.
- In the short, unmaximized window the review card sits above the dock and is squeezed out when a todo list is
  showing (screenshot 02); maximizing shows it (screenshot 09). The todo panel can also be collapsed.

## Checks

Run after the final code change, from `desktop/`:

- `cargo fmt --all -- --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test -p studio-ui --lib`: 473 passed
- `cargo test -p studio-ui --test parity --no-fail-fast`: 77 passed (11 composer tests)
- `cargo test -p studio-engine -p fframes-studio --no-fail-fast`: all passed
## Limitations

- Native file dialogs cannot be driven by posted messages, so file picking is covered by the parity tests only.
- Send-now was exercised through its button; the ctrl-enter chord cannot be posted.
- In a short window with a todo list, the candidate review card can be pushed out of view; the todo panel collapses and a larger window shows everything.
- Intermittent, not related to this change: `services::cancelling_an_export_leaves_no_file`,
  `services::an_unmigrated_project_exports_only_the_default_whatever_was_saved`, and once
  `agent::undo_in_the_prompt_undoes_the_typing_there` (preview build timed out under load; passes alone).
- Earlier remote runs on this branch failed in CI: `codex_wire::capture_waits_for_a_command_that_outlives_its_turn`
  panicked at `workflow_world.rs:371` on the self-hosted validation; it is a timing flake outside the files touched.
