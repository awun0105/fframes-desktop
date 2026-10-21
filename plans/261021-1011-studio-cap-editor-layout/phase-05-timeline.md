# Phase 5: Timeline restyle

## Context

- Cap's timeline constants are in `editor_timeline.rs:61-164`: card border 1; padding 11
  left/right, 9 top, 11 bottom; gutter 104 with an 8 px gap; ruler 26; ticks 8 and 5; track
  44; row gap 6; band radius 8; tile 22 with a 4 px inset; segment radius 8, a 3 px accent
  bar, label padding 13 / 10; minimap 10.
- Studio's timeline is in `timeline/view.rs`, which has a header row with "Timeline", zoom and
  Split, and the lanes `Scenes` and `Audio`. The other files are `segment.rs` (`track_row`,
  segment box), `ruler.rs` and `playhead.rs` (a 1 px line and a 10 px pentagon head).

## Steps

1. Remove the timeline header row, because its controls moved to the transport in phase 4.
   The minimap stays.
2. Apply Cap's paddings, track height, row gap and ruler label format (`0:01`).
3. Track gutter: a 22 px rounded tile with the lane icon on `ctl`, then the lane name in 12 px
   `text_2`, as in Cap's "Video" and "Zoom" rows.
4. Segments: fill with the track colour at 13 %, border at 34 %, a 3 px accent bar on the
   left, the label in `text_1` with the duration after it in `text_3` (for example
   "Intro 3.0s"), and a stronger border when selected.
5. Playhead: a 1.5 px line in `playhead`, with a 10 px filled circle on the ruler in place of
   the pentagon.

## Validation

`cargo test -p studio-ui timeline`, plus the timeline parity tests (`tests/parity/timeline.rs`;
positions may move with the new paddings).
