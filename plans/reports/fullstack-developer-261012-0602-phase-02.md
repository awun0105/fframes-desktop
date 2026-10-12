# Phase 2 report: primitives, icons and the bounds probe

Outcome: 38 Lucide icons (ISC, lucide-static) are embedded in `desktop/app/assets/icons/`, served by `assets.rs` (registered with `with_assets` on the studio window only), and exposed through `ui/` (`icon`, `card`, `card_2`, `divider`, `section`, `label`, `muted`, `heading`, `BoundsProbe`, `with_probe`, `bounds_to_record`). `lucide-LICENSE.txt` is in `packaging/sdk/notices/`; `package-phase-zero.py` already copies the whole notices folder.

Probe: both `StudioShell::button` and `ConversationPanel::button` now call `with_probe`; same ids, same gating (qualifying plus the ten ids; agent-policy/agent-apply), same `[left, top, width, height]` values into `button_bounds`. `design_system::surface()` had no callers and is deleted.

Validation: ui:: and assets:: tests pass (5 tests); fmt check passes; clippy `-D warnings -A deprecated` clean; full lib run is 160 pass and 2 fail (the known `python` spawn failures). Not run: `x11_shell` and `qualify-m2-playback.py` (need Linux, Xvfb, xdotool), so the telemetry equivalence is by code diff and the pure bounds test only.

Notes: Lucide renamed icons (`more-horizontal` is `ellipsis`, `alert-triangle` is `triangle-alert`); `Icon` variants keep the planned names. `card()` uses the `line` border (not `line_strong`), per the spec's 1 px line.
