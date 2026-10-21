---
title: "Studio editor in Cap's layout: player and tabbed sidebar over a full-width timeline"
description: "Rebuild the Studio window to match Cap's editor: header, player card with toolbar and transport, a 416 px tabbed sidebar (Agent, Project, Assets, Style), a full-width timeline card below, and Cap's colours."
status: completed
priority: P2
effort: "4-6 engineer-days"
branch: main
tags: [desktop, ui, layout, theme, port, cap]
blockedBy: []
blocks: []
created: 2026-10-21
---

# Studio editor in Cap's layout

Made with `ak:xia --port`. The source is Cap's editor at `C:\Users\Administrator\code\Cap`, commit
`a32e4806b`, read in `apps/desktop-gpui/src` (`editor_window.rs`, `editor_sidebar.rs`,
`editor_timeline.rs`, `theme.rs`, `ui/tab_rail.rs`). The user's reference is a screenshot of
Cap's editor (header, player with Auto/Crop/Frame and Full/Half/Quarter, Background/Layout
sidebar, timeline with Video and Zoom tracks).

This plan replaces the "keep the agent column" recommendation in
`plans/reports/xia-cap-desktop-editor-ui-compare.md`. The user chose Cap's layout.

## Outcome

```diagram
┌ header 52: title · path ············ undo redo · Create Open Import Close · ⚙ · [Export] ─┐
│┌ player card (flex) ──────────────────────────────┐ ┌ sidebar card 416 ─────────────────┐ │
││ toolbar 44: preview badge · canvas hint · audio  │ │ tab rail 46: [Agent][Proj][Ast][S] │ │
││                                                  │ │                                    │ │
││                    stage                         │ │  agent panel (Chat/History/Changes)│ │
││                                                  │ │  or the Project / Assets / Style   │ │
││ transport 48: 0:00 / 0:06 · ⏮ ▶ ⏭ · mute ·      │ │  drawer contents                   │ │
││   Split · − ═○═ + Fit · Build preview · ⋯         │ │                                    │ │
│└──────────────────────────────────────────────────┘ └────────────────────────────────────┘ │
│                           ═══ grip ═══                                                       │
│┌ timeline card, full width ───────────────────────────────────────────────────────────────┐ │
││ ● ruler 0:00 0:01 …   (red playhead with round head)                                      │ │
││ [▦ Scenes] │ Intro 3.0s │ Main 6.3s │                                                     │ │
││ [♪ Audio ] │ track.mp3 ~~~~~~~~                                                          │ │
│└──────────────────────────────────────────────────────────────────────────────────────────┘ │
└ status bar 28 ───────────────────────────────────────────────────────────────────────────────┘
```

## Decisions

| Decision | Cap | Studio today | Chosen (source) |
| --- | --- | --- | --- |
| Right column | 416 px config sidebar, 6 icon tabs | Rail + drawer on the left, agent column on the right | Cap's sidebar with tabs Agent, Project, Assets, Style; left rail removed (user) |
| Cap-only controls | Auto, Crop, Frame, Full/Half/Quarter, Presets, Clips, Background, Zoom track, Generate, Add track | none | Not added. Only Studio's real controls go into Cap's slots (user) |
| Colours | `EditorPalette` grey, `#007aff`, red playhead | Uncommitted blue brand palette | Cap's editor palette, light and dark (user) |
| Baseline | | Uncommitted layout A and theme edits | Build on the working tree as it is; nothing is committed or stashed (user) |
| Sidebar width | Fixed 416 | 320–520 grip | 416 by default, the grip stays and keeps 320–520, because chat benefits from width |
| Timeline width | Full window below player and sidebar | Under the player only | Full width; the split measures the whole workspace |
| Split / zoom place | Transport bar | Timeline header | Transport bar. Studio's Split cuts at the playhead; that does not change |
| Undo / redo | Header | Timeline header buttons and shortcuts | The buttons move to the header and still send `UndoTimelineEdit` / `RedoTimelineEdit` |
| Agent sub-tab "Project" | | Collides with the sidebar's Project tab | The label becomes "Changes"; the id stays `agent-tab-Project` |
| Code reuse | AGPL-3.0 | Kit ported with the owners' permission | Re-implement from the geometry; reuse Studio's own `TabRail`, `Slider`, `IconButton` |

## Dependency matrix

| Cap part | Studio equivalent | State |
| --- | --- | --- |
| `TabRail` (`ui/tab_rail.rs`) | `studio-ui/src/kit/tab_rail.rs` | EXISTS |
| `EditorPalette::light/dark` (`theme.rs:469-508`) | `studio-theme/src/palette.rs` (Cap values at HEAD `585ac39`) | CONFLICT with the brand WIP, replaced |
| Header, transport, sidebar, timeline constants | `studio-theme/src/metrics.rs` | CONFLICT, values change |
| Sidebar host switching tab bodies | `shell/left_column.rs` (rail + drawer), `shell/sections.rs` | NEW composition over existing contents |
| Full-width timeline split | `shell/layout.rs` `vertical_split`, `measure` | EXISTS, re-pointed |
| Transport with Split and zoom | `shell/transport.rs`, `timeline/view.rs` header | EXISTS, moved |
| Track chips, segment tint, round playhead head | `timeline/segment.rs`, `ruler.rs`, `playhead.rs` | EXISTS, restyled |

## Phases

| # | Phase | Files owned |
| --- | --- | --- |
| 1 | [Theme: Cap palette and metrics](phase-01-theme-palette-metrics.md) | `studio-theme/src/{palette,metrics,lib,elevation}.rs` |
| 2 | [Shell: player and sidebar row over a full-width timeline](phase-02-shell-layout.md) | `shell/{layout,sections,mod,observation,keymap}.rs` |
| 3 | [Tabbed sidebar](phase-03-tabbed-sidebar.md) | `shell/left_column.rs`, `kit/nav_rail.rs`, `agent/mod.rs` (header only) |
| 4 | [Header, player toolbar and transport](phase-04-header-player-transport.md) | `shell/{header,player,transport,overflow}.rs` |
| 5 | [Timeline restyle](phase-05-timeline.md) | `timeline/{view,segment,ruler,playhead}.rs` |
| 6 | [Tests, docs and screenshot check](phase-06-tests-docs-verify.md) | `studio-ui/tests/parity/*`, `app/tests/x11_shell.rs`, `docs/desktop/*` |

Phases run in order. 1 comes first because every later phase paints with its tokens; 2 and 3
share `sections.rs` state and run back to back.

## Ids that must survive

Parity and X11 tests look these up (`studio-ui/tests/parity/{window,timeline,transport,composer}.rs`,
`app/tests/x11_shell.rs`, `shell/observation.rs:256-308`): `agent-column` (now the sidebar card),
`agent-tab-Chat`, `agent-tab-History`, `agent-tab-Project`, `timeline-fit`, the `split` and
`drawer` telemetry keys, plus `agent-resize-handle`, `timeline-resize-handle`, `status-strip`,
`status-details`, `timeline-split`, `timeline-zoom-*`, `play-pause`, `previous-frame`,
`next-frame`, `mute-audio`, `build-preview`, `cancel-preview`, `overflow-actions`, `settings`,
`export`, the caption ids. `rail-Project`, `rail-Assets`, `rail-Style` are replaced by
`sidebar-tab-{Agent,Project,Assets,Style}`; the `drawer` telemetry key reports the selected
sidebar tab.

## Acceptance

- The window matches the diagram above at 1280×800 and 1920×1080, in light and dark mode, with
  a project open and with none (the sidebar and its Project tab stay reachable).
- Cap's palette values are in `EditorPalette`; the playhead is red with a round head.
- Every control in the window works; none is a placeholder.
- A pending permission card, a question or a running turn shows a dot on the Agent tab while
  another tab is selected. `FocusPrompt` selects the Agent tab and focuses the composer.
- Playback re-renders neither the transcript nor the drawer contents (cached sections).
- `cargo fmt --all`, `cargo clippy --workspace -- -D warnings` and
  `cargo test -p studio-theme -p studio-ui` (parity suite included) pass on Windows. No CI.
- A screenshot of the running app is compared with the user's Cap screenshot and the
  differences that remain are listed (they should be only Cap-only controls).

## Non-goals

Cap's config features (background, padding, color correction, presets, clips, zoom segments,
crop, preview quality), new track kinds, a sliding tab indicator, macOS vibrancy, saving the
splits between launches.

## Risks

| Risk | Mitigation |
| --- | --- |
| Replacing the uncommitted brand palette loses the user's WIP colours | Phase 1 copies the WIP `palette.rs` to the plan's reports folder before editing |
| The agent panel behind a tab hides permission cards | Attention dot plus auto-select of the Agent tab when a permission card arrives |
| The chat gets short at the 240 px timeline minimum | Check at 1280×800; `player_min_h` 320 also bounds the sidebar |
| Moving Split and zoom breaks timeline parity tests | Ids stay the same; tests that read positions are updated in phase 6 |

Risk score: medium. The main exposures are churn in the parity tests and overwriting the
uncommitted brand palette.

Rollback: the baseline is uncommitted and partly staged, so before any edit phase 1 saves it
in `plans/reports/`:

- `git diff --cached --binary > baseline-261021-staged.patch`
- `git diff --binary > baseline-261021-unstaged.patch`
- `git status --porcelain > baseline-261021-status.txt` (there are untracked files)

To roll back, and only after the user confirms (it overwrites their working tree), run
`git checkout HEAD -- desktop`, then `git apply --index` the staged patch and `git apply` the
unstaged patch. Then delete the files the redesign created, which are the ones missing from
the saved status.

## Handoff

Plan ready at `./plans/261021-1011-studio-cap-editor-layout/plan.md`. To implement, run
`/ak:cook plans/261021-1011-studio-cap-editor-layout/plan.md`.
