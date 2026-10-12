# Phase 8: Agent panel

Status: done (agent column restyled; see reports/fullstack-developer-261012-0602-phase-08.md). Depends on: Phase 4. Effort: 2-3 days.

## Goal

Restyle the always-visible agent column (`desktop/app/src/conversation_panel.rs`, 2,793 lines, plus `controls.rs` and `rows.rs`) with the component library. Behavior, streaming, list virtualization and telemetry do not change.

## Current state (inventory)

- Root id `agent-panel`, `render` at `:2698`; header tabs `agent-tab-Chat`, `agent-tab-Project`, `agent-tab-Setup`; `agent-dismiss-notice`.
- Task banner `banner` `:1494` and review buttons `review_buttons` `:1567`: `agent-apply` (telemetry), `agent-discard`, `agent-export-candidate`.
- Options bar `options_bar` `:1663`: `mode-{id}`, `config-{id}`.
- Chat tab `chat_tab` `:1741`: virtualized `ListState` (`OVERDRAW_PX` 240, paging at 6 rows, `rows.rs:24-32`), `agent-load-earlier`, `agent-transcript`, `agent-jump-latest`, `queue-remove-{id}`.
- Rows `row_element` `:1164`, kinds User, Agent, Thought, Tool, Permission, Notice, Error, Validation, Changes, Outcome.
- Composer `prompt_form` `:1956`: `agent-prompt` (its bounds are `prompt_bounds`, clicked by `click_prompt` and `input_click`), `agent-send`, `agent-stop`. Keys via `controls::route_input_key` (`controls.rs:620`).
- Project tab `project_tab` `:2020`: `agent-project-tab`, `agent-policy` (telemetry), `agent-undo`, `agent-ack-writer`, `agent-export-draft`.
- Setup tab `setup_tab` `:2232`: `agent-setup-tab`, `agent-provider-{id}`, `agent-handoff-*`, `agent-mcp-toggle`, `agent-save-adapter`, `agent-check-adapter`.
- Telemetry `telemetry()` `:377` (keys include `buttons`, `prompt_bounds`); bounds captured in `button` `:1036` for `agent-policy` and `agent-apply` only.
- Colour sites: `:69-71`, `:1128`, `:1152-1159`, `:1185-1230`, `:1501-1517`, `:2257`, `:2711-2770`.

## Target

| Area | Target |
| --- | --- |
| Column | `card`, radius 12, `line` border (from Phase 4) |
| Header | 46 px `TabRail`-style bar with text tabs (Chat, Project, Setup) using `Segmented` (ids `agent-tab-*` on the items), dismiss notice as an X `IconButton` |
| Task banner | `Notice` (tone by phase: Info running, Success done, Danger failed) with review actions: `agent-apply` `Primary`, `agent-discard` `Destructive`, `agent-export-candidate` `Editor`; `agent-apply` keeps its probe |
| Options bar | `mode-{id}` as `Segmented`; `config-{id}` as `Select` + `Menu` or `Toggle` depending on the option's type (check the option model in `controls.rs` before choosing) |
| Rows | User: `card_2` bubble, radius 12, Body; Agent: plain Body `text_1`; Thought: Small `text_2` italic in a `Collapsible`; Tool: row with icon, name (Label), status `Badge`, collapsible output in a mono block on `card_2`; Permission: `Notice` Warning with actions; Notice, Error, Validation: `Notice` with matching tone; Changes: file list with + and − counts (`success` and `danger` colours); Outcome: `Notice` Success or Danger |
| List controls | `agent-load-earlier` and `agent-jump-latest` as small `Button` `Editor` (jump-latest floats above the composer when not at the bottom); `queue-remove-{id}` as X icon buttons on queued rows |
| Composer | Visual height only: a 64 px container around the existing single-line `TextInput` (multi-line editing would be new behavior and is out of scope), radius 12, `ctl` fill, `accent` focus border; `agent-send` `IconButton` with an up-arrow inside the input's bottom right, `agent-stop` replaces it while running; `agent-prompt` keeps `prompt_bounds` capture |
| Project tab | `FieldRow`s for policy and writer state; `agent-policy` `Button` keeps its probe; undo and export as `Editor` buttons |
| Setup tab | Provider list as radio rows (`agent-provider-{id}`), `agent-mcp-toggle` as `Toggle`, adapter actions as buttons; handoff controls as buttons with the same ids |

## Steps

1. Header and tabs, then banner and review buttons, then options bar.
2. Rows: change only how each `RowKind` renders. Keep row heights deterministic for the `ListState` measurement (`diff_rows` Remeasure path, `rows.rs:69`): avoid layout that changes height after first paint (no async image sizing inside rows).
3. Composer, then Project and Setup tabs.
4. Replace the colour sites listed above.

## Tests

- `cargo test -p fframes-studio --test agent_workflow_ui`, `--test agent_workflow`, `--test agent_workflow_apply_gate`, `--test provider_profiles`, `--test provider_handoff`, and the unit tests in `controls.rs` and `rows.rs`.
- Native: `tests/x11_shell.rs` agent flows (`agent-policy`, `agent-apply`, `click_prompt`) on Linux with Xvfb; record if not run.

## Validation

On the user's machine: send a prompt and watch streaming (no jitter or scroll jumps), stop a run, approve a permission, apply and discard a candidate, change mode and config options, switch providers in Setup, load earlier messages, jump to latest; both appearances.

## Done when

All rows and controls use the component library and tokens, every listed id is unchanged, and `agent-policy`, `agent-apply` and `prompt_bounds` still report correct bounds.

## Risks and rollback

Row height changes break scroll anchoring: keep heights stable and test with a long transcript. Collapsible rows change height on toggle: call the list's remeasure for that row. Rollback: revert; logic files (`host.rs`, `qualification.rs`, `provider-profiles.rs`) are not touched.
