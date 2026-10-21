# fframes design system: Editorial Engineering

Status: proposed system, grounded in the implemented Studio web pages. The name
is a working description, not an existing industry standard or an approved brand
name. This specification does not change the deployed pages.

The central idea is **make the mechanism legible**. Present a precise explanation
of what changes, who changes it, and what must happen before a result is accepted.
Typography establishes hierarchy; diagrams establish relationships; motion
explains a transition. The interface should feel like a well-edited technical
publication with working instruments inside it.

The companion [token sheet](../../design-system/tokens.css) is an implementation
starting point. It is intentionally not loaded by the existing pages yet.

## 1. What the current design is

The visual language combines three traditions. These are interpretations of the
implemented work, not claims about its historical provenance.

| Influence                         | Expression in Studio                                                            | Purpose                              |
| --------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------ |
| Editorial modernism               | Asymmetric columns, generous margins, large restrained headlines, thin rules    | Create a clear reading order         |
| Technical illustration            | Exploded axonometric planes, leader lines, numbered stages, restrained diagrams | Explain hidden structure             |
| A developer's working environment | Source excerpts, terminal surfaces, monospace labels, explicit checks           | Connect the promise to a real action |

The supplied OLED/LCD reference contributed its **explanatory method**: separate
layers, show the path through them, annotate their functions. The page translates
that method into selection, a draft, validation and an accepted revision. These
are conceptual stages, not literal physical layers inside Studio.

The result is not defined by green, beige, or isometric artwork alone. Its
recognizable qualities are a quiet canvas, a strong reading hierarchy, visible
system boundaries and motion that teaches.

### The product story

The homepage follows one concrete change: a title moves from “Hello.” to “Make
your move.” Selection supplies context. The agent changes a draft. Build and
inspection precede acceptance. The accepted revision updates the preview.

The technical page expands the same mechanism: separate processes, capture and
checks, optional source anchors, read-only broker tools, preview readiness,
containment and recovery. This consistency matters more than adding more visual
motifs. A user should recognize the same objects and boundaries across both pages.

### The communication pattern

**Promise → mechanism → evidence and limits → next action.**

This is an explanation-led product page. A large headline makes the proposition
easy to identify, a diagram makes it understandable, technical details qualify
it, and a highlighted command or download makes it actionable.

## 2. What is implemented versus proposed

| Area       | Current implementation                         | Generalized direction                                                  |
| ---------- | ---------------------------------------------- | ---------------------------------------------------------------------- |
| Color      | Six root variables; many local colors          | Primitives → semantic roles → component tokens                         |
| Type       | Geist + Geist Mono; several local sizes        | Named type roles with readable minimums                                |
| Spacing    | Carefully tuned per section                    | A small spacing scale, with documented optical exceptions              |
| Components | Shared CSS classes and HTML patterns           | Documented contracts, then extraction where reuse is proven            |
| Diagrams   | Three SVG generators and local styling         | A shared visual grammar and consistent legends                         |
| Motion     | Shared preference controller, page controllers | A reusable preference contract plus figure-specific progress renderers |
| Validation | Chrome browser harnesses and screenshots       | Repeatable component/state coverage and human comprehension checks     |

An audit of `home.css` and `how-it-works.css` found **121 distinct hex colors**.
That count includes legitimate illustration tints; it is not a count of defects.
It shows why simply exporting the current CSS would preserve too many local
decisions. Several diagram labels are 8–10 px, compact links are 34–37 px tall,
and the copy control is 36 px square. Those choices should be revisited during
generalization, especially for essential information and touch use.

## 3. Principles and decision rules

1. **Explain before decorating.** Every illustration needs a question it answers.
   Every animated property needs a change it communicates.
2. **Keep the user's object recognizable.** The title, candidate or accepted
   revision must retain its identity across a sequence.
3. **Show ownership and boundaries.** Distinguish the user's project, agent tools,
   Studio checks and the preview worker. Avoid implying capabilities that have
   not been observed or verified.
4. **Use restraint to create emphasis.** Reserve the strongest contrast for the
   central result and the next action. Supporting material should remain readable.
5. **Preserve meaning in every mode.** Mobile, reduced motion, no JavaScript,
   keyboard navigation and an intermediate scroll position all need a coherent
   explanation.
6. **Optimize for understanding, then delight.** More motion is useful when it
   improves the viewer's account of what happened.

## 4. Foundations

### Color: roles before swatches

| Semantic role   | Current source value | Generalized meaning                              |
| --------------- | -------------------- | ------------------------------------------------ |
| Canvas          | `#fafbf8`            | Quiet background for reading and diagrams        |
| Primary text    | `#202925`            | Main reading hierarchy                           |
| Secondary text  | `#626b65`            | Supporting prose, still readable                 |
| Separator       | `#dfe3df`            | Decorative section divisions                     |
| Primary action  | `#245e45`            | A deliberate next step                           |
| Inverse surface | `#19372b`            | Featured command or important technical artifact |
| Selection       | pink family          | The user's target or supplied context            |
| Candidate       | blue family          | Work under consideration                         |
| Accepted        | green family         | An accepted state or successful stage            |
| Pending         | muted amber family   | Preparation or review, when explicitly labeled   |
| Focus           | `#2764db`            | Keyboard location, independent of product state  |

Selection pink is not an error color. Candidate blue is not a universal brand
color. An accepted green check must say what passed; it cannot imply that every
frame is correct. Do not communicate state using color alone: add labels, shape,
position or an icon.

The blue fframes wordmark remains its own brand asset. The Studio page's green
action color is not evidence that every fframes surface should be rebranded green.
Keep brand identity and functional roles separate so themes can evolve safely.

Calculated contrast pairs in the current source are 14.39:1 for primary text on
canvas, 5.31:1 for secondary text on canvas, 7.32:1 for green text on canvas and
7.52:1 for the terminal's secondary text on its dark surface. These are selected
opaque color pairs, not a complete accessibility audit: opacity, font size,
background layering, states and SVG scaling still need inspection.

### Typography: editorial voice and technical evidence

Use the sans-serif family for claims, explanations and actions. Use monospace
for commands, object identifiers, stage numbers and compact metadata. Monospace
should signal a kind of information, rather than decorate every paragraph.

| Role              | Current examples                   | Proposed reusable rule                              |
| ----------------- | ---------------------------------- | --------------------------------------------------- |
| Display           | 74 px hero, 82 px install headline | Fluid 48–82 px, weight 450, tight leading           |
| Section heading   | 32–44 px                           | Fluid 32–44 px, weight 450                          |
| Component heading | 16–24 px                           | 20–24 px, weight 450–500                            |
| Body              | 13–15 px                           | Default 16 px / 1.65; 14 px for supporting copy     |
| Control label     | 10–14 px                           | Default 14 px / 1.4                                 |
| Technical label   | 8–13 px                            | Target 12 px in rendered screen space               |
| Featured command  | 14 px desktop, 12 px mobile        | Target 14 px / 1.8, with safe wrapping or scrolling |

The proposed sizes are not yet applied. Increasing labels requires checking
line breaks, diagram geometry and sticky height; a token rename alone cannot
solve those constraints. Tiny diagram identifiers may remain supplemental only
when the same information is readable in surrounding text.

Use relative tracking for reusable type styles. The current fixed `-4px` hero
tracking is a page-specific optical adjustment, not a default for every heading.
Load fonts locally with system fallbacks. The current optional font display
deliberately keeps the first painted layout stable on slow connections.

### Space and geometry

Proposed spacing steps: **4, 8, 12, 16, 24, 32, 48, 64, 96, 128 px**. Use small
steps within a control, medium steps within a section, and larger steps between
different ideas. Permit optical exceptions for diagrams, and document them.

Use 1 px rules, mostly 4 px corner radii and little or no elevation. A panel
should look like a defined surface rather than a floating app window. Thin
separators are decorative; an interactive boundary may need stronger contrast.

The current content width is 1240 px; demonstration areas can reach 1450 px.
Mobile gutters are 20 px per side. Current responsive thresholds are 760 and
1100 px, with a 1500 px refinement. CSS variables cannot directly parameterize
standard media-query conditions; these remain documented literal breakpoints
until a build system with breakpoint generation is justified.

### Composition

Use four reusable arrangements:

- **Reading + seeing:** a narrow explanation column beside a dominant figure.
  Current wide-screen split: 38% / 62%; tablet: 42% / 58%.
- **Claim + qualification:** a headline on one side, an explanation or limit on
  the other. Useful for technical sections without scroll pinning.
- **Full-width explanation:** a large diagram between its introduction and
  caption. Use when the relationship needs horizontal space.
- **Action + artifact:** a clear next action paired with the command, file or
  example that makes it concrete. The install section is this pattern.

These are content relationships, not mandatory ratios. On mobile, preserve the
reading order and keep the next control reachable. The current sticky figures
use 40svh with a 370 px cap; that is a tested page recipe, not a universal figure
height. Inspect short screens and landscape orientation before reusing it.

## 5. Component contracts

These are proposed names for patterns already present, not an existing component
package. Prefer native semantic elements before choosing a framework wrapper.

| Component          | Required content and behavior                                                                        |
| ------------------ | ---------------------------------------------------------------------------------------------------- |
| `PageHeader`       | Brand link, concise navigation, primary destination, skip link                                       |
| `SectionIntro`     | Eyebrow, meaningful heading, optional supporting explanation                                         |
| `ActionLink`       | Clear verb and destination; native anchor for navigation/download                                    |
| `TextLink`         | Underline or other persistent affordance; visible keyboard focus                                     |
| `CommandPanel`     | Exact selectable command, environment label, optional copy control, visible success/failure status   |
| `DownloadAction`   | Platform and architecture, direct file destination, release-notes alternative                        |
| `AnnotatedFigure`  | SVG title/description, visible illustration label, caption and readable labels                       |
| `WorkflowStory`    | Stable object identities, ordered steps, one figure, current-step readout, working anchor navigation |
| `MotionPreference` | OS default, explicit enable/pause, pressed state, explanatory status, optional persistence           |
| `TechnicalList`    | Named facts or capabilities with their boundary stated nearby                                        |

Component states should be specified explicitly: default, hover, keyboard focus,
active/current, success and failure where relevant. Do not invent a disabled or
loading state for a component that has no real asynchronous operation.

For new components, use a 44 px preferred touch target and allow at least 16 px
space around crowded controls where practical. This is a system design target,
not a claim that the existing compact navigation already meets it. Avoid turning
every rounded card into an interactive control.

The command is the source of truth for copying. Copy must preserve its executable
text while removing decorative prompt characters. Show denial feedback and leave
the command selectable. Without JavaScript, hide the inactive copy control.

## 6. Diagram grammar

Choose the diagram by the relationship to explain:

| Question                               | Diagram pattern                                       |
| -------------------------------------- | ----------------------------------------------------- |
| What are the separate layers or gates? | Exploded axonometric stack                            |
| Who owns which responsibility?         | Flat process/boundary map                             |
| What context moves between objects?    | Annotated flow with stable nodes                      |
| When can a revision replace another?   | Parallel lanes and a handoff marker                   |
| What survives interruption?            | Stationary accepted state and separate retained draft |

Axonometric stacks are the signature illustration, but using them for every
relationship would make the system less clear. Flat diagrams are equally valid.

The current stack transform is `matrix(.866 .5 -.866 .5 tx ty)`, an isometric-style
axonometric projection. Keep a consistent projection within a figure. Use thin
edges, pale faces, subtle side faces and generous separation. Place labels outside
the planes with short leader lines. Use actual vector groups so parts can move
independently; a raster illustration cannot provide that same semantic control.

Reusable visual vocabulary:

- Plane: conceptual layer or stage, labeled as such.
- Solid node: a named object, process or artifact.
- Outline boundary: ownership, containment or an inspection boundary.
- Packet or arrow: direction of context, a candidate or a request.
- Dashed guide: alignment or an illustrative path, not guaranteed execution.
- Check: a named completed check; never an unqualified product guarantee.
- Parallel lane: states that coexist during preparation.

Do not reuse the same glyph for different meanings in one sequence. Keep object
identity and legends stable across pages. Distinguish illustrative diagrams from
actual application screenshots. If timing is illustrative, state that diagram
distance does not represent elapsed build time.

## 7. Motion grammar

The relevant interaction pattern is **scrollytelling**: reading progress directly
manipulates an explanatory figure. Its defining property here is reversibility.
It is not an autoplay presentation triggered by entering the viewport.

The generalized contract is:

```text
document position → normalized progress → named phases → element properties
```

For a phase between `a` and `b`, the existing controllers use:

```js
const t = Math.max(0, Math.min(1, (progress - a) / (b - a)));
const eased = t * t * (3 - 2 * t); // smoothstep
```

Keep phase endpoints ordered and distinct. Explain which product boundary each
phase represents. The scroll mapping is illustrative reading progress, not an
estimate of the real operation's duration.

Motion recipes:

1. **Separate to explain:** unfold related planes or cards while preserving their
   identities and alignment.
2. **Transfer context:** move a small packet along an existing connection.
3. **Expose a gate:** reveal a named check as the candidate reaches it.
4. **Prepare before handoff:** advance preparation lanes while the current state
   remains available, then show the replacement.
5. **Preserve the baseline:** keep accepted work stationary while unfinished
   work becomes a retained draft.

An intermediate frame must be meaningful. Stopping must hold the same properties.
Returning to a scroll position must reproduce them exactly. Do not add CSS
transitions to scroll-derived transforms: their delayed settling would break that
contract. Time-based hover feedback can use the separate short feedback token.

Measure geometry on setup, resize and font readiness, outside the scroll draw
path. Coalesce scroll events into one requestAnimationFrame callback and cache
settled progress. Prefer transforms and opacity; restrained SVG stroke drawing
is allowed but requires paint profiling. “Transform” is not a guarantee that an
SVG animation is free or runs entirely on the GPU.

Honor the OS motion preference by default. Allow a visible, explicit override;
persist it when storage is available, without depending on storage for basic use.
With reduced motion or no JavaScript, render a complete static explanation.

## 8. Content and evidence

Write with concrete actors and verbs: “The agent edits a draft”; “Studio builds
the candidate”; “The accepted revision updates the preview.” Keep the causal
chain visible. Pair limitations with the capability they qualify.

Separate these evidence types:

1. Directly observed application behavior or original recordings.
2. Source-supported behavior and controlled test fixtures.
3. Illustrative scenarios and proposed behavior.

The current work has source and controlled-test evidence, but no original native
interaction recording or signed-in agent replay from this environment. A polished
illustration must not be presented as that missing evidence.

## 9. How to generalize without freezing accidents

Use three layers:

```text
Foundations: color, typography, space, geometry, focus, motion preference
    ↓
Components: actions, commands, navigation, figures, lists, status
    ↓
Patterns: workflow story, technical explanation, install/download
```

Keep the foundations portable. Add web behavior as a platform implementation.
If the system later includes the GPUI desktop app, map semantic roles to native
tokens and focus behavior separately. A web landing-page layout is not a desktop
editor layout, and the current work has not standardized the native application.

Recommended adoption sequence:

1. Review the vocabulary, semantic colors and readable type targets.
2. Pilot the token sheet in a small isolated surface; test both font-loaded and
   fallback layouts. Do not import it globally just to declare adoption.
3. Replace existing literal values with semantic aliases component by component.
   Consolidate near-identical tints only after comparing screenshots.
4. Extract the proven component contracts. Keep page-specific narrative controllers
   separate from the shared motion-preference service.
5. Build a reference surface with default, focus, error, reduced-motion and static
   examples. Include captions describing what each diagram means.
6. Pilot a second kind of page before promoting the draft to a stable system.

New tokens need a semantic use, a second-use case or a documented exception.
Product-specific coordinates belong in figure geometry, not the global spacing
scale. Add a theme only when a real use case needs it. Avoid introducing a UI
framework, utility compiler or animation dependency solely to package these rules.

## 10. Acceptance and research

For every adopted component or pattern, check desktop and narrow/short mobile
layouts, text wrapping, keyboard reachability and focus, working links and copy
feedback, static rendering, reduced motion, exact scroll reversal and performance
while its figure is active. Check labels at their actual rendered SVG scale.

The current 143 Chrome checks are page-level evidence, not certification of the
proposed system. Safari/Firefox, real devices, screen readers, repeated field
performance measurements and unfamiliar-viewer sessions remain necessary.

A short comprehension session:

1. Show the page for 10 seconds. Ask what the product is and what it helps them do.
2. Let them scroll slowly, stop and reverse. Ask what the moving object represents.
3. Ask what changes first, what is checked, and when the preview can update.
4. Ask whether the figure is a working app, a recording or an explanation.
5. Ask them to find the Windows installer and explain the alternative command.
6. On the technical page, ask what remains after an interruption.

Record their words and errors without coaching. Confusion between a draft and an
accepted revision, or a belief that the diagram guarantees all frames correct,
is a design problem even when the animation looks polished.

## Source map

- `landing/studio/home.css`: foundations, shared elements and homepage composition.
- `landing/studio/how-it-works.css`: technical figures, supporting diagrams and layout.
- `landing/studio/scroll-motion.js`: shared preference and persistence contract.
- `landing/studio/home.js`: selection → edit → checks → accepted preview.
- `landing/studio/how-it-works.js`: gate, readiness and supporting-diagram progress.
- `scripts/build-studio-workflow.mjs`: exploded edit illustration.
- `scripts/build-studio-internals.mjs`: transaction and readiness illustrations.
- `scripts/build-studio-support.mjs`: context, broker and recovery figures.
- `scripts/check-studio-{landing,internals,motion,details,performance}.mjs`: browser evidence.
