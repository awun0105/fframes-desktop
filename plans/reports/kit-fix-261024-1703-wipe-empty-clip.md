# Kit fix: empty clip on the first frame of a wipe

Status: done

## Fixed

- **`Wipe`:** on the first frame of a transition (progress 0), the entering side was clipped
  to a rectangle with zero width or height, which `inspect` reports as invalid SVG. `Wipe`
  now draws nothing until something is uncovered, and draws the content unclipped at
  progress 1. This is the same behaviour as `Iris` and `CircleWipe`.
  - `components/fframes-kit/src/transitions/wipe.rs`
  - `components/gallery/tests/transitions.rs`: frame 15 now draws two rectangles, not six.
    The new `the_first_frame_of_a_wipe_has_no_empty_clip` checks that `inspect` reports
    nothing invalid. Both tests fail without the fix.
- **Gallery `BenchHandWritten`:** the accent bar was a zero-height `<rect>` before it grew.
  It is now left out until then, like `LowerThird`.
  (`components/gallery/src/scenes/core.rs`)

A sweep of every gallery frame (`inspect --all-frames`, 24,273 frames) finds nothing invalid.
Five warnings remain, all text crossing the canvas edge in the middle of a `Slide` or
`SceneOverlap`, which is expected.

## Not a bug: `Glow` and `DropShadow` filters

The phase 11 report ([kit-phase-11-261024-1414.md](kit-phase-11-261024-1414.md)) left one
item open: that `Glow` and `DropShadow` blur in linear light. That is not the case. The
converted SVG of `Glow@50%` carries `color-interpolation-filters="sRGB"` on every primitive
(`feGaussianBlur`, `feOffset`, `feFlood`, `feComposite`, `feMerge`), because usvgr inherits
the attribute from the `<filter>` element. No change is needed in `decor/filters.rs`.

## Checks

- Clippy, `-D warnings`:
  - the workspace with `--all-features`;
  - the kit with `skia`, `biology`, `biology,3d`, `cs`, `chemistry,3d`, `chemistry` and with
    no features.
- Kit tests:
  - all features: 313 unit, 90 doc and 2 shader tests;
  - `skia`, `biology`, `biology,3d` and no features.
- Gallery tests pass.
- The wasm bridge checks.
- `node scripts/e2e.mjs cpu`: 87 snapshots, all 0.000 %.
