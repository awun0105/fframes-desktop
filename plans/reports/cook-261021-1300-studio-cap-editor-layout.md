# Studio in Cap's editor layout: delivery report

Plan: [261021-1011-studio-cap-editor-layout](../261021-1011-studio-cap-editor-layout/plan.md). All six phases are implemented on the uncommitted working tree of `main`. Nothing was committed.

## Outcome

The Studio window now follows Cap's editor layout:

- The header is 52 px high. It holds the title, undo and redo, a divider, Create, Open, Import Rust, Close project, Settings, and Export last with the accent fill.
- The player card and a 416 px tabbed sidebar card share a row. The sidebar's tabs are Agent, Project, Assets and Style. It keeps its grip, a 320 to 520 px range, and an attention dot on the Agent tab.
- A full-width timeline card sits below them, then the status bar.
- The transport keeps Cap's three parts: the clock, centred playback with a round play button, and the tools on the right. The tools are mute, Split, Cycle, zoom out, the slider, zoom in, Fit, build and cancel, and the overflow menu.
- The timeline is restyled. The header row is gone, each lane has a 22 px tile in its gutter, and segments are tinted with a 3 px accent bar, the label and the duration after it. The playhead is a 1.5 px line with a 10 px round head.
- The agent panel's "Project" sub-tab is renamed "Changes"; its id stays `agent-tab-Project`. NavRail is removed.

## Evidence

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean on Windows.
- `cargo test -p studio-theme -p studio-ui` passes: 16 theme tests, 476 UI unit tests and 88 of 88 parity tests. `cargo test --workspace --no-run` compiles, and the `ui_gallery` example builds.
- There are two new parity tests:
  - `timeline::the_transport_and_header_buttons_edit_the_timeline` clicks Split, Undo, Redo and Zoom in. It fails when the header-section notify is removed, which I checked.
  - `window::send_stays_in_a_short_sidebar_while_the_guidance_shows` fails on the previous chat layout, which I checked.
- Screenshots of the running app are in [screenshots-261021-cap-layout](screenshots-261021-cap-layout/). They show the plan's logical 1280×800 and 1920×1080 sizes in light and dark. The display is at 150 %, so the windows were 1920×1200 and 2880×1620 physical pixels.

## Decisions taken during delivery

- **Transport breakpoints differ from Cap's 760/620.** Studio's tool cluster is wider than Cap's: it adds mute, Cycle, Build, Cancel and the overflow menu. I measured it in the parity harness:

  | Mode | Tools width | Player width needed |
  | --- | --- | --- |
  | Full: slider, labelled Split and Build | 566 px | 1290 px |
  | Compact: icons only, no slider | 333 px | 822 px |
  | Narrow: no zoom group | 214 px | 584 px |

  The playback group is 108 px wide. Playback stays centred while the tools fit half the bar, so the thresholds are 1290 px for Full and 820 px for Compact, and the total time hides below 620 px. In the real app a 1280×800 window gives the player about 824 px, so zoom and Fit stay visible there. The measured widths are recorded in `shell/transport.rs`.
- **Cancel build is always an icon**, so that Split and Build can keep their labels in Full mode.
- **`ACTIONS_W` is 680.** The labelled header cluster, undo to Export, measures 652 px in the test platform.
- **Chat tab in a short sidebar.**
  - With no conversation, the transcript's minimum height drops to 72 px; the setup guidance scrolls inside it.
  - The tab scrolls instead of clipping when even the minimums do not fit.
  - At a logical 1280×720 this keeps Send visible; before the change it was cut off. Wheel scrolling over the transcript still scrolls the list, which the existing `agent.rs` test covers.

## Files reset or backed up

- The uncommitted brand work in `studio-theme` (`palette.rs`, `lib.rs`, `elevation.rs`, `metrics.rs`) and `shell/player.rs` was reset to HEAD. HEAD has Cap's port; the brand work had the dotted stage grid and a transport wrapper.
- The full baseline is kept:
  - `baseline-261021-staged.patch`, `baseline-261021-unstaged.patch` and `baseline-261021-status.txt`, in this folder;
  - the brand palette, in `brand-palette-261021.rs.txt`.

## Things you should know

- **Deleted files I did not delete.** Six files in `landing/studio/brand/` show as deleted in the working tree:
  - `fframes-studio-icon-blue-v1.png`
  - `fframes-studio-logo-blue-v1.png`
  - `fframes-studio-logo-concept-v1.png`
  - `icon-blue-v1-prompt.md`
  - `logo-blue-v1-prompt.md`
  - `logo-concept-v1-prompt.md`

  They are not in the baseline status. The folder changed at 11:29 today, and nothing in this session or the other recent session log on this machine touched them. They are still in HEAD, so `git checkout HEAD -- landing/studio/brand` restores them. I left them alone.
- **Extra project opened.** For the screenshots I also opened `C:\Users\Administrator\studio-hand-checks\02-scenes-and-audio`, which the plan did not name, because Hello World has no scenes. Its preview build wrote to `%LOCALAPPDATA%\fframes-studio`.
  - The script switched `app-settings.json` to light or dark for each pass and restored the original file each time; it is back to `system`.
  - Every app instance it started was stopped.
- **`app/tests/x11_shell.rs`** makes no assumptions about the rail, drawer, agent column or `timeline-fit`. It compiles but cannot run on Windows.
- **Flaky playback test.** `transport::playback_renders_the_player_and_leaves_the_other_sections` failed on the untouched baseline as well. It passed in every later run here, so it is flaky, not fixed.
- **Journal.** The journal entry was created with `ak journal create`.

## Differences from Cap that remain

These are all intended:

- The Cap-only controls listed in the plan's non-goals: Auto/Crop/Frame, Full/Half/Quarter, the Background and Layout sidebar pages, and the Video and Zoom track types.
- Studio-only controls: mute, Cycle, build and cancel preview, the overflow menu, and the audio-output buttons in the player toolbar.
- The sidebar tabs show the agent and project pages instead of Cap's editing pages.

## Unresolved questions

- Should the `landing/studio/brand` files stay deleted?
- Are the measured transport breakpoints (1290 and 820) acceptable in place of Cap's 760 and 620?
