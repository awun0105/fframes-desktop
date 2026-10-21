# Studio pages: scroll animation research and design

Date: 2026-10-20 · Scope: `landing/studio/index.html`, `landing/studio/how-it-works.html` · Status: six experiments implemented (see Implementation), not committed

## Implementation

The open questions were defaulted to this report's recommendations:

1. **Shipped:** hero (assemble), loop, filters, select, timing and swap. Stack-sdk, stack-agent,
   fence, chain and proc stay static.
2. **Hero:** assemble.
3. **Clock:** scrub at 900px and wider where `view()` and `timeline-scope` are supported.
   Play once elsewhere. Figures that scroll sideways (`.fig.overflows`, every figure at phone
   width) stay still.
4. **Pages:** both, since the shipped set spans them.

What changed:

- **`scripts/build-studio-figures.mjs`:** adds an `anim(kind, s, e, markup, vars)` group helper
  and a `playhead()` helper. `stack()` takes an optional per-layer `motion`. The six figures
  emit their choreography. The focused stacks and the optional figures are byte-identical to
  before.
- **`landing/studio/*.html`:**
  - regenerated;
  - a `.fig-clock` span in the five view-clocked figures;
  - `data-clock="page"` on the hero.
- **`landing/studio/studio.css`:** the engine (keyframes, kinds, scrub and play rules), all
  inside `prefers-reduced-motion: no-preference`.
- **`landing/studio/studio.js`:**
  - sets `motion-scrub` or `motion-play` on `<html>`, using one feature test, and re-evaluates
    on width and motion-preference changes;
  - an IntersectionObserver for play once.

  A change from the design: CSS animates nothing until the script sets a class. JS is the
  single place that decides, so CSS and JS cannot disagree, and a page without JS is static
  in every mode.
- **Lab:**
  - skips runtime tagging for figures that already carry hooks;
  - no longer loads `studio.js`, so its toolbar alone controls it.

Verified in headless Chrome against screenshots of the live pages taken before the change:

| Check | Result |
| --- | --- |
| Generator run twice | Second run changes nothing |
| Motion off (reduced motion), 1280px | All 11 figures pixel-identical to before |
| Motion off, 390px | Identical, or channel differences of 10 or less, at most one pixel above threshold (deterministic, not noise) |
| Motion on, 1280px | `motion-scrub`, 52 and 20 live animations; mid-scroll states correct on all six; hero shows ghosts at load and settles by 500px with the figure fully in view |
| Scrub final frames, 1280px | Same as the lab: geometry exact, edge antialiasing differs on loop, timing and hero (110–1530 px), effectively exact on select, filters and swap |
| Motion on, 899px | `motion-play`; the loop plays when scrolled to |
| Motion on, 390px | `motion-play`, but every figure overflows, so all stay still and match motion off |
| Print, motion on, no scrolling, 899px and 1280px | Identical to print with reduced motion on both pages. The engine is gated on `@media screen`; without the gate the same check fails |

Not verified: Safari 26 and Firefox (only Chrome was available), and a page with JavaScript
disabled (static by construction: no class, no animation).

## Outcome

Every figure on both Studio pages can be animated on scroll with CSS, from the SVGs we already
generate, without redrawing anything. A working lab page shows one experiment per figure, with
toggles for the open decisions:

```
node plans/261020-1617-studio-scroll-experiments/prototype/build-lab.mjs
start plans/261020-1617-studio-scroll-experiments/prototype/index.html
```

The lab copies the generated figures out of the live pages and tags them at runtime
(`scroll-lab.js`). The live pages and `scripts/build-studio-figures.mjs` are unchanged.

- **Toolbar:** Scrub or Play once, Hero assemble or pinned, Motion or Static.
- **URL switches:** `?motion=on` forces motion when the OS asks for reduced motion; `?tag=off`
  shows the untouched SVGs.

## What we have

- **11 figures in two pages.** All are isometric or flat line drawings written by
  `scripts/build-studio-figures.mjs` between `<!-- fig:name -->` markers. The generator rewrites
  them on every run, so animation hooks must come from the generator, not hand edits.
- **One visual vocabulary in `studio.css`:**
  - `f-line`, `f-lead`, and the tone pairs `t-*`/`s-*`;
  - `muted` (opacity .38) and `see-through` (fill-opacity);
  - dashed `f-faint` guides.
- **Layout:** figures sit in `.fig > .scroll` (`overflow-x: auto`). Below 900px they scroll
  sideways at a `--min` width.
- **Existing pattern:** the home page already uses a one-shot IntersectionObserver reveal
  (`landing/index.html`, footer wordmark).

## Research findings

**Browser support.** Scroll-driven animations (`animation-timeline: view()/scroll()`) run
natively in Chromium and Safari 26+. Firefox still has them behind a flag in stable as of the
mid-2026 sources. That is why the design starts every animation from the finished drawing: a
browser without support shows today's static figure.

**Verified in headless Chrome during this work:**

| Check | Result |
| --- | --- |
| Anonymous `view()` inside `.scroll` (overflow-x) | Works in Chrome (resolves to the page). Not relied on. |
| Named timeline + `timeline-scope` | Works. One clock per figure, readable by the SVG. |
| `animation-range: cover calc(X% + var(--s) * Y%)` | Works. Per-element staggering needs no JS. |
| `scroll(root)` with a px range | Works. Used for the hero, which is on screen at load. |
| `pathLength="1"` on `<line>` and curved `<path>` | Works. Lines draw on without measuring. |
| Headless Chrome reports `prefers-reduced-motion: reduce` | The static fallback engaged by itself. |
| Tagging the SVGs (wrapping groups, moving text) | Pixel-identical to the untouched SVGs for all 11 figures. |
| Final frame, play once | Pixel-identical for 8 figures; the other 3 differ by at most 42 antialiased edge pixels. |
| Final frame, scrub | Same geometry and colours. Edges of animated elements antialias slightly differently: 0–1500 pixels per figure, nothing visible at 1x. |
| 390px viewport | The parts of select and timing that carry the point animate while scrolled off to the side. |

**Problems found while building the lab, and their fixes:**

1. **Ease-out hides progress.** At 74% of a long draw window, an ease-out curve has drawn
   about 95% of the line, so the loop line and the playheads looked finished too early.
   - Rule: anything showing progress (draws, bars, playheads) moves linearly.
   - Only things settling into place ease out.
2. **Tall figures animate below the fold.** With the figure itself as the timeline subject, a
   figure about a viewport tall ran half its choreography while its bottom half was off-screen.
   - Fix: a 1px `.fig-clock` marker at the figure's centre owns the timeline, and
     `timeline-scope: --fig` on `.fig` lets the SVG use it.
   - Every figure now gets the same scroll distance: its centre moves from 85% down the
     viewport to the middle, about 300px. Each figure is finished when centred.
3. **The hero is on screen at load.** A view timeline would show it half built.
   - It runs on page scroll instead (0 to 500px).
   - Its start state is never empty: layers start as 12% ghosts.
   - In the pinned variant the closed stack shows its bottom label dimmed, and the two labels
     that would overlap it fade in as it opens.
   - The pinned variant only pins at 900px and wider; narrower screens get assemble.
4. **Finished animations must leave no trace.** With `animation-fill-mode: both`, finished
   elements kept rendering through a separate path and antialiased differently.
   - Fix: `backwards`, so a finished animation has no effect at all.
   - The one exception is helpers that are not in the drawing (playheads). They fade out with
     their own `forwards` exit.
5. **Style collisions.**
   - Opacity keyframes would overwrite `.muted`, so each stack layer is wrapped in a new
     animated `<g>` and the inner `.muted` stays intact.
   - Draw-on is never applied to `f-faint`, which already uses a dash array.

## Design principles

1. **The static drawing is the last frame.** Animations only define start states, so no-JS,
   no support, print and reduced motion all show today's figure.
2. **Motion carries the figure's argument, never decoration.** If a motion does not explain
   something the caption says, cut it.
3. **One clock per figure, 0 to 100.** Elements own windows on it (`--s`, `--e`), so the
   choreography is data the generator can emit.
4. **Done when centred.** Readers who stop scrolling see the full figure.
5. **Cheap properties only.** Transforms, opacity, `stroke-dashoffset` and one `fill`
   animation. On SVG children these repaint rather than run on the compositor, which is cheap
   at these sizes.

## Experiments

Ranked by how much the motion explains. The lab implements all of them.

| # | Figure | Experiment | What the motion argues | Rank |
| --- | --- | --- | --- | --- |
| 1 | `hero` | **Assemble**: Studio's five layers settle from ghosts onto the library's three as the page scrolls | "Same three layers, five more on top" | Ship |
| 1b | `hero` | **Pinned** (≥900px): the figure sticks for about 1.4 screens; both stacks open from closed blocks, then the five layers settle one by one | Same, more theatrical | Alternative |
| 2 | `loop` | **Run the loop**: the line draws, nodes pop in order, inspect fails and the repair arc goes back to edit, "video ready" comes only after preview | "Agent finished is not video ready" | Ship |
| 3 | `filters` | **Rise through the filters**: the draft bar grows up through the planes, each plane lights as it passes, fail branches at inspect | The edit transaction | Ship |
| 4 | `select` | **Explode, click, hand over**: a flat frame separates into planes, the pointer clicks, the lead line draws, context rows fill | One click gives exact task context | Ship |
| 5 | `timing` | **Scrub the cut**: a playhead sweeps, split and trim snap in, `timeline.json` fills blue, the build stays grey | No rebuild for timing | Ship |
| 6 | `swap` | **Swap at the playhead**: compile, stage and ready fill under a moving playhead, the swap appears where they meet | Old build plays until the new one is whole | Ship |
| 7 | `stack-sdk`, `stack-agent` | **Insert the layer**: the stack opens, the focused layer slides into its slot | Studio adds this layer | Optional (repeats the hero idea) |
| 8 | `fence` | **The escape**: a child slides out through the fence, the red line draws, "refused" appears | Containment | Optional |
| 9 | `chain` | **Pixel to Rust**: cards and arrows left to right | Lookup order | Optional, low value |
| 10 | `proc` | **Wire the processes**: boxes, then outside processes, then wires, the preview protocol last | Structure | Optional, low value |

Two cross-cutting choices, both switchable in the lab:

- **Scrub vs Play once.**
  - Scrub ties the drawing to the scroll position. Play once runs the same windows on a 1.4s
    clock when a third of the figure is visible.
  - Play once is also the fallback where scrub is unsupported (Firefox), using the same
    keyframes.
  - **Recommendation:** scrub at 900px and wider. Below 900px, where figures scroll sideways,
    use play once or static.
- **Hero assemble vs pinned.** Assemble keeps the page flow unchanged. Pinned adds about 1.4
  screens of scroll above the first section.

## Production path (after a pick)

1. **Generator.** `build-studio-figures.mjs` emits the hooks the lab adds at runtime:
   - group each stack layer (slab plus leader) in one `<g>`;
   - add `class="a a-<kind>"` and `style="--s:..;--e:..[;--dx/--dy]"` to animated parts;
   - add `pathLength="1"` on drawn lines;
   - add playheads as extra elements that are hidden without motion.

   Rerun, then confirm two things:
   - the diff touches attributes and grouping only;
   - with motion off, the figures are pixel-identical (the lab's tag-on/off comparison).
2. **Clock marker.** `<span class="fig-clock" aria-hidden="true">` must be a direct child of
   `.fig`, outside `.scroll`. The generator only writes inside the `fig:` markers, which sit
   inside `.scroll`, so the span belongs in the page HTML (or is added by `studio.js`).
3. **CSS.** Move the engine part of `scroll-lab.css` (not the lab toolbar) into `studio.css`:
   - wrap it in `@media (prefers-reduced-motion: no-preference)`;
   - put the scrub rules inside `@supports (animation-timeline: view())`.
4. **Play-once guard.** Hide start states for play once only under a class that `studio.js`
   adds to `<html>` just before it starts observing, never under CSS alone. Otherwise a Firefox
   visitor whose script fails or is blocked sees empty figures. Scrub needs no guard, because
   CSS both hides and reveals.
5. **JS.** About 15 lines in `studio.js`:
   - the guard class;
   - an IntersectionObserver that adds `is-visible`;
   - the `.fig-clock` insert if it is not written in the HTML.
6. **Verify.**
   - Chrome, Edge, Safari 26 and Firefox (fallback).
   - A 390px viewport.
   - Print preview.
   - Reduced motion.
   - CLS 0 (transforms do not move layout).
   - Final frames against tag-off.

**Measured weight:**

- The hooks add 8.3 KB raw but only 1.1 KB gzipped across all 11 figures.
- The engine CSS is under 2 KB gzipped.
- The runtime-tagging JS of the lab is not needed in production.

**Acceptance criteria:**

- With motion off or unsupported, the figures are pixel-identical to today's.
- Play-once final frames match within edge antialiasing.
- Every figure is complete when centred.
- Nothing animates under `prefers-reduced-motion: reduce`.
- Nothing is hidden if `studio.js` does not run.
- No new dependencies.
- Under 4 KB gzipped added to the two pages.

## Risks

- **Safari 26 parity** for `timeline-scope` with named view timelines on SVG children is not
  verified; only Chrome was available. Check it before shipping scrub. The fallback is play
  once.
- **Scrub final frames antialias slightly differently** on the edges of animated elements
  while the timeline is active. This is not visible at 1x. Check on a 2x screen, or use play
  once if it shows.
- **The lab's runtime tagging is position-based** (attribute lookups) and breaks if the
  drawings change. That is fine for a throwaway; production emits the hooks from the generator.

## Unresolved questions

The four design questions were defaulted (see Implementation). If any default is wrong, it
is a small change: a figure's `anim` calls, the `motion` option on the hero stack, or the
`wide` test in `studio.js`.

## Sources

- [CSS Scroll-driven Animations without JavaScript (ICS Media)](https://ics.media/en/entry/230718/)
- [Replace your scroll-listener JS: CSS scroll-driven animations go cross-browser](https://www.buildmvpfast.com/blog/css-scroll-driven-animations-replace-js-2026)
- [Mozilla Connect: implement CSS scroll-driven animations](https://connect.mozilla.org/t5/ideas/implement-css-scroll-driven-animations-animation-timeline/idc-p/60742)
- [The latest in CSS and web UI: I/O 2024 recap (Chrome)](https://developer.chrome.com/blog/new-in-web-ui-io-2024?hl=en)
- Local checks: headless Chrome tests of timelines, ranges and `pathLength`; mid-animation
  screenshots of every experiment at 1280 and 390px; final-frame pixel comparisons. All were
  taken during this work.
