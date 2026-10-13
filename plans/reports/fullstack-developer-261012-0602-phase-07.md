# Phase 7 report: left column content

Outcome: the Project, Assets and Style tabs are restyled in `desktop/app/src/studio_shell.rs` (left column functions) and `desktop/app/src/preset_panel.rs`. No commit, no `timeline_view.rs` or root `examples/` edits, no background processes left.

Changes:
- Column: 248 px wide, padding 16 (content 216), dispatches on `left_tab_content(tab)` (Project to Files, Assets to Assets, Style to Presets).
- Project tab: `SectionTitle` "Files" with a `Badge` count, 28 px rows (File icon 14, truncated name, hover fill, full name as tooltip) keeping `file-{file}`, "Showing n of m files" as Small `text_3`. "Managed SDK" and "Recent projects" are `Collapsible` sections (state `sdk_open`, `recents_open`) shown on the Project tab only. The SDK section has a `StatusDot` (green ready, amber missing, blue busy "Operation in progress…") and three full-width `Editor` buttons (`install`, `import-sdk-bundle`, `sdk`) with unchanged labels and enable rules.
- Recents: `render_recents()` is shared with the empty state; rows show name, Missing `Badge` (Warning) and path (Small `text_3`). `locate-{id}` ("Locate…" button) and `remove-{id}` (X icon button, "Remove recent" as tooltip and name) sit in an overlay that appears on pointer hover and while keyboard focus is inside the row.
- Assets tab: same rows with the Image icon, `asset` "Copy asset…" as an `Editor` button with a Plus icon at the bottom, same enable rule.
- Style tab: preset panel now renders only here (inside `preset-scroll`), plus the style file rows or the old "No style preset configured" line. The panel has no card; sections are split by `divider()`. Preset list is radio-style rows (h 34, check icon on the selected one, ids `preset-{key}`); Apply is `Primary`, Reapply and Reset are `Editor`, stacked; import, export and CSS report are stacked `Editor` buttons (ids unchanged). Overrides: `SectionTitle` "Project overrides", each row token (Body) over "layer · value" (Small), `clear-{token}` X icon button ("Clear" tooltip); the two compact inputs are stacked with `override-set` below; diagnostics are Warning `Notice`s; `preset-status` is a `Notice` (Success or Danger) with the id on its root; the CSS report is a `Collapsible` "CSS import report" inside `css-report`. Keyboard routing, `PresetCommand`, `execute` and telemetry untouched.
- Removed the now unused shell `button()` helper.
- Pure tests added: `left_tab_content_maps_each_tab`, `recent_rows_offer_locate_only_when_missing`, `sdk_status_tone_prefers_busy_then_ready`, `status_text_splits_title_from_body`.

Validation (run in `desktop/app`): `cargo fmt --all --check` ok; `cargo clippy --all-targets -- -D warnings -A deprecated` clean; lib tests 225 pass, 2 fail (the known python fake worker pair: `preview_worker_client::...malformed_destinations...`, `worker_client::...stalled_control_request...`). `tests/preset_workflow.rs` is `#![cfg(target_os = "linux")]`, so it runs 0 tests on this host; `real_sdk_presets` is ignored.

Deviations to review:
- Preset rows show the text without the old "✓ " prefix (the check icon replaces it); the accessible name and tooltip keep the full old label including "✓ ".
- Overrides use a custom two-line row instead of `FieldRow` (its 96 px label column would overflow at 216 px); the "Project overrides" heading keeps its old label instead of "Overrides".
- Import, export and CSS report are a stacked button group, not a `Menu`.
- Recent actions use `in_focus` on a focusable wrapper for keyboard reveal; this is unverified on screen (no GPU/X here).
- The Project tab file list, recents and SDK status layout, and the 1280/1920 width checks need the user's machine.

Status: DONE_WITH_CONCERNS
Summary: The three left column tabs and the preset panel are restyled with ids, labels, telemetry and behavior unchanged; fmt, clippy and the pure tests pass apart from the known python-worker lib failures.
Concerns/Blockers: Visual layout, hover/focus reveal of recent actions and native qualification are unverified here; the integration-test sweep result is added below if it completed.

Integration sweep (`cargo test --tests --no-fail-fast`): failures only in the known baseline set (lib 2, agent_workflow_apply_gate 2, build_sharing 1, candidate_runner failures). All other suites pass.
