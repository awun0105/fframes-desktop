# CI, build, release and UX fixes

## Verification
- CI fmt (`cargo fmt --manifest-path desktop/Cargo.toml --all --check`): clean.
- CI clippy (`-p studio-project -p studio-engine -p studio-bootstrap -p studio-sdk -p studio-agent-spike -p fframes-studio --all-targets -- -D warnings`, no `-A deprecated`): clean. `worker_client.rs` `try_update` compiles and is clean.
- `cargo test --locked --no-fail-fast` over the same package set, with the python shim on PATH for the cargo process only: 0 failures (239 lib tests plus every integration binary; ignored tests need the SDK).
- Top-level telemetry JSON keys (studio_shell, conversation_panel, timeline_view, canvas_view, preview_element): identical to HEAD. Only the documented additions below.
- Python steps of the CI step: `validate-qualification.py` for m0 and m2 pass; `test-packaging.py` runs 60 tests with one error, `test_help_documents_every_argument`, because it spawns `python3`, which this host lacks (the shim provides only `python.exe`); unrelated to this work, Linux CI has `python3`. Nothing in `desktop/scripts` or `desktop/qualification` pins the `buttons` key set (the M2 script reads named entries only). No code outside `setup_view.rs` writes `SetupState::Failed`, so `fail()` is the only entry.
- Cross-check `cargo check --target x86_64-unknown-linux-gnu` is not possible on this host (cc-rs needs `x86_64-linux-gnu-gcc`); the Linux-only code was reviewed by reading instead (A4).

## A. CI, build, release
1. fmt and clippy clean (above).
2. `x11_shell.rs` is the only test with fixed click coordinates (`CHAT_TAB`, `PROJECT_TAB`, `UNDO_BUTTON`). It is COMPILE-EXPOSED in `desktop.yml` (the Linux job builds it under `cargo test` and `clippy --all-targets`), but its tests are `#[ignore]`d and no workflow job runs them: only `qualify-m3-agent.py` (`native_x11_shell`) and `qualify-m5-selection.py` do, and no workflow calls those scripts. Windows never compiles it (`#![cfg(unix)]`), so my edit was desk-checked only: all 8 call sites use the owned `screen`, `window` and `telemetry` bindings (same types the existing `click_prompt` calls use), nothing references the removed constants, and `click_panel_button` asserts the measured centre is inside the 1280x800 window so a scrolled-off control fails clearly instead of timing out. Equal-width tabs would still depend on font metrics and the 248/agent column widths, so I used the probe route: `probed_button` in `conversation_panel.rs` now also reports `agent-tab-Chat`, `agent-tab-Project` and `agent-undo` in the panel's `buttons` telemetry map (rail items go through `segment_button`, which now takes the probe). The test clicks measured bounds through a new `click_panel_button` helper (waits until the bounds are stable, like the existing `agent-policy` clicks). The three constants are removed. Other click sites in the test and in `qualify-linux-native.py` already use telemetry (`input_click`, `source_click`, `prompt_bounds`, `ruler_bounds`). Not run (needs Xvfb, xdotool, the SDK); it compiles only on unix, so it is unverified here.
3. `qualify-m2-playback.py seek()`: consistent. `thumbnail_metrics.ruler_bounds` is the timeline area's left plus `axis_inset()` (112 px) and the pointer handler subtracts the same inset (`pointer_frame`, scroll wheel anchor); the ruler sits at the area's top, `ruler_h` high, and the script clicks at 55 percent of that height, inside the ruler hit zone. `pixels_per_second` and `scroll_x` are axis-relative. No app change needed.
4. `cfg(target_os)`/`cfg(unix)` in files this work touched: only `main.rs` `cfg(windows)` blocks (not changed by the restyle). `ui/`, `design_system/`, `assets.rs` and the restyled views contain no platform cfgs, so Linux/macOS compile the same code Windows does.
5. Release: `Cargo.toml` and `Cargo.lock` are unchanged (no new dependency; icons are embedded with `include_bytes!`). Notices ship via `shutil.copytree(desktop/packaging/sdk/notices, ...)` in `assemble-phase-zero-sdk.py`, `package-phase-zero.py` and `package-linux-deb.py` (the whole directory), so `lucide-LICENSE.txt` is included without a list change. The Windows installer script lists only `notices/LICENSE.txt` as a required file. No licence gate file enumerates notices. `release-policy.json` has no dependency list.

## B. UX
1. `ui::Menu`: `MenuState::with_disabled`, `is_disabled`, `Menu::row_ids`. Disabled rows use `text_3`, have no hover or pointer cursor, are skipped by Up, Down, Home and End, click and Enter are inert, and `set_highlighted` refuses them. Pure tests cover skipping, wrap, Home/End, commit on a disabled row, and an all-disabled menu. The overflow menu in `studio_shell.rs` always lists Refresh, Save checkpoint, Restore as copy, Reveal retained draft and Reveal checkpoint; rows carry the HEAD ids `refresh`, `checkpoint`, `restore`, `draft`, `reveal-checkpoint`; `overflow_actions` now returns `(action, enabled)` with the old enable rules and the trigger is always enabled.
2. `Icon::ArrowUp` (Lucide `arrow-up.svg`, lucide-static 1.55.0, ISC, same header as the other icons); `Icon::ALL` is 39 and the asset tests cover it. `agent-send` is a round primary `IconButton` with the submit label as tooltip and accessible name; the blocked hint text stays.
3. `Notice` details scroll in a 160 px maximum height box (`DETAILS_MAX_H`).
4. Reasoning rows stay collapsed; while streaming the header shows a "Thinking..." info badge. The header has a fixed 20 px height so row height is the same with or without the badge. The old header text "Reasoning · writing…" is replaced by "Reasoning" plus the badge.
5. `SetupView` has a view-local `failed_step`, set by `fail()` (the only way into `Failed`); `step_statuses_after` marks that step failed, earlier steps done, later pending. Install, build and render run as one pipeline, so a failure there is attributed to "Install SDK" (the pipeline reports no sub-step). `step_statuses` is unchanged. Pure tests added.
6. `docs/desktop/design-system.md` updated (Menu, Notice, icons, screen behaviours, telemetry note).

## Deliberate deviations
- New keys in the agent panel `buttons` telemetry map (qualification only): `agent-tab-Chat`, `agent-tab-Project`, `agent-undo`. The set of top-level telemetry keys is unchanged.
- The overflow menu rows regain the HEAD ids (`refresh`, `checkpoint`, `restore`, `draft`, `reveal-checkpoint`); these were previously missing after the restyle.
- Reasoning header text no longer says "writing…"; the badge says "Thinking…".
- The overflow trigger (`overflow-actions`, in the transport bar next to build and cancel) is always enabled; at HEAD the equivalent buttons were individually disabled and the trigger was disabled when no action applied.
- `agent-send` has no visible text label any more (tooltip and accessible name keep it).

## Unresolved
- `x11_shell.rs` and the M2 playback script could not be executed on this Windows host.
- The Linux target could not be cross-checked locally.

Status: DONE_WITH_CONCERNS
Summary: fmt, clippy and the full test set are clean; all requested CI, layout-probe and UX changes are in, with the deviations listed above.
Concerns/Blockers: x11_shell.rs (compile-exposed on Linux CI) and the M2 script could not be run or compiled here; `test-packaging.py` has one host-only failure (`python3` missing).
