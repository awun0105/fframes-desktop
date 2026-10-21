# Phase 4: Header, player toolbar and transport

## Context

- Cap's header (`editor_window.rs:9247-9520`) is 52 px high with `px(10)` and `gap(6)`. On the
  left are the project name and folder actions; on the right are undo and redo, Presets,
  Clips and a filled accent Export.
- Cap's player card (`:9527-9680`) has a 44 px toolbar (`px 12`, `gap 8`, "Preview" in 12 px
  medium `text_2`), the stage (padding 16, canvas padding 4, preview radius 6) and a 48 px
  transport bar.
- Cap's transport (`:9942-10180`) has three parts:
  - left, the clock: the current time in 13 px medium `text_1` and " / total" in `text_3`;
  - centre, absolutely placed: prev and next as 24 px boxes with 14 px `text_2` glyphs, gap
    14, and play as a 32 px round button filled `text_1` with a `card` icon;
  - right: a Split button (gray, small, radius 7), a divider, zoom out, a 72 px slider (track
    3, fill accent, thumb 12) and zoom in.
  Compact layout applies below 760 px of player width and narrow below 620.
- Studio's header is `shell/header.rs`: Create, Open, Import Rust, Export (primary), Close
  project and Settings. Its transport (`shell/transport.rs`) is a rounded pill holding
  time-readout, previous-frame, play-pause, next-frame, mute-audio, build-preview,
  cancel-preview and overflow-actions. The timeline header (`timeline/view.rs:971-1035`)
  holds the zoom and Split controls.

## Steps

1. Header: left, the icon, the title and the project path, as now. Right:
   - undo and redo `IconButton`s (`timeline-undo`, `timeline-redo`) that dispatch
     `UndoTimelineEdit` and `RedoTimelineEdit`, disabled when there is nothing to undo or redo
     or a task runs;
   - a divider, then Create, Open, Import Rust and Close project;
   - Settings;
   - Export last, filled with the accent like Cap's.
   Keep the existing narrow-window icon-only rule.
2. Player card: radius 12, `card` background, 1 px `line` border, card shadow. Toolbar 44 px
   high, keeping the current contents (preview badge, `canvas-hint`, audio note) in Cap's
   type and spacing. Stage padding 16, preview radius 6.
3. Transport: a flat 48 px bar inside the card in place of the pill.
   - Left, the clock in Cap's style.
   - Centre, previous-frame, play-pause (the 32 px round button) and next-frame, keeping
     their ids and frame-step meaning.
   - Right, mute-audio, then a divider, then Split (`timeline-split`, which cuts at the
     playhead), then a divider, then zoom out, the slider, zoom in and Fit (`timeline-zoom-*`,
     `timeline-fit`), then a divider, then build-preview or cancel-preview, then
     overflow-actions.
   The zoom handlers stay on the timeline entity and are called through it. Keep the compact
   and narrow breakpoints; the slider goes away first, as in Cap.
4. Cross-entity wiring:
   - The undo and redo state is already in the timeline snapshot (`can_undo`, `can_redo`,
     `timeline/view.rs:936-1045`, where the buttons sit today). Move the buttons, and
     notify the cached header section when the snapshot changes.
   - The transport's zoom slider reads the timeline entity's zoom. The player section
     observes the timeline entity, so Ctrl-wheel zoom on the timeline also moves the slider.
   - Move the `timeline-fit` bounds probe (`observation.rs:308`) with the button.
5. Icons: add the `prev`, `next` and `scissors` Lucide glyphs if they are missing, through
   `app/src/assets.rs` and `Icon::ALL`.

## Validation

`cargo test -p studio-ui` (the icon test, transport parity). Check by hand that Split, zoom,
undo and redo do the same as their keyboard shortcuts.
