# Phase 3: Tabbed sidebar

## Context

- Cap's sidebar (`editor_sidebar.rs:2455-2530`) is a card with radius 12, `card` background,
  a 1 px `line` border and the card shadow. It has a `TabRail::editor` 46 px high at the top,
  then the selected tab's body.
- Studio already has the kit `TabRail` (`kit/tab_rail.rs`), which is icon-only and shows the
  label as a tooltip.
- The drawer contents are in `shell/left_column.rs` (`render_project_tab`, `render_assets_tab`,
  `render_style_tab`, ids `left-library`, `preset-scroll`, `recents`). The agent panel
  (`agent/mod.rs:898-990`) draws its own 46 px header with the title "Agent" and tabs Chat,
  History and Project (`agent-tab-*`, `:775-777`).

## Steps

1. `left_column.rs` becomes the sidebar: a `TabRail` with id `sidebar-tabs` and items
   Agent (`Icon::Bot` or the nearest existing icon), Project, Assets and Style, with ids
   `sidebar-tab-{label}`. Below it is the selected body: the agent section, or the existing
   tab contents without the drawer's own title row or close button. Remove `NavRail` from the
   shell, and delete `kit/nav_rail.rs` if nothing else uses it (including the gallery).
2. Attention dot on the Agent tab: shown while a permission card or a question is pending,
   or a turn is running, and another tab is selected. When a new permission card arrives,
   select the Agent tab. Add a `dot` option to `TabRailItem` instead of drawing it in the
   shell.
3. Agent panel header: drop the "Agent" title, since the sidebar tab names it, and keep one
   row with the Chat, History and Changes sub-tabs (the third label was "Project"; its id
   stays `agent-tab-Project`) and the settings button. Keep `probed_button` unchanged.
4. Sidebar card styling follows Cap: radius 12, `card` background, 1 px `line` border, card
   shadow, body padding 16.

## Validation

`cargo test -p studio-ui`; manually switch tabs during playback and check that the
transcript does not re-render (the existing observation render counters).

## Risk

`x11_shell.rs` clicks `agent-tab-Project` by measured bounds. The id stays the same, so only
label assertions, if there are any, need updating.
