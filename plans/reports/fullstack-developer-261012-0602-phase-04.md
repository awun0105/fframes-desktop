# Phase 4 report: app shell

Outcome: `StudioShell::render` now builds the card layout (8 px gutter, header card, notices, status strip, 248 px left column with icon tab rail, player card, timeline card, agent column card) from private region functions; empty state added.

Files (desktop/app/src): `studio_shell.rs` (region fns `render_header`, `render_notices`, `render_status_strip`, `render_left_column`, `render_recents`, `render_empty_state`, `render_player_area`, `render_agent_column`; `LeftTab` replaces `navigation`; pure fns `header_actions`, `status_strip_expanded`, `error_title_details` with 4 tests), `ui/tab_rail.rs` (`TabItem::id` so tabs keep ids `Project`/`Assets`/`Style`). Phase file status updated. Nothing under root `examples/`; no commit; no background processes.

Placement: header buttons are `ui::Button` (create/open/import Editor with icons, export-mp4 Primary, close Editor), all with the qualification probe (`bounds_probe` extracted from `button()`, key set unchanged, close still probed). Create/open/import render only in the empty state while no project is open (`header_actions`), recents render once (left column with a project, empty state without). awaiting-preview, error (first line title, full text under Details, `independent` action), export (`export-status`, `cancel-export` action), interrupted and recovery lines are Notices; status lines are a collapsible strip (StatusDot green when source = checkpoint; auto-expanded on error or Queued/Running/Cancelling job). Preset panel shows in the left column under the Style tab. Player region content is moved unchanged (preview surface probe, canvas_origin/extent, preview_extent, transport buttons, history) inside the player card; timeline sits in a content-sized card below. `start_qualification`/`start_agent_telemetry` untouched.

Validation: `cargo fmt --check` ok; clippy `-D warnings -A deprecated` for fframes-studio `--all-targets` clean; lib tests 210 pass, 2 fail (known python spawn failures); promotion_handoff and agent_workflow_ui run 0 tests on this host. Not run: x11_shell (no X/GPU), on-screen rendering.

Deviations and notes:
- Step 0 not confirmed: Cap's EditorSection code did not show a card radius; `radius_card` stays 12.
- Timeline card has no fixed 260 height; the timeline view's own height drives it until Phase 6 sizes it. The error Details block is not scrollable (Notice's collapsible has no max height).
- Close is now an Editor button (was Destructive), per the element map. Tabs are icon-only with the old label as tooltip and accessible name.
- Export notice tone varies (Success on complete, Danger on failure, Warning on warning) instead of always Info.

Status: DONE_WITH_CONCERNS
Summary: Phase 4 shell is implemented with tests; fmt and clippy are clean and only the two known python lib tests fail.
Concerns/Blockers: Layout, notice, strip and empty state are unverified on screen; the timeline height and Cap radius check are deferred as noted.
