# Studio home page: better figure animations

Status: implemented, uncommitted · Branch: main (uncommitted, alongside existing user edits)

## Outcome

The five figures on `landing/studio/index.html` explain their operation as scroll scenes:
the "Anatomy of an edit" hero and the four product chapters (project, timing, SDK, export).
The facts stay those of the existing titles, descriptions, captions and chapter copy.

## Approach

1. Share the scene engine instead of copying it. Move the generic `.sc*` rules and the
   `scroll-cue` rules from `how-it-works.css` into `home.css` (both pages inline it);
   gate them on `:is(.story-motion, .internals-motion)`. Move token placement into
   `scroll-motion.js` (`studioScrollMotion.scene`), used by `how-it-works.js` and `home.js`.
2. Product figures (`scripts/build-studio-features.mjs`): rebuild as `--p` scenes with
   `studio-scenes.mjs` helpers, a step narration line and the scroll cue label.
3. Hero (`scripts/build-studio-workflow.mjs`, `home.js`, `home.css`): keep the isometric
   planes, the 680×690 viewBox and the step variables; replace the block arrows with
   labelled chips on straight paths, move the request note off the draft, add the click,
   the diff strike/insert, gate progress bars and the accepted flash.
4. `node scripts/build-studio-workflow.mjs && node scripts/build-studio-features.mjs &&
node scripts/build-studio-delivery.mjs`, then prettier.

## Contracts to keep

- Features check: `.pf-output`, `.pf-clip-c`, `.pf-tool-two`, `.pf-render-bar` change
  opacity/transform with scroll, hold and reverse exactly; no `style` on figures under
  reduced motion; `.pf-render-bar` transform `none` without JS; SVG min-width scroll.
- Landing check: `.title-before`/`.title-after` 1/0 until step 4; `.gate-build-label` 0 at
  step 2 and 1 at step 3; `.gate-inspect-label` 1 at step 4; final figure fits 1440×1000.
- Phones keep the compact cards. No transitions; scroll is the only clock.

## Acceptance

- Screenshots at 1440 (several positions), 390 and without JS look complete.
- features, landing, motion, performance checks: no failures beyond the baseline
  (features 23, landing 6, motion 0, performance 0, run on this tree before edits).
- how-it-works motion and diagrams checks unchanged after the shared move.

## Result

- Shared engine: `.sc*` and scroll-cue rules now live in `home.css`; token placement is
  `studioScrollMotion.scene` in `scroll-motion.js`. How it works rendered pixel-identical
  before and after the move (24 screenshots across its 12 figures).
- Product figures rebuilt as scenes with a step narration line: project (files, editor
  and Git, Studio workspace, build, preview), timing (split, move, trim; timeline.json
  REV 1→4; Rust unchanged, no rebuild), SDK (tools drop in, install bar, example build,
  worker/frame/audio checklist, current preview replaced), export (worker formats,
  format/size/frame rate, render with cancel, frame counter, saved file).
- Hero: click ring on the selection, `title_key` / `candidate` / `accepted` chips that
  travel between the moved planes, highlighted key line, struck "Hello." and growing
  insert, gate progress bars, request note moved off the draft, green accepted outline.
  Every value holds still near a step position (the 390px reverse check depends on it).
- Formatting `how-it-works.html` with prettier split the renderer's `xml:space="preserve"`
  code lines; they are restored and wrapped in a `prettier-ignore` group. After that, all
  24 How it works screenshots matched the ones taken before this work again.
- Checks on the final build, same headless Chrome as the baseline: features 21 (baseline
  23; SDK now changes at 390/320), landing 6, motion 0, performance 0 (all baseline);
  how-it-works diagrams 37, internals 10, scoped details 12 (unchanged).

## Follow-up: bigger hero

- The hero was height-bound (680×690 viewBox) and left a wide empty band in its column.
  The planes now step down and to the left (viewBox 660×532) instead of stacking
  vertically, so the drawing fills the column width: about 1.25× the previous scale at
  1440×1000. Labels sit beside each plane's right corner, the request note at top left,
  the foot line right-aligned. Chips read their endpoints from `data-from`/`data-to`.
- `.edit-diagram` uses `height: auto` with the clamp as `max-height`, so a width-bound
  figure (1024) no longer reserves empty height above and below it.
- Checks after the change: landing 6, features 21, motion 0, performance 0 (unchanged).
