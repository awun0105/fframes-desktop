---
title: "Cap-style UI for fframes-studio"
description: "Rebuild the fframes-studio UI on a runtime light/dark design system and a component library taken from Cap's editor, then restyle the shell, player, timeline, left column, agent panel and setup screen."
status: implemented; pending on-machine checks (rendering, theme switch, screenshots, Cap comparison, x11 and M2 native runs)
priority: P2
effort: "18-26 engineer-days"
branch: main
tags: [desktop, ui, gpui, design-system]
blockedBy: []
blocks: []
created: 2026-10-11
---

# Cap-style UI for fframes-studio

## Overview

Give the gpui editor (`desktop/app`) Cap's look in three layers, bottom up:

1. **Design system** (Phases 1 and 2): runtime light and dark tokens for colour, spacing, radii, sizes, typography, shadows and motion, plus icons, surfaces and a shared telemetry bounds probe.
2. **Components** (Phase 3): buttons, toggle, segmented control, select and menu, slider, tab rail, tooltip, key chip, field rows, badges, notices, collapsible, and the text input restyle, with a gallery and a design-system doc.
3. **Screens** (Phases 4 to 9): the window frame, the player, the timeline, the left column, the agent panel and the setup screen, each with an element-by-element map from today's UI to the new components.

Phase 10 verifies everything. Behavior does not change.

Values come from [the Cap design spec](../reports/xia-261011-0016-cap-design-system-spec.md) (Cap `apps/desktop-gpui` at `877d3a3`, with file and line references). Today's UI was inventoried element by element (ids, enable rules, telemetry, colour sites); each phase carries the part it needs.

## Decisions

By the user:

- Scope: restyle plus restructure the editor.
- Appearance: light and dark, following the OS; no manual override.
- Materials: flat surfaces everywhere; no glass.
- Agent panel: always visible in its own right column.
- Linux font: platform default; no bundled font.
- Colours: exact Cap values everywhere, accepting measured WCAG AA shortfalls (white on `accent` 4.02 light, 3.65 dark; light `text_2` on `window` 4.49; light `text_3` on `card` 2.60).
- Verification: rendering is checked on the user's machine; this VM runs builds and tests.

In this plan, open to veto:

- Left column keeps today's 248 px width, always visible (user decision), with an icon tab rail (Project, Assets, Style) on top.
- The preset panel moves from under the timeline into the Style tab.
- Status lines move into a collapsible status strip; banners become notices.
- Zoom controls stay in the timeline header (Cap puts them in the transport) because the timeline owns zoom state and the `timeline-fit` telemetry.
- Every control gets a visible focus ring and Enter or Space activation (Cap's widgets mostly have neither).

## Licensing line

Cap's desktop app is AGPL-3.0; fframes is MIT. Take design values and behavior as facts from the spec; do not copy, translate or closely paraphrase Cap's Rust code. Work from the spec tables rather than open Cap files. Icons come from Lucide (ISC) with its licence in `desktop/packaging/sdk/notices/`. This is an engineering judgement, not legal advice.

## Hard constraint: telemetry and ids

Native tests and qualification scripts click elements by the bounds the app reports. These must stay rendered by default and keep reporting:

- Shell button probes (only while qualifying): `build-preview`, `cancel-preview`, `play-pause`, `mute-audio`, `next-audio-output`, `retry-audio-output`, `no-audio-output`, `previous-frame`, `next-frame`, `close`.
- Timeline: `timeline-fit` and `ruler_bounds`.
- Agent panel: `agent-policy`, `agent-apply`, and `prompt_bounds` for `agent-prompt`.
- Preview: `canvas_origin`, `canvas_extent`, `preview_extent`, and the `source_click` mapping.

Consumers: `desktop/app/tests/x11_shell.rs` (`build-preview`, `next-frame`, `agent-policy`, `agent-apply`, composer, `ruler_bounds`), `desktop/scripts/qualify-m2-playback.py` (`play-pause`, `mute-audio`, `previous-frame`, `next-frame`, `timeline-fit`, the three audio-output buttons, `ruler_bounds`), and `qualify-linux-native.py` and `qualify-windows-native.py` (`input_click`, `source_click`). `cancel-preview` and `close` have no current consumer but stay always rendered.

gpui's headless `TestAppContext` is not available in this workspace, so new tests are pure-function tests and rendering is checked in the gallery and the native runs.

All other element ids and labels stay unchanged too; icon-only buttons keep their id and use the old label as tooltip and accessible name.

## Non-goals

- No new editing features, no protocol or worker changes, no GPU preview work.
- No manual theme setting, no bundled fonts, no glass.
- `app.rs` and `agent_spike.rs` keep their own colours.
- The unassigned `CreatingProject` and `Building` setup states are noted, not fixed.

## Phases

| # | Phase | Depends on | Effort |
|---|---|---|---|
| 1 | [Design system foundation](phase-01-design-system-foundation.md) | none | 2-3d |
| 2 | [Primitives, icons and the bounds probe](phase-02-primitives-and-icons.md) | 1 | 1-2d |
| 3 | [Component library](phase-03-components.md) | 2 | 4-5d |
| 4 | [App shell](phase-04-app-shell.md) | 3 | 2-3d |
| 5 | [Player card](phase-05-player.md) | 4 | 2-3d |
| 6 | [Timeline card](phase-06-timeline.md) | 3, 4 | 1-2d |
| 7 | [Left column content](phase-07-left-column.md) | 4 | 2d |
| 8 | [Agent panel](phase-08-agent-panel.md) | 4 | 2-3d |
| 9 | [Setup view](phase-09-setup-view.md) | 3 | 1d |
| 10 | [Verification and evidence](phase-10-verification.md) | 1-9 | 1-2d |

Phases 1 to 4 are sequential. After Phase 4, Phases 5 and 6 both edit regions placed by `studio_shell.rs` but different files (5 edits `studio_shell.rs`, `preview_element.rs`, `canvas_view.rs`; 6 edits `timeline_view.rs`), Phase 7 edits `preset_panel.rs` and the left column functions in `studio_shell.rs` (do it after 5 to avoid conflicts in that file), Phase 8 edits only `conversation_panel*`, and Phase 9 only `setup_view.rs` (after the in-flight release edits to that file are committed). One developer should run them in order 5, 6, 7, 8, 9.

## Acceptance criteria

1. Light and dark render correctly and switch live with the OS; the Phase 1 contrast tests pass with the accepted shortfalls recorded.
2. No `rgb(`/`rgba(` literal or `colors::` use remains in `desktop/app/src` outside `design_system/`, `app.rs` and `agent_spike.rs`.
3. The layout matches Phases 4 to 9, every listed element id and label is unchanged, and every telemetry key above reports correct bounds; the telemetry JSON keys are identical before and after.
4. `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` pass with the commands in `.github/workflows/desktop.yml`.
5. Every component in Phase 3 exists with its tests; the gallery shows all states in both appearances; `docs/desktop/design-system.md` exists.
6. Token values, component sizes and layout measurements match the spec, checked by unit tests, apart from the listed deliberate differences.
7. Screenshots in both appearances at three window sizes are reviewed on the user's machine, and a side-by-side comparison with Cap lists no undocumented differences.
8. The Lucide notice ships in the package; no Cap code was copied.

## Deliberate differences

Collected from the phase reports and the CI and UX report; each is accepted unless noted.

Design system and components
- Status colours use Radix step 11 for text and step 3 for surfaces; step 9 is unused.
- `ctl*` alphas are rounded to whole bytes; a selected segment lightens the fill only, not the border.
- Handlers are `Fn(&mut Window, &mut App)` so one handler serves click and Enter/Space.
- The focus ring is a 0-blur 2 px accent shadow; destructive hover and pressed blend `danger_surface` toward `danger` (12 % and 22 %) because the spec gave no values.
- `design_system::button()` keeps the wrapper form (styles a caller-built element) with the new look.
- `Menu` rows can be disabled (skipped by Up, Down, Home, End; click and Enter inert). `Icon::ArrowUp` was added (39 icons).
- `Notice` details scroll in a 160 px maximum-height box.
- Tooltips on icon buttons show text only; no `Kbd` chips (no key slot on `IconButton`).
- Card radius stays 12: the Cap card radius could not be confirmed (Phase 4 step 0).

Screens
- Close is an Editor button, not Destructive; tabs are icon-only with the old label as tooltip and accessible name.
- Export notice tone varies (Success, Danger, Warning) instead of always Info.
- The timeline card has no fixed height; zoom constants are mirrored from `studio-engine/src/timeline.rs`; zoom, Fit and scene-cycle keep their old labels and have no key chip.
- Scene lane 0 is `TrackKind::Clip`, later lanes `Overlay`; the thumbnail strip and status line stay below the tracks.
- The time readout is not tabular-figure (fixed minimum width).
- The overflow menu always lists Refresh, Save checkpoint, Restore as copy, Reveal retained draft and Reveal checkpoint, with unavailable rows disabled. Its trigger (`overflow-actions`) is always enabled; at HEAD the individual buttons were disabled and the trigger with them. Rows carry the HEAD ids. The enabled state is fixed when the menu opens.
- Preset rows drop the "✓ " prefix visually (accessible name keeps it); overrides use a custom two-line row; the "Project overrides" heading keeps its old label; import, export and CSS report are a stacked button group, not a `Menu`.
- Agent panel: tabs and modes are ghost `Button`s (not `Segmented`) to keep `agent-tab-*` and `mode-*` ids; select options still cycle on click; tool rows have no output collapsible and changes rows have no line counts (no model data); the conflict block sits below the banner; `agent-send` is a round primary icon button with the label as tooltip and accessible name.
- Reasoning rows stay collapsed; while streaming a "Thinking..." badge replaces the "Reasoning · writing…" header text, in a fixed 20 px header.
- Setup: `SetupView` remembers the failing step (`failed_step`); install, build and render run as one pipeline, so failures there are attributed to "Install SDK". An intro line was added under the heading. `CreatingProject` and `Building` both show "Build template" active and are never assigned (noted, not fixed).

Telemetry
- New qualification-only entries in the agent panel `buttons` map: `agent-tab-Chat`, `agent-tab-Project`, `agent-undo`. Top-level keys are unchanged.
- `x11_shell.rs` no longer uses fixed click constants for the agent tabs and Undo; it clicks measured probe bounds.

Outside the UI but touched
- `worker_client.rs` uses `try_update` instead of the deprecated `fetch_update` so clippy passes on the pinned toolchain.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Translucent Cap colours painted through `rgb()` lose alpha | Near-black fills that still compile | Phase 1 switches the palette to `Rgba` and removes the `colors` module |
| Telemetry probe refactor shifts coordinates | Native qualification fails silently | Phase 2 moves the probe unchanged; Phase 10 diffs telemetry JSON |
| Taller timeline ruler breaks scrub or select | Clicks select instead of scrub | Phase 6 replaces the hard-coded 24 px boundary with the ruler geometry |
| Row restyle breaks the agent list's scroll anchoring | Jitter while streaming | Phase 8 keeps row heights deterministic and remeasures on collapse |
| Narrow left column (216 px content) for the preset form | Cramped overrides | Phase 7 stacks the inputs |
| Implementers paraphrase Cap code | Licence contamination | Work from the spec; review diffs |
| In-flight release edits in `setup_view.rs` | Merge conflict | Phase 9 starts after they are committed |

## Rollback

Each phase lands as one or more commits. Screens (Phases 4 to 9) can be reverted individually; reverting Phase 1, 2 or 3 requires reverting everything after it.

## Unresolved questions

None blocking. Cap's main card radius is confirmed in Phase 4 step 0 (default 12).
