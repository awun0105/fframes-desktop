---
phase: 5
title: "Trim, split, reorder and snapping"
status: pending
priority: P1
effort: "2.5-3.5d"
dependencies: [2, 4]
---

# Phase 5: Trim, split, reorder and snapping

## Goal

Users trim clip edges, split at the playhead, reorder by drag, snap to the playhead and neighbouring edges, and undo or redo from the keyboard.

Effort was 3-4d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md), which found that Cap does snap, so snapping is a port rather than new design.

## Port sources

Cap `editor_window.rs`: drag state, `split_at_playhead`, undo and redo wiring. Snapping ports from Cap: `arm_drag_snap` (`editor_window.rs`) builds a one-dimensional target list once when a drag arms (zero, the playhead, the total and every other segment's edges; Shift bypasses, `DRAG_SNAP_PX` = 8), `snap_dragged_time` applies it, and `snap_split_time` (`editor_edits.rs`, `SPLIT_SNAP_PX` = 7) snaps splits.

## Tasks

1. Trim handles send `TimelineBeginGesture`, `TimelineTrim` per move and `TimelineEndGesture` on release.
2. Split at playhead as an action with a shortcut that does not collide with today's `s` scene cycling.
3. Reorder by dragging a clip along the track, with an insertion marker; sends `TimelineMove`.
4. Snapping to playhead and clip edges, computed once at gesture start so targets do not move during the gesture; a held modifier disables it.
5. Undo and redo bound to Ctrl+Z and Ctrl+Shift+Z (Cmd on macOS).
6. Disabled with a visible reason when the project is not migrated, an agent task holds it, or publication is blocked.
7. gpui tests for each gesture, snapping, and undo through the real engine.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Drags measure the track's bounds with a zero-size `canvas` in prepaint and store them in the view. During a drag a full-window transparent layer receives mouse move and up events, so the gesture survives leaving the track (Cap's editor drag layer in `editor_clips.rs` and `editor_panels.rs`). End the gesture, and send `TimelineEndGesture`, on mouse up anywhere and on window blur.
2. Undo and redo are bound in the timeline and workspace contexts; a focused text input or the composer keeps Ctrl/Cmd+Z (M2 phase 4 keymap). Test both.

## Todo

- [ ] Drag layer and gesture end on blur; undo key-context tests

- [ ] Trim
- [ ] Split
- [ ] Reorder
- [ ] Snapping
- [ ] Undo and redo bindings
- [ ] Disabled states
- [ ] Tests

## Success criteria

The roadmap's timing Done-means item holds by hand on this VM; gpui tests cover every gesture; parity tests green.
