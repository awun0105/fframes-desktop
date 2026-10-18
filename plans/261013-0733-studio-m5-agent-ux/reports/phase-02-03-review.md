# Review: composer driver and engine half (phase 02 and 03)

Scope: `driver-engine.diff` against the current tree in `fframes-m5-drv/desktop`. Read-only review. Neither cargo nor git was run, because Bash is unavailable. Every finding below comes from reading code. Windows filesystem behaviour is stated from Win32 and NTFS semantics, not from a run on this host. Paths are relative to `desktop/crates/`.

Findings are ranked. Priority 1 is attachments (A-), priority 2 is correctness (C-), then lower-severity items.

## High

### A1. Windows deny-list bypass for outside files, and 8.3 short names for any file
`studio-engine/src/agent/attachments.rs:109-122, 208-209, 224-226, 254`

`denied_component` is a plain string test on the final component, and `name` is never canonicalised. Inside the project, `ProjectPath` already rejects `:`, trailing `.` and trailing space, and `\`. The outside branch has no such guard. The outside branch checks only `check_denied(requested)` and `check_denied(&resolved)`, and `resolved` is `canonicalize(parent).join(name)`.

Win32 path normalisation then reaches the denied file under these spellings, none of which `denied_component` recognises:
- `C:\x\secret.key ` or `C:\x\secret.key.` (trailing space or dot is stripped by the OS). Neither ends with `.key`.
- `C:\x\.env ` (trailing space). Note that `.env.` happens to be caught by the `.env.` prefix test.
- `C:\x\.env::$DATA`, `C:\x\.env:s`, `C:\x\id.key:s`, `C:\x\a.pem::$DATA`. These are alternate data streams. The name is neither `.env`, nor does it start with `.env.`, nor does it end with `.key` or `.pem`.
- 8.3 short names of the final component, in the outside branch and the inside branch alike. `.env` has the short name `ENV~1`. `.env.production` has `ENV~1.PRO`. Parent components are expanded by `canonicalize`, so `GIT~1\config` is caught, but `name` is joined raw. The inside branch passes `ENV~1` through `ProjectPath`, which has no rule against `~`.

Failure scenario: the user, or a compromised UI, submits `File{path: C:\proj\ENV~1}` and gets a resource link to `.env`. For outside files, `File{path: "C:\\Users\\me\\deploy.key ", outside_confirmation: same}` is attached although the doc says "cannot be confirmed past". The unit tests cover only ASCII case.

Fix:
1. After the no-follow checks, canonicalise the full path, which expands short names and drops stream and trailing-dot forms. Re-run `check_denied` on every component of the result, in both branches.
2. In `denied_component`, trim trailing `.` and space and cut at the first `:` before matching.
3. Refuse any component containing `:` outside the drive prefix.
4. Add Windows-gated tests for `X.key `, `X.key::$DATA`, `ENV~1`, and a junction to `.ssh`.

### A2. Outside-project "confirmation" is caller-supplied and binds a spelling, not a file
`attachments.rs:41-49, 254-265`; `composer_ops.rs` `prepare_attachment`

The engine takes `outside_confirmation: Option<PathBuf>` from the same request and compares it to `requested` with `==`. Under the stated threat model (UI untrusted, the engine decides what leaves the machine), this is not a gate. Any caller passes the same path twice. Three consequences:
- Nothing records that a human ever saw a prompt. The engine cannot tell a real click from a forged call.
- The confirmation is bound to the requested spelling. A path such as `C:\proj\sub\notes.txt` is judged outside because `sub` is a junction or symlink to `C:\Users\me\Documents`. The dialog presumably shows the spelling the user dropped, but the file that leaves the machine is the canonical `resolved` target. The user confirms one file and a different one is sent.
- The confirmation is never tied to a file identity (volume and file index, or dev and inode, plus size and mtime). A path that is re-pointed between confirmation and `prepare` is accepted, because only the spelling is compared.

Fix:
- Split the operation. `AgentWorkflow::stage_outside_attachment(path) -> StagedOutside { canonical, identity, token }` runs the full check and returns the canonical path for the dialog to display.
- `prepare` then accepts only a token minted by the engine and not yet used (one-shot, short TTL, bound to canonical path and file identity). It re-verifies the identity on use.
- If the UI really is untrusted, minting the token still needs a path the UI cannot self-complete. The honest minimum is: display the canonical path, bind to it, make the confirmation one-shot, and say in the docs that this defends against accidents and not against a hostile UI.

### A3. Resource links point at the real project root, not the agent's draft working copy (verify)
`attachments.rs:249-252` (`root_canonical.join(relative)`); `composer_ops.rs:prepare_attachment` (`controller.project.root`); `actor/turn.rs:507` (`cwd: active.ctx.draft.clone()`)

The agent session runs with `cwd = ctx.draft`. The link built for an inside-project file is `<project root>/<relative>`, so the agent is handed a path outside its own working copy. Effects:
- It leaks the absolute real-root path, including the Windows user directory, to the agent and to whichever provider the agent talks to.
- It points at the stable file, not the draft. After the agent has edited `src/a.rs`, "attach `src/a.rs`" delivers the stale version.
- If the agent is confined to the draft by an OS sandbox or the tool allowlist, the link is either unreadable or a way to read outside the sandbox. In both cases the result is not what the user expects.

I did not confirm how `ctx.draft` relates to `project.root`. Confirm it. If the draft is a copy, map the link into the draft (`ctx.draft.join(relative)`), or inline small text files as content.

### C1. A held message overwrites a pending send-now redirect: the message is lost silently and the order is inverted
`studio-engine/src/agent/actor/composer.rs` (diff lines 929-948, 1020-1069): `composer_track_tool`, `start_redirect`, `turn_accepts_followup`

`turn_accepts_followup` checks only `redirect.is_none()`. It ignores `held`. Sequence:
1. `send_at_boundary(A)` with a tool open. `held = A`.
2. `send_now(B)`. `start_redirect(B)` calls `cancel_turn()` and sets `redirect = B`.
3. The cancelled tool reports `Failed` or `Completed`. `composer_track_tool` sees `open_tools` empty and takes `held` (A).
4. `start_redirect(A)` runs. It cancels again and overwrites `redirect = A`.

B is dropped with no warning. A is delivered, although the user sent B last and wanted it now. `composer_turn_finished` does handle "redirect and held both present" correctly, with a warning. The track-tool path does not.

Fix: in `composer_track_tool`, release `held` only when `redirect.is_none()`. In `composer_send_now`, drop any `held` first and warn that it was replaced. Add a test of the sequence above.

## Medium

### A4. The "no-follow open" is advisory: the handle is discarded, and the agent re-opens by path
`attachments.rs:17-19, 243-252, 266`

Both branches open the file, check it, and drop the handle. The agent then opens `file://<path>` later and follows links freely. That window is documented, but the module docs and the spec ("opened without following links") overstate it. Everything the engine enforces is a check at attach time.
- If `src/a.txt` is swapped for a symlink to `.env` or `~/.ssh/id_rsa`, or a parent directory is swapped for a junction, after attach, the agent reads it.
- A hard link is not detected. `ln .env notes.txt` inside the project (or `mklink /H`) passes both the deny list and the link checks. Check `nlink > 1` (Windows `nNumberOfLinks`) and refuse, or compare the file identity to a deny-list identity set.
- File size is unbounded: there is no cap on a linked file, so a resource link to a 50 GB file is accepted. Add a cap such as 10 MiB, checked on the open handle, plus a regular-file check.
- On Windows, the project-path open (`ProjectPath::open_file`) is path-based, not handle-based. Intermediate components can be raced between `resolve_existing` and the open. That is existing project-crate behaviour, but the new code relies on it as if it were handle-relative.

Fix: either be honest in the docs and spec ("checked at attach time; the agent opens the path later") and cap the size, or read the bytes in the engine and send content. Reading bytes keeps the checks at the same moment as the read.

### A5. PNG passthrough keeps all metadata and trailing bytes
`attachments.rs:304-306`

The module doc says images are "decoded, re-encoded as PNG when not one already". A PNG input is forwarded byte for byte after decoding only to validate it. That includes `eXIf`, `tEXt`, `iTXt` and `iCCP` chunks (GPS, device names, and prompt-injection text), and any bytes after `IEND`. The 16 MiB input cap and 4 MiB output cap do apply.

Fix: always re-encode (`decoded.write_to(PNG)`) so ancillary chunks and trailing data are stripped. Apply EXIF orientation first for JPEG, otherwise rotated photos lose orientation on transcode. Dimension, alloc and size caps are otherwise sound (`max_alloc` 128 MiB, 8192 per side, 16 MiB in, 4 MiB out), and attachment bytes reach no persisted row.

### C2. Failures after the synchronous check lose the user's text
`actor/composer.rs` `prepare_followup`, `composer_steer`, `start_redirect`, `composer_turn_finished`; `actor/prompt.rs:529-535`

The operations return `Ok(())` once the message is queued to the actor, and the UI clears the composer. Later failures are only a warn row: "not working on a turn", "not delivered", "held message not sent", "session ended", "prompt rejected" (for example Claude `refuse_while_background`). For a redirect, the turn is already cancelled when the follow-up is refused, so the user has lost both the turn and the text.

ACP steer is the worst case. `driver.steer` only enqueues `Command::Steer`, and `composer_steer` immediately pushes a `User/Followup` row. If the adapter then rejects `_session/steering`, `OptionRejected{option:"steer"}` is turned into "The agent refused the option change `steer`". The row still says the message was sent, and the text is gone. For ACP, the steer also carries no turn id, so a steer racing a turn end can reach the next turn.

Fix:
- Publish an `unsent: Option<String>` (or restore text) in `ComposerView`, and have every warn path above set it.
- Push the follow-up row only after the adapter confirms the steer. If the `steer` rejection is handled specially, mark the row as undelivered.
- For ACP, make the steer carry the expected turn id if the extension allows it.

### C3. Stale held, redirect and commands survive driver retirement
`actor/composer.rs:Followup.epoch`, `ComposerRun`; `finish.rs:178`, `apply.rs:31`, `turn.rs:544` bump `driver_epoch` but nothing resets `ComposerRun`

`redirect` and `held` are checked against the epoch only when a turn ends. If a task ends or fails with `redirect` set (the driver is torn down, so no `PromptFinished` consumes it), the old `redirect` stays `Some`. In the next session, `turn_accepts_followup` returns false for the whole first turn because `redirect.is_none()` fails, so `SteerSupport` is `None` and send-now and steer are refused. When that turn ends, the stale follow-up is dropped with a misleading "agent session ended" warning. `commands` also survive a driver or adapter switch until the new agent sends its own list.

Fix: clear `held`, `redirect`, `open_tools`, `questions` and `commands` where `driver_epoch` is bumped.

### C4. Codex parked steer: the actor thread can block for 30 s, and the steer can arrive after it was reported failed
`studio-harness/src/driver/runtime/codex/connection.rs:296-311, 780-783`; `codex/mod.rs:278-302`; `actor/composer.rs:composer_steer`

`CodexDriver::steer` blocks on `recv_timeout(request_timeout)` (30 s). `composer_steer` calls it on the actor thread. Parking removes the early `NoActiveTurn` refusal for a turn that has not started, so the actor thread now waits until `turn/started` arrives (or 30 s). Nothing else in the actor (Stop, event pumping, other messages) runs meanwhile.

If the wait times out, the caller reports "Codex did not answer the steer in time". The parked steer is still in `parked_steer`. It is sent when `turn/started` finally arrives, so the message is delivered after the user was told it failed. A resend then delivers it twice.

Fix: do not park. Return `NoActiveTurn` (as before) or `Busy` when the turn has not started. If parking stays, make it cancellable: on timeout send a withdraw input, or clear `parked_steer` when the receiver is gone (`reply.try_send` fails).

## Lower

### A6. `file_uri` is wrong for UNC paths
`handle.rs` `file_uri` (diff 3153-3174)

`\\?\UNC\srv\share\f` becomes `file:///UNC/srv/share/f`. `\\srv\share\f` becomes `file:////srv/share/f`. Both are broken links, and the first also leaks a malformed host. A non-UTF-8 path is lossily converted. A UNC outside file is quite possible, because the outside branch accepts any canonical path. The fix is to map `UNC\srv\share` to `file://srv/share/...`, or refuse UNC paths.

### A7. Label handling
`attachments.rs:274-280`; `actor/composer.rs:attached_text`

Image labels filter control characters but not bidirectional or format characters (U+202E and similar), so a label can spoof the row. File labels from `to_string_lossy` are not filtered at all, and a Unix file name can hold a newline. Labels go only to the conversation row (follow-ups send `text` without labels), so this is low impact. Strip `char::is_control` and Cf characters for every label.

### C5. First-prompt attachments dropped, but the row says "Attached:"
`actor/turn.rs` (`with_attachment_labels` at task start); `actor/composer.rs:take_first_attachments`

The conversation row is built with `Attached: x` before the first prompt is sent. If `checked_attachments` then refuses (capability, image budget), the attachments are dropped and a warning is shown, but the user row still lists them. `first_attachments` is also taken with `mem::take`, so a resent first prompt after a session restart carries nothing. Build the row text after the check, or mark dropped labels.

### C6. Restore and replay behaviour
`studio-harness/.../shared.rs` (`on_message`, `on_update`, `emit_display_state`); `connection.rs` (removed `finish_streams_at_turn_end(true)`)

- Replayed user and agent text is now dropped before `release_text`, so it no longer reaches `transcript` either. The old comment said replay was "available through transcript pages". If any consumer (inspect or transcript pages) relied on that, it is gone. Confirm that none does.
- Replayed tool calls and updates are now skipped. Good.
- A replayed plan is kept and emitted, so a resumed session shows its old plan with no active task, until `begin_task` clears it. That is probably intended.
- `early_display` accepts a Plan or AvailableCommands update for any `session_id` while `session` is unset, so a wrong session id is not a protocol violation in that window. It is display-only, but the identity check is skipped. Compare with the id from the `session/new` response once known, or buffer.
- No kept transcript text was found. No held-secret-prefix carries over, because replay no longer enters the streams.

### C7. Event-queue coalescing reorders
`events.rs` (diff 2816-2829)

A new Plan or AvailableCommands replaces the latest unread event of the same kind in place, at its old position. `[Plan1, PromptFinished, Plan2]` becomes `[Plan2, PromptFinished]`, so Plan2 is applied before the turn end. This is display-only. It matters because `begin_task` clears the plan: if the task boundary falls between the two, the plan can show for the wrong task. Remove the old one and append the new one.

### C8. Small question issues
- `agent.rs` Select guard: an item count of 0 passes `q.items.len() > 1`, `registry.open` is removed, then the actor falls to the "several questions" arm and warns. The question can no longer be answered. Refuse `items.len() != 1` in the engine.
- The Claude `answers_line` keys answers by question text. Two items with identical text collide and one answer is lost. The engine's count check passes.
- Verified: a question cannot be answered with a default the user did not give. `answer_questions` requires exactly one answer per item, each with a selection or non-blank text, labels from the item's own options, a single choice unless `multi_select`, and text of at most 4096 bytes. The old zero-padding path is gone. Drivers do not re-validate, so `answer_question` must stay reachable only through the engine.
- A refused or lost `Msg::AnswerQuestions` after `registry.open.remove` leaves the agent blocked until Stop. The actor only warns.

### Other
- `Cargo.toml`: `image` gains jpeg, gif and webp workspace-wide, so every crate that uses `image` links the extra decoders for untrusted input.
- `prepare_attachment_with` spawns an unbounded thread per call. Each decode may allocate 128 MiB. Cap concurrency at 1 or 2.
- A slash-command name from the agent is passed through the redactor only, not stripped of whitespace or control characters. The UI inserts `/name` into the composer, so a name with a newline can inject lines. Restrict names to `[A-Za-z0-9:_.-]`.
- A tool that reports only `status: None` after its first event never closes, so a held message waits until the turn ends. It is then sent as a clean follow-up, which is acceptable.

## Checked and found sound
- Case folding: ASCII case-insensitive compare. The Windows case-insensitivity gap is limited to ASCII names, which is fine.
- `..` segments: handled by canonicalising the parent. `..` and `.` as the final name are rejected.
- Parent symlinks and junctions: resolved by `canonicalize(parent)`, so the deny list sees the real components. A directory reached through a link to `.ssh` or `.git` is caught.
- Final-component links: refused by `symlink_metadata` plus the reparse attribute, then `O_NOFOLLOW` or `FILE_FLAG_OPEN_REPARSE_POINT`. A drive-relative (`C:foo`) or rooted (`\foo`) outside path is refused as not absolute.
- Attachment bytes: nothing persisted. `PreparedAttachment` Debug prints label and length only, and rows and the session store carry labels. The driver has no prompt tracing.
- Image decode: limits are set (dimensions, alloc, input size) and the formats are limited to png, jpeg, gif and webp.
- Replay: no kept transcript content was found (see C6).
- Blocking: `prepare_attachment` is documented as blocking, with a threaded variant, and the actor never calls it. The only actor-thread blocking found is C4.

Status: DONE_WITH_CONCERNS
Summary: Four high findings (Windows deny-list bypass, engine-unbound outside confirmation, links pointing at the real root instead of the draft, and a held message overwriting send-now) and several medium issues (advisory no-follow open with no file size cap, PNG metadata passthrough, lost text on async failure, stale composer state, Codex parked steer). The questions path does not synthesise answers.
Concerns/Blockers: Not compiled or run, because Bash was unavailable. A3 needs confirmation of how `ctx.draft` relates to `project.root`. The Windows findings (A1) need a test on a Windows host.
