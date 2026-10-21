# How it works: redesign the remaining figures

Status: implemented, uncommitted · Branch: main (uncommitted, alongside existing user edits)

## Outcome
Every figure on `landing/studio/how-it-works.html` tells its story as a scroll-driven
scene, like the rebuilt rendering figure: one progress value per figure drives what
appears, what moves along which connector, and a short step readout. The facts each
figure states stay the same (titles, captions, descriptions are the accuracy contract).

## Constraints
- Scroll-driven only, reversible, no timers. Motion off / reduced motion / no JS show the
  finished state (CSS defaults = progress 1).
- Edit the generators, never the generated blocks: `scripts/build-studio-explanations.mjs`
  (process, capture, review, containment, timing, export), `build-studio-support.mjs`
  (context, broker, recovery), `build-studio-internals.mjs` (gates, readiness), then
  `node scripts/build-studio-delivery.mjs`.
- Keep the check contracts: `[data-explanation]`, `.ex-path` (no dasharray unless
  `.internals-motion`), `.ex-step-b/c` dashoffset advances, `[data-ex-node]`/
  `[data-support-node]` text inside its rect, `.support-c` never transforms, support
  figure `style` changes with scroll, gate/readiness geometry used by the alignment checks.
- Mobile compact cards (`studio-mobile-diagrams.mjs`) stay as they are.

## Approach
1. Shared scene engine in `how-it-works.js`: per figure `--p` (0..1); tokens
   (`[data-along]`) placed on a connector path with `getPointAtLength`.
2. Shared CSS helpers in `how-it-works.css`: each animated part gets a window
   (`--a`, `--b`) and derives local progress `--k`; fade, draw, pop and caption-step rules.
3. Redesign per figure: process (request travels agent → broker → worker → frame),
   capture (bytes captured twice, compared cell by cell), review (two routes through one
   guard), containment (child escapes the boundary, refused), context (selection
   brackets close, identity resolves, optional anchor), broker (tool calls cycle, frame
   returns), recovery (task interrupted, baseline restored, draft set aside), timing
   (clips split/reorder/trim, timeline.json changes, build hash unchanged), export
   (frozen inputs → frames tick out → file). Gates stay; readiness gains a preview that
   swaps at the handoff.

## Acceptance
- Screenshots at 1440 and 390 at progress 0 / 0.5 / 1 and with JS off look complete.
- Diagram/details/motion/internals checks: no failures beyond the 39 pre-existing ones.
- No console errors; scroll performance check stays clean.

## Result
- Nine figures rebuilt as scroll scenes (six explanations, three support figures).
  Readiness keeps its animation and gains a preview chip that swaps "Hello." for
  "Make your move." at the handoff. Gates keeps its existing candidate animation; only
  its wording changed ("keeps draft").
- Checks on a clean headless profile, compared with HEAD served side by side:
  diagrams 37 failures (baseline 39, none new; node-label fit now passes), internals 10 and
  landing 6 (identical to HEAD: 15 px scrollbar overflow, agent icons), details scoped to
  `.support-figure` 12 (identical to HEAD). motion and performance pass.
- Late changes: "SCROLL TO FOLLOW" / "SCROLL TO ADVANCE THE FRAME" cues hide when nothing
  plays (motion off, no JS, compact phone cards; the renderer keeps its cue on phones);
  broker narration enlarged for its narrow column; timing scenes node back at x=25
  (w 215); narration aligned with the descriptions (broker scope, interruption, tracked
  child processes); unused support marker/packet rules removed.
- Start-state screenshots were taken at progress 0.1, not 0.
- Check builds: diagrams and motion ran on the final build; scoped details before the cue
  labels; internals, landing, performance before the timing and narration fixes. The later
  edits only change label display, narration copy and one node width.
- Known check defect: `check-studio-details` queries `[data-scroll-figure] .support-c`,
  which the renderer figure does not have, so the unscoped script throws.
