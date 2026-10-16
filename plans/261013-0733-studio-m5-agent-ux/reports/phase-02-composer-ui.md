# Phase 2 (UI half): composer component

Worktree `C:/Users/Administrator/fframes-m5`, branch feat/studio-m5-ux. Nothing is committed or staged.
Verified from `desktop/`: `cargo test -p studio-ui --lib agent::composer` passes (41 tests) and
`cargo clippy -p studio-ui --all-targets -- -D warnings` is clean. No blanket `allow(dead_code)`.

## Layout (`desktop/crates/studio-ui/src/agent/composer/`)

| file | lines | holds |
| --- | --- | --- |
| mod.rs | 539 | `Composer` view, `ComposerEvent`, `Submission`, `AttachRequest`, `AttachKind`, render (card, slash popup, chip strip, controls) |
| input.rs | 607 | `ComposerInput` struct, events, undo, clipboard, newline, submit, key handling |
| input/element.rs | 211 | GPUI element: prepaint, paint, caret, selection |
| input/geometry.rs | 118 | point/index mapping, affinity, UTF-16 conversions |
| input/ime.rs | 202 | `EntityInputHandler` (marked text, replace) |
| input/layout.rs | 153 | cached text shaping and wrap |
| input/motion.rs | 347 | caret and selection movement, deletion |
| input/pointer.rs | 197 | mouse, multi-click, drag autoscroll, wheel |
| input/render.rs | 69 | the input's `Render` |
| input/tests.rs | 563 | ported and new tests for the input |
| slash.rs | 357 | token detection, filtering, menu state, insertion (with tests) |
| attachments.rs | 223 | `Attachments`, chips, strip (with tests) |
| steer.rs | 120 | `SteerMode`, `SendAction`, `steer_actions` (with tests) |
| keymap.rs | 167 | context, actions, bindings, `init` |
| metrics.rs | 152 | layout and scroll constants and pure helpers |
| tests.rs | 339 | composer-level GPUI tests |

All files are under 800 lines.

## Public API

`Composer::new(placeholder, theme, cx)`, `set_theme`, `set_commands(Vec<SlashCommand>)`, `set_running(bool)`,
`set_steer_mode(SteerMode)`, `set_attach_disabled(AttachKind, Option<reason>)`, `add_attachment(AttachmentDraft) -> AttachmentId`,
`remove_attachment(id)` (silent), `attachments()`, `text(cx)`, `set_text`, `restore(Submission)`, `focus`.
`keymap::init(cx)` must be called once by the owner. Context `"AgentComposer"`, actions namespace `agent_composer`.

Events (`ComposerEvent`, `Clone`):
`Submit(Submission)`, `SendAtBoundary(Submission)`, `Steer(Submission)`, `SendNow(Submission)`, `Stop`,
`AttachRequested(AttachRequest)`, `AttachmentRemoved(AttachmentId)`.
`Submission { text, attachments }`: the text is trimmed, the draft is cleared and the chips are taken before the event is emitted.
`restore` puts them back under their old ids if the owner refuses.
`AttachRequest` is `PickFiles | PickImages | Paths(Vec<PathBuf>) | Images(Vec<Arc<gpui::Image>>)`.
Chips come from the owner as `AttachmentDraft { kind, label, image }`; a chip carries no path.

Confinement is the owner's job: the composer reads and validates nothing on disk (documented in the `mod.rs` doc comment).

## Keys and steering

Enter sends, Shift+Enter inserts a newline, Cmd/Ctrl+Enter is send-now, Tab accepts a completion, Escape dismisses it.
Enter with a completion row selected accepts it. Enter is ignored while an IME composition is active.

| state | Enter | Cmd/Ctrl+Enter | button |
| --- | --- | --- | --- |
| idle | Submit | Submit | Send |
| running, AtBoundary | SendAtBoundary | SendNow | "Send at boundary" and "Send now (cancels)" |
| running, Native | Steer | SendNow | "Steer" and "Send now (cancels)" |
| running, None | nothing (hint "This agent cannot take a message while it works") | SendNow | "Send now (cancels)" |

The Stop button emits `Stop` and performs nothing. The composer only picks the event.

## GPUI adaptations

Kept the stored `studio_theme::Theme` (`ComposerInput::new(placeholder, theme, cx)`, `set_theme`) instead of Zeron's `Theme::of`.
`window.blur(cx)` form, `layout_text(width, &TextStyle, window)` takes no cx, layout cache key is width, font, size, colour, marked range and placeholder.
Own key context so it never shadows the kit `TextInput` bindings.
Attachment ids are monotonic and never reused. This is NOT Zeron's numbering struct.
While a kind is disabled via `set_attach_disabled`, pasted images or paths of that kind are silently dropped (no `AttachRequested`) and the button is disabled with the reason as tooltip.

## Dropped from Zeron

TextProjection (projection is the identity), file/chat/attachment mention modules, `invocation`, `composer_markdown`
(Markdown faces, list indent and newline editing, syntax spans), mention tooltips, inline chip painting, dictation and voice,
palette context, ghost text, single-line mode, wizard context swap, transcript-selection copy, compact/expanded morph, edge fade,
viewport reveal and resizing, Tab list indent, and the Composer wrapper's RPC, queue, pickers and appshots.

Zeron tests ported: CRLF navigation and deletion, soft-wrap affinity, vertical column retention, undo steps, IME, shift-backspace,
layout cache reuse. `attachment_chip_tests.rs` (inline chips) does not port.

## Limitations

- The slash token cannot gate on `in_code` (needs pulldown-cmark, not a direct dependency). It opens when `/` starts the draft or follows whitespace or `([{>`;
  a slash inside backticks still opens the menu. Recorded in the `slash_token` doc comment.
- Disabling an attach kind gates new pastes and the attach button only. Chips of that kind that were already accepted still go out with the next send, so the owner must filter them.
- A paste of a disabled kind is dropped with no visible feedback to the user.
- `pub mod composer;` in `agent/mod.rs` and the `serde`, `similar` and `unicode-width` lines in `studio-ui/Cargo.toml` were added by the lead or another agent, not by this task. `pulldown-cmark` is not a dependency; the `in_code` gate can be restored if it is added later.

## TextInput vs ComposerInput

plan.md records the 2026-10-14 decision to KEEP ComposerInput, while the phase file's reference pattern 1 says kit `TextInput`. I followed the dated decision.
However, the spike's three reasons for keeping ComposerInput were projection, inline mention and attachment chips, and Markdown faces.
This port drops all three, so what remains (multiline wrap, undo, IME, Enter/Shift+Enter keymap) overlaps heavily with kit `TextInput`.
The plan owner should revisit, or restore projection when `@` mentions land.

## Remaining for driver wiring (not done, out of scope)

1. Call `composer::keymap::init(cx)` at boot.
2. Fill `set_commands` from `AgentEventKind::AvailableCommands`.
3. Handle `AttachRequested`: pickers, project confinement and deny list, image transcoding and size cap; call `add_attachment`. Gate on image capability via `set_attach_disabled`.
4. Map `SteerMode` from the driver capability flag; call `set_running` from turn state.
5. Handle `Submit`, `SendAtBoundary`, `Steer`, `SendNow` and `Stop`; call `restore` when a send is refused.
6. Replace the prompt field in `chat_tab.rs`, `controls.rs` and `commands.rs` with `Composer` (not touched here).
