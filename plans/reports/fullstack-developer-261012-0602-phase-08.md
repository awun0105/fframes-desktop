# Phase 8 report: agent panel

Outcome: `desktop/app/src/conversation_panel.rs` (the only file changed, plus the phase status line) is restyled with the `ui/` kit and tokens. Behavior, telemetry keys, element ids and labels are unchanged. No commit, no background processes.

Changes:
- Building blocks: `button()` now returns `ui::Button` (Tone maps to Editor/Primary/Destructive); `icon_button`, `segment_button`, `segment_rail`, `provider_row`, `activate` added. `agent-policy` and `agent-apply` keep their bounds probe (`probed_button` gate, same `button_bounds` map); `agent-prompt` still wraps the input with the canvas that fills `prompt_bounds`.
- Header: 46 px bar, "Agent" title, Chat/Project/Setup as a rail of ghost buttons with ids `agent-tab-*`; readiness headline under it; the dismiss notice is a Warning `Notice` plus an X `IconButton` (`agent-dismiss-notice`, tooltip "×"). Root no longer paints its own bg or left border (the shell column card does).
- Banner: `Notice` (Info running, Success accepted, Warning waiting/awaiting/interrupted, Danger failed/conflict) with `agent-apply` Primary, `agent-discard` Destructive, `agent-export-candidate` Editor as actions; conflict block on a nested card below it.
- Options: `mode-{id}` as a segmented rail (selected item highlighted and inert, label keeps the "● " prefix); boolean `config-{id}` as `FieldRow` + `Toggle`; select options stay click-to-cycle `Button`s with a chevron.
- Rows: User on `card_2` bubble, Agent plain Body, Thought in a `Collapsible` (state in `thoughts_open`, toggle calls `list.remeasure_items` for that row only), Tool row with icon, Label name and status `Badge`, Permission/Notice/Error/Validation/Outcome as `Notice` with matching tone (Permission actions inside the notice), Changes as a file list with coloured A/M/D/X marks. Row spacing is padding inside the measured element, all text uses fixed line heights, no async sizing.
- List controls: `agent-load-earlier` small Editor button; `agent-jump-latest` floats at the bottom of the transcript (Editor, opaque card background); `queue-remove-{id}` is an X `IconButton` (tooltip "Remove").
- Composer: fixed 64 px container (`ctl`, radius 12), single-line `TextInput` on top (its own accent focus border), `agent-send` and `agent-stop` bottom right. The old header duplicate of `agent-stop` is gone; the composer one shows whenever a stop is available.
- Project tab: `agent-policy`, `agent-undo`, `agent-ack-writer` in `FieldRow`s; export buttons stay Editor buttons. Setup tab: providers as radio rows (`agent-provider-{id}`, old button label kept as accessible name and trailing badge), `agent-mcp-toggle` as `Toggle` in a `FieldRow` (same label text), settings error as a Danger `Notice`.
- All old colour sites and `text_xs`/`rounded_md` literals are gone.

Tests: pure tests added (`presentation_tests`: probe gating, banner tone, outcome title/tone, tool badge). `cargo fmt` clean for the file; clippy `--all-targets -D warnings -A deprecated` is clean for this file (it stops on a `redundant_closure` in `setup_view.rs:433`, owned by the concurrent agent; with `-A clippy::redundant_closure` everything passes). lib: 231 pass, 2 fail (known python-fake-worker pair). `provider_profiles` 17 pass; `agent_workflow_ui`, `agent_workflow`, `provider_handoff`, `agent_tools` run 0 tests on this host; `agent_workflow_apply_gate` fails 2 (known baseline). Not run: `x11_shell` (no X/GPU; it compiles under `--all-targets`), on-screen look, streaming or scroll behavior.

Deviations to review:
- Send is a labelled Primary button, not an up-arrow icon: the kit has no arrow glyph and I may not edit `ui/`. Its label stays dynamic (`controls.submit.label()`), and send stays visible next to stop because it also queues.
- Tabs and modes use ghost `Button`s in a rail instead of `Segmented`, because `Segmented` item ids are `(id, index)` and the plan requires `agent-tab-*` / `mode-{id}` unchanged.
- Select-type config options are not `Select` + `Menu`: the option model is click-to-cycle, and a menu would be new interaction state.
- Tool rows have no collapsible output block (the `ToolCard` model has no output). Changes rows have no +/- line counts (the model has none), only change kind marks.
- The conflict block sits below the banner notice rather than between its text and buttons (`Notice` takes text only).
- A Thought row is collapsed by default, including while streaming; its text appears when the reader expands it.

Status: DONE_WITH_CONCERNS
Summary: Agent panel is restyled with the component library, all ids, labels, probes and `prompt_bounds` preserved; fmt, clippy (for this file) and tests show only the known baseline failures.
Concerns/Blockers: Visual layout, streaming and scroll anchoring were not checked on screen or via `x11_shell`; the deviations above (text Send button, button rails instead of `Segmented`, cycle buttons for select options) need review.
