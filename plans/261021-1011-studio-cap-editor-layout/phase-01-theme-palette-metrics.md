# Phase 1: Theme, Cap palette and metrics

## Context

- Cap's editor palette is in `Cap/apps/desktop-gpui/src/theme.rs:469-508`. Light: window `f1f1f3`,
  card `ffffff`, card_2 `f6f6f7`, stage `e9e9ec`, line `rgba(0,0,0,.075)`, line_strong
  `rgba(0,0,0,.12)`, text `1c1c1e` / `6e6e75` / `a0a0a8`, ctl `rgba(0,0,0,.045)`, accent
  `007aff`, playhead `ff3b30`. Dark: window `131315`, card `1b1b1e`, stage `121214`, accent
  `0a84ff`, playhead `ff453a`; read the remaining dark roles there.
- Studio's committed `palette.rs` (HEAD, `585ac39` "port the Cap theme") holds those values.
  The working tree replaced them with the blue brand palette (uncommitted).
- Cap's segment tints are in `apps/desktop/src/styles/theme.css:42-62` (`--ed-seg-tint` 13 %,
  `--ed-seg-line` 34 %). Track colours are in `editor_timeline.rs:178-188` (clip `3b82f6`,
  audio `22c55e`, scene `8b5cf6`).

## Steps

1. Save the baseline as described under rollback in `plan.md` (the staged patch, the
   unstaged patch and the porcelain status), and copy the working-tree `palette.rs` to
   `plans/reports/brand-palette-261021.rs.txt`.
2. Restore `EditorPalette` and its contrast tests from
   `git show HEAD:desktop/crates/studio-theme/src/palette.rs`. That file's only commit is the
   Cap port, so don't retype hex values; check them against Cap's `theme.rs`. Also diff the
   unstaged `elevation.rs`, `lib.rs` and `metrics.rs` against HEAD. If the card shadows
   changed with the brand, restore Cap's. Leave the non-editor palette structs (`palette.rs:90`, `:147`)
   and the rest of the WIP alone unless they only exist for the brand editor colours.
3. Set `track_color` and `segment_colors_for` to Cap's track colours and tint/line mixes:
   scenes use the clip blue, audio the green.
4. Metrics (`metrics.rs`): `header_h` 52, `toolbar_h` 44, `transport_h` 48, `track_h` 44,
   `tab_bar_h` 46 (unchanged), new `sidebar_w` 416 replacing `agent_w` (min 320, max 520
   unchanged), remove `rail_w` and `drawer_w`, card radius 12 for the three main cards
   (`radius_card`, check other users of the token first), timeline paddings 11 / 9 / 11 and
   `track_tile` 22. Update `metrics_follow_editor_density_scale`: the values change on
   purpose.
5. Update the contrast tests in `studio-theme` to record Cap's figures again (the design doc
   lists white on accent 4.02 light / 3.65 dark, light `text_2` on `window` 4.49, light
   `text_3` on `card` 2.60).

## Validation

`cargo test -p studio-theme`; `cargo build -p studio-ui` compiles after the removed metrics
are replaced at their call sites (temporary uses are fixed in phases 2 and 3).

## Risk

The brand palette is someone's uncommitted work. Step 1 keeps a copy; tell the user where.
