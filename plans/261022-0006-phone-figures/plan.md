# Studio pages: animated phone figures

Status: done, one open decision (hero height at 761-1100px) · Branch: main (uncommitted, alongside existing user edits)

## Outcome

Below 760px both Studio pages (`landing/studio/index.html`, `how-it-works.html`) show real
figures instead of text cards: a portrait SVG per figure, driven by the same scroll state
as the desktop figure. 16 figures: the hero and four product chapters on the home page,
and the 11 How it works figures that use cards today (the renderer figure already has a
phone layout).

## Approach

- `scripts/studio-mobile-diagrams.mjs` keeps its role (key per figure, rerunnable marker
  injection before `<figcaption>`) but returns a portrait SVG (`viewBox` 340 wide, text
  ≥ 12 units) built with the `studio-scenes.mjs` helpers. The `.compact-*` cards and their
  CSS go away.
- Scenes inherit the figure's `--p`; every phone id carries a `-m` suffix so tokens follow
  phone paths. Only one variant is displayed, so only one is in the accessibility tree.
- Hero: fixed planes in a ~340×300 box that fits the sticky 40svh slot; reuses the global
  hero part classes. `home.js` adds unitless `--context-k`, `--candidate-k`, `--result-k`.
- Gates and readiness: the desktop JS queries are scoped to the desktop SVG; the phone
  versions read one figure-level progress variable.

## Acceptance

- 390 and 320 screenshots of every figure at several scroll positions look complete;
  no-JS and motion-off show the finished state.
- A phone check: parts change with scroll, hold still, reverse exactly at 390 and 320.
- Desktop 1440 screenshots identical before and after on both pages.
- Checks do not get worse: landing 7, features 21, motion 0, performance 0, diagrams 37,
  internals 10, scoped details 12.

## Result

- 16 phone figures; phone check 108/108 at 390 and 320 on both pages (change, hold, reverse,
  only the phone drawing shown, no overflow). No-JS and motion-off show the finished state.
- Desktop 1440: 34/34 screenshots byte-identical to the baseline.
- Checks: landing 7, features 21, motion 0, performance 0, diagrams 37 (same sets as the
  baseline); internals 10 (baseline 11 minus "Accepted figure fits at 390px", which now passes);
  scoped details 13: "Homepage enlarged diagram at 768" fails (box 314 < 500). Regression from
  the hero enlargement (`height` to `max-height` at 761-1100px). The drawing is 390x292 either
  way; the old 639px box passed with 180px blank above and 166px below it. Open: restore the
  letterbox, update the threshold, or redesign the tablet layout.
- Phone SVGs are named by the wide drawing's `<title>` (`aria-labelledby`), so each figure
  keeps one `<title>`.