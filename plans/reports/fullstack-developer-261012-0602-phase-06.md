# Phase 6 report: timeline card

Outcome: `desktop/app/src/timeline_view.rs` is restyled to the Cap-style card; only that file changed (plus the phase status line). No commit, no root `examples/` edits, no background processes.

Changes:
- Card content: fixed height `timeline_default_h` (260 incl. the shell card border), padding 11 (top 9). `studio_shell.rs` untouched.
- Header: `SectionTitle` "Timeline"; right side `IconButton` ZoomOut (`timeline-zoom-out`), 72 px zoom `Slider` (track 3, new id `timeline-zoom-slider`, log scale 0.01..100000 px/s, same zoom path anchored at the playhead), ZoomIn (`timeline-zoom-in`), `Button` "Fit" (`timeline-fit`, probe fills `fit_bounds`), Layers `IconButton` (`timeline-cycle`).
- Geometry from `Metrics`: ruler 26, rows 44 with 6 gap, gutter 104 + 8 gap. Gutter now sits left of the time axis inside `compiled-timeline`, so the viewport width and pointer x are measured from `bounds.left + 112`.
- `hit_zone(y, top)` replaces the literal 24: ruler (`y - top < ruler_h`) scrubs, lanes select. Clicks in the gutter are ignored.
- Telemetry: `ruler_bounds` = `[left + 112, top, width - 112, 26]` (the time axis only, so existing click points at 10-20 % of its width and +10 px stay on the ruler); `fit_bounds` is the Fit button's real bounds.
- Ruler ticks (major 8, minor 5, `line_strong`, labels Small `text_3`; engine ladder already gives labels >= 80 px apart), rows with `card_2` band radius 8, segments via `segment_colors` + 3 px accent bar + label anchored to the visible slice with ellipsis, playhead 1 px line with 12 px head and 4 px `card` ring, selection fill with 1 px accent edges. All colour literals and old `colors::` uses are gone. No hover ghost line (view does not track hover).
- Tests (pure): `ruler_click_scrubs_and_lane_click_selects`, `zoom_slider_maps_to_zoom_range_and_back`, `segment_label_anchors_to_visible_slice`, plus minor-tick and inset tests.

Validation: `cargo fmt --check` ok; clippy `--all-targets -D warnings -A deprecated` clean; timeline_view 5/5, `timeline_controls` (non-ignored) and thumbnail cache tests pass; lib 221 pass, 2 fail (the known python-fake-worker pair). Not run: `x11_shell`, `qualify-*.py`, ignored SDK tests, on-screen look (no X/GPU).

Deviations to review:
- Zoom icon buttons keep the old labels "−" and "+" (tooltip and accessible name), as do Fit and "Cycle scene (S)"; `IconButton` has no key chip slot, so no separate S chip.
- `ZOOM_MIN/MAX` are mirrored from private constants in `studio-engine/src/timeline.rs` (outside this phase); if the engine range changes, update both.
- Scenes on lane 0 use `TrackKind::Clip`, on later lanes (overlapping) `TrackKind::Overlay`; audio uses `Audio`. The report has no explicit overlay flag.
- The thumbnail strip and status line stay below the tracks (38 + 16 px), so the lanes area shows about two rows and scrolls.
- `x11_shell.rs`/`qualify-m2-playback.py` read `ruler_bounds` and should still hit the ruler by construction; unverified by running.

Status: DONE_WITH_CONCERNS
Summary: Timeline card, header with zoom slider, ruler-based scrub/select boundary and accurate `ruler_bounds`/`timeline-fit` telemetry are implemented; fmt, clippy and the pure tests pass with only the known python failures.
Concerns/Blockers: Visual layout and native click qualification not run here; mirrored zoom constants and icon-button label changes noted above.
