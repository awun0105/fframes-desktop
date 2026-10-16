# Driver and engine half of the composer, the todo panel and replay-free loading

Working copy: `fframes-m5-drv`, branch `feat/studio-m5-driver`, Rust workspace in `desktop/`. Nothing is committed. No UI wiring was done: the panel, composer, todo panel and history list still have to call the API below.

## What the UI agent calls

All types are exported from `studio_engine::agent`.

### Snapshot

`WorkflowSnapshot.composer: ComposerView` (Debug, Clone, PartialEq, Default):

| field | type | meaning |
| --- | --- | --- |
| `commands` | `Vec<CommandInfo { name, description, input_hint: Option<String> }>` | slash commands, display and insertion only; names have no leading `/` |
| `plan` | `Vec<PlanEntry { text, status: PlanStatus }>` | latest plan; `PlanStatus` is `Pending`, `InProgress`, `Completed`, 1:1 with studio-ui `TodoStatus` |
| `steer` | `SteerSupport { None, AtBoundary, Native }` | `Native`: `steer` works; `AtBoundary`: only `send_at_boundary` and `send_now`; `None`: no live turn or no way to redirect |
| `images` | `bool` | the agent takes image prompts (before launch: adapter probe or advertised capability) |
| `files` | `bool` | the agent takes file attachments (before launch: true for ACP only) |
| `turn_live` | `bool` | a turn is running that a follow-up can reach |
| `held` | `Option<String>` | text of the message waiting for the next step boundary |
| `questions` | `Vec<OpenQuestion { reference: PermissionRef, title, items: Vec<QuestionItem { header, question, options: Vec<QuestionOption { label, description }>, multi_select }> }>` | every open question with all items and options |

`RowKind::User { source: UserSource::Followup }` is the new row for a message sent while the agent works. Studio-ui only got a label arm for it (`"You (follow-up)"`) and a `composer: Default::default()` in one test literal.

### `AgentWorkflow` methods

```rust
fn prepare_attachment(&self, AttachmentRequest) -> Result<PreparedAttachment, WorkflowError>   // BLOCKS: file IO, image decode
fn prepare_attachment_with(self: &Arc<Self>, AttachmentRequest, done: impl FnOnce(Result<PreparedAttachment, WorkflowError>) + Send + 'static)  // same, on its own thread
fn submit_with_attachments(&self, brief: &str, scope: Option<TaskScope>, Vec<PreparedAttachment>) -> Result<u64, WorkflowError>
fn steer(&self, text: &str) -> Result<(), WorkflowError>                        // needs steer == Native and turn_live
fn send_at_boundary(&self, text: &str, Vec<PreparedAttachment>) -> Result<(), WorkflowError>   // needs steer != None and turn_live; one held at a time
fn send_now(&self, text: &str, Vec<PreparedAttachment>) -> Result<(), WorkflowError>
fn cancel_held(&self) -> Result<(), WorkflowError>
fn answer_questions(&self, &PermissionRef, Vec<QuestionAnswer { selected: Vec<String>, text: Option<String> }>) -> Result<(), WorkflowError>
```

`AttachmentRequest::File { path }` (project files only), `AttachmentRequest::Staged { token }` (an outside file after `stage_attachment(path)` returned a `StagedAttachment` the user confirmed; the token is single use) or `AttachmentRequest::Image { label, bytes }` (see "Review fixes"). `PreparedAttachment` is opaque (`label()`, `is_image()`, Debug never prints bytes) and only the engine can build one, so the send operations cannot be handed an unchecked file.

Refusals are synchronous `Err(WorkflowError::Refused(reason))` with a reason meant to be shown, so the UI can restore the user's text. The actor checks again when the message arrives and shows a warning row if something changed in between.

Behaviour to know:

- `send_at_boundary` sends at once when no tool call is open; otherwise it is held until the last open tool call closes (shown in `composer.held`). A held message is dropped with a warning row if the turn ends without a clean boundary.
- `send_now` and a released boundary message cancel the turn and prompt again in the same session; the cancelled turn's end starts the follow-up and is never treated as a failed edit.
- `steer` with Codex asked before Codex announced the turn is parked in the driver and delivered when the turn is announced (the call blocks that short time); it is refused with `NoActiveTurn` only if the turn ends first.
- `answer_questions` validates: one answer per item, in order; each answer has a selection or non-empty text; selections must be offered labels; single-select takes at most one; text at most 4096 bytes. A refusal leaves the question open. Nothing reaches the agent until all items are answered.
- `reply_permission(Select)` on a request that has several question items is refused (it used to answer slot 0 only).
- `WorkflowError::Refused(String)` is new.

### Attachment rules (engine)

- Files are sent to ACP agents as resource links. A project file must pass `ProjectPath` and is opened without following links; the link carries the canonical path.
- A file outside the project needs `outside_confirmation == path` (the exact file the user confirmed), an absolute path, and is also opened without following links.
- Always refused, even when confirmed: `.env`, `.env.*`, `*.pem`, `*.key`, and anything under `.git/` or `.ssh/` (case-insensitive component check).
- Images are decoded with limits (8192 px per side, 128 MiB decode allocation, 16 MiB input), transcoded to PNG within a 4 MiB cap, sent through `prompt_with_images`, at most 3 per message (the user's images take the slots before the app's own before-evidence images), and refused with a reason when the agent did not advertise image prompts. At most 16 files per message.
- Attachment bytes and file contents never go into the session store or conversation log; rows show labels only (`Attached: name`).
- Claude Code and Codex: file attachments are refused with a reason (`composer.files == false`); images are accepted.

## What each driver supports

| | ACP | Claude Code | Codex |
| --- | --- | --- | --- |
| commands | `available_commands_update` | init frame `slash_commands` | fixed `compact`, `review` |
| plan | `plan` update -> `AgentEventKind::Plan` | `TodoWrite` -> `Plan` (workflow test) | `turn/plan/updated` -> `Plan` (workflow test) |
| steering | `Native` only if the agent advertises `agentCapabilities._meta.steering` (sent as `_session/steering`), else `AtBoundary` | `Native` | `Native` |
| images | when advertised | yes | yes |
| files | resource links | no | no |
| questions | n/a | `AskUserQuestion`, all items | `item/tool/requestUserInput`, all items |
| replay on load | replayed transcript dropped, latest commands and plan kept | no replay | no replay |

The steer mode in the snapshot comes from `driver.capabilities()` in one place (`publish_composer`). `resume_session` replays nothing.

Driver API additions (`studio_harness::driver`): `DriverCapabilities { deterministic_turn_end, stops_turn_in_place, steering: SteeringMode { None, StepBoundary, TurnBoundary }, images, files }`, `AgentEventKind::AvailableCommands` and `Plan`, trait methods `prompt_with_files`, `steer`, `answer_question(id, Vec<QuestionAnswer>)`, and `QuestionPrompt`, `QuestionItem`, `QuestionOption`, `QuestionAnswer`. Plan and command events are latest-wins in the event queue.

## Files touched

Harness: `Cargo.toml` (image formats png, jpeg, gif, webp), `driver.rs`, `driver/api.rs`, `driver/events.rs`, `driver/runtime/mod.rs`, `shared.rs`, `connection.rs`, `handle.rs`, `claude/{connection,mod}.rs`, `codex/{connection,mod}.rs`, tests `acp-peer.py`, `fake-claude.py`, `acp_v1/{scenarios,services}.rs`, `cross_driver.rs`.

Engine: `agent.rs` (one `composer` registry field, `Refused`, exports, `submit_with_scope`), `agent/model.rs`, new `agent/composer.rs`, `agent/composer_ops.rs`, `agent/attachments.rs`, `agent/actor/composer.rs`, and targeted edits in `actor/{mod,prompt,rows,turn}.rs`. Studio-ui: `agent/cards.rs`, `agent/controls.rs` (one arm, one literal).

Tests: `crates/studio-engine/tests/support/acp-agent.py` (steering extension, replay on load, commands and plan, open tool calls, resource-link evidence), new `app/tests/agent_workflow/composer.rs` (15 workflow tests), attachment unit tests in `attachments.rs`, and the harness driver tests listed in the change set.

## Tests

- Harness: lib 88, acp_v1 51 (+15 review), claude_stream_json 12, codex_app_server 12, cross_driver 9.
- Workflow (`composer.rs`): commands and plan publication and plan cleared at the next task; Claude and Codex commands; load with replay shows each row once and keeps commands and plan; native steer (ACP with the extension, Claude, Codex); no-extension agent refuses `steer` and takes `send_now`; boundary message held until the open tool closes, withdrawn, re-held and released; boundary message immediate with no open tool; all question items required, refused attempts leave the question open, `Select` on a multi-question refused; project file as resource link with content absent from the session files; deny list and outside-file confirmation; image sent as PNG with no bytes in the session files; image refused when not advertised; Claude and Codex refuse files.

## Limitations

- Attachments given at submit time are checked again against the live agent when the first prompt is sent; if refused then (for example the agent changed), they are dropped with a warning row and the text still goes out.
- A file link is read by the agent later, so a file can change between attaching and reading; the engine guarantees the path was valid and unlinked when it was checked, not the content afterwards.
- Before launch, `images` depends on the adapter probe (or advertised capability) and `files` on the wire kind; both are exact once a session is live.
- A held message is dropped when the turn ends without a clean boundary.
- `steer` text carries no attachments (the wires have none in that position).
- The UI is untouched apart from the label arm; no panel shows commands, plan, held messages or the new questions yet.

## Acceptance results

- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `studio-harness`: lib 88, acp_v1 51, acp_v1_review 15, claude_stream_json 12, codex_app_server 13 (includes the early-steer case), cross_driver 9.
- `studio-engine`: all green (lib 215).
- `fframes-studio`: green; `agent_workflow` 81 passed, 2 ignored before the composer additions, composer suite 15 passed.
- `studio-ui --lib`: 466. `studio-ui --test parity agent::`: 10.
- `studio-ui/src/timeline/view.rs` is 1,803 lines; it predates this work and was not touched. Every file I added or edited is under 1,500 lines.
- `acp-agent.py`: on `session/load` it now reassigns `SESSION_ID` to the loaded session, so replayed updates carry the loaded id; the existing restore tests still pass.

Status: DONE
Summary: Driver and engine support for commands, plan, replay-free load, steering and send modes, per-item questions and attachments is in place with harness and workflow tests; UI wiring is left to the next agent.
Concerns/Blockers: none beyond the limitations above.

## Review fixes

Every finding of `phase-02-03-review.md` was addressed. Verification results are at the end of this section.

### Attachments (engine, `agent/attachments.rs`)

- The deny list (`.env*`, `*.pem`, `*.key`, `.git/`, `.ssh/`) is checked on the canonical path, every component, after trailing dots and spaces, `:stream` suffixes, 8.3 short names and junctions are resolved. Windows tests use real spellings, real junctions and real 8.3 aliases (`.ssh\id` has no distinct 8.3 name on this volume, so that one case skips with a stated reason).
- Outside files are two steps: `stage_attachment(path)` returns a `StagedAttachment` (canonical path, label, size) the UI shows for confirmation; `AttachmentRequest::Staged { token }` redeems it once. The engine keeps the path and file identity (volume and index); a changed file, an expired, reused or unknown token is refused. `outside_confirmation` is gone.
- Files are read through one checked handle: no-follow open, link count above 1 refused, size caps, regular file only. An outside file is sent as an embedded resource read from that handle, never as a path the agent could re-resolve. Inside files are linked inside the agent's draft (working copy), never at the real project root; a file absent from the draft is refused.
- Images are always decoded and re-encoded (PNG included), EXIF orientation applied first; decode is bounded.
- Doc comments now state only what holds (see limitations).

### Sending, steering, questions (engine actor and harness)

- A held message is released only when no redirect is pending; `send_now` drops a held message first, with a visible warning, so a later release cannot overwrite the redirect.
- New harness event `AgentEventKind::SteerResult { text, accepted, message }`: exactly one per accepted `AgentDriver::steer` call. `steer` never waits. The follow-up row appears only on an accepted result; a refusal or a steer unconfirmed at turn end warns and puts the text in `ComposerView.unsent`. ACP no longer reports `OptionRejected { option: "steer" }`.
- Codex steer no longer blocks any thread: `Input::Steer` is queued, one steer may be parked until the turn id exists, a parked steer that times out is dropped and reported rejected, and is never delivered later. A second steer while one is parked is rejected.
- On driver retirement (finish, apply, writer spawn) the redirect, held message, open tools, questions and commands are reset and a dropped held message is reported; the plan stays.
- A question with zero items is cancelled at once (ACP via the driver, Claude immediate deny, Codex empty answers). A lost `answer_questions` reply cancels the question and warns instead of stranding the agent.
- The "Attached: ..." text is added to the brief row only after the attachments passed the checks, so it names only what is actually sent; refused attachments produce a warning.
- Slash command names must match `[A-Za-z0-9:_.-]`, at most 64 characters (`CommandInfo::is_valid_name`); others are dropped. `file_uri` handles UNC and non-UTF-8 paths. Chip labels are sanitised.

### Restored history

Replayed user and agent text during `session/load` is dropped before it reaches the transcript. Consumers of `transcript` / `transcript_page` were searched across `desktop/`: only harness tests read them (they pass); no engine, app or UI code relies on replayed text, and the stored conversation log is what the panel shows after a restore. No change needed.

### Tests added or changed

Harness: steer results for ACP, Claude and Codex, a parked Codex steer that is dropped at the request timeout without blocking, command-name validation, embedded resources gated on negotiation, UNC and non-UTF-8 URIs. Engine: 21 attachment tests (spellings, junctions, 8.3, tokens, links, images). Workflow (`app/tests/agent_workflow/composer.rs`): link points into the working copy, staged token single use and secret not stageable, outside file as embedded content, outside file refused and not named without negotiation, `send_now` over a held message, held message dropped at session end, a refused steer returning to the composer. The fake ACP agent gained a `steer-reject` flag file.

### Documented limitations

- Plan updates are not coalesced (kept as is). The workspace `image` feature (`desktop/Cargo.toml`) stays jpeg/gif/webp-limited.
- Claude answers are keyed by question text, not index; an empty or duplicate text is refused.
- `embedded_files` is false until a session is live.
- A project file linked in the working copy may differ from the project content, and is refused if absent from the draft.
- One Codex steer can be parked at a time; an ACP steer unconfirmed at turn end is shown as possibly not delivered.
- Guarantees are bounded by the checks above: the identity check narrows but cannot eliminate a swap between check and read for an outside file path; reading through the checked handle is what closes it for the sent bytes.

### Finding map (review IDs)

| ID | Result |
| --- | --- |
| A1 | Fixed: deny check on the canonical path, every component, with trailing dot/space, `:stream`, 8.3 and junction tests on Windows |
| A2 | Fixed: `stage_attachment` + single-use `Staged { token }`, file identity kept in the engine |
| A3 | Fixed: links point into the agent's draft; verified by the workflow test comparing the link with the agent cwd |
| A4 | Fixed: checked handle, link count > 1 refused, size caps, outside files sent as embedded content from that handle; docs corrected |
| A5 | Fixed: images always decoded and re-encoded, EXIF orientation first |
| A6 | Fixed: UNC and non-UTF-8 `file_uri` handling with unit tests |
| A7 | Fixed: labels stripped of control and format characters |
| C1 | Fixed: held released only with no redirect pending; `send_now` drops held first with a warning (workflow test) |
| C2 | Fixed: `SteerResult`; row only after acceptance; refusal restores text (workflow test with a refusing fake agent) |
| C3 | Fixed: redirect, held, open tools, questions and commands reset on retirement, plan kept (workflow tests; commands return with the next live session) |
| C4 | Fixed: Codex steer never blocks; a parked steer that times out is dropped and never delivered (harness test) |
| C5 | Fixed: "Attached:" added to the brief row only after the attachments pass; refusal warns (workflow test) |
| C6 | Checked: no consumer of transcript pages relies on replayed text (see Restored history). The early display window before the session id is stored still accepts plan and command updates; kept, they are display-only and the id is unknown then |
| C7 | Not changed, documented limitation |
| C8 | Fixed: engine refuses `items.len() != 1` for single replies; zero-item questions are cancelled at once on all three wires; a lost `answer_questions` cancels the question and warns; duplicate/empty Claude question texts refused |
| Other | Workspace `image` features kept (documented); attachment jobs bounded; command names restricted to `[A-Za-z0-9:_.-]`; status-less tools keep a held message until turn end (accepted as the review says) |

### Verification

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p studio-harness -p studio-engine -p fframes-studio --no-fail-fast` passed except three composer workflow tests that expected commands to survive the end of a task. Commands now go with the live session, so those tests were changed (commands are asserted while live and again for a second task after a stop; Claude and Codex command publishing stays covered by `studio-harness/tests/cross_driver.rs`, and the workflow test for the two CLIs is now named for the plan it checks). Afterwards the whole `agent_workflow` binary was re-run: 87 passed, 2 ignored.
- `studio-ui --lib`: 466 passed. `studio-ui --test parity agent::`: 10 passed (whole parity not run).
- No file in the touched crates exceeds 1,500 lines.

Status: DONE
Summary: All review findings are fixed or documented with real Windows tests; steering has a new SteerResult event, Codex steer never blocks, and outside files go through single-use staging and embedded content.
Concerns/Blockers: Slash commands are empty between tasks (they reappear when the next session is live); the UI should expect that. Documented limitations remain as listed above.
