# Research: a component model for fframes, like Remotion with Manim's vocabulary

Date: 2026-10-23. Scope: how to build an fframes component library with many components that are
easy to import, configure and edit, the way Remotion (React components for video) and Manim
(mobjects and animations) are. This report settles the model; it builds on the overview map
(`components-261023-0753-overview-map.md`), the strategy (`components-261023-0804-library-strategy.md`)
and the Manim research (`research-261023-1404-manim-api-for-fframes.md`), and replaces their
"how a video uses a component" sections.

Facts about Remotion, Leptos, Dioxus, Yew and the `svgr!` macro below were checked against the
sources listed at the end on 2026-10-23. Items marked *unconfirmed* were not found in an official
page and must be verified before relying on them.

## Summary

- **Use Remotion's model, not Manim's.** Remotion renders a video as a tree of components whose
  output is a pure function of the current frame, with `<Sequence from=..>` shifting time for its
  children. That is exactly how `Video::render_frame` and `Scene` already work in fframes, so the
  component model fits without a new runtime. Manim's imperative `play()` script does not fit; its
  value is the vocabulary (`Axes`, `MathTex`, `Create`, `Write`, `Transform`) and the catalog of
  figures, which become components and animation props.
- **A component is a Rust struct of props plus one `render` method.** Importing is `use
  fframes_kit::prelude::*;`. Changing a parameter is setting a field. Defaults come from
  the theme. Composition is a `children` closure that receives the (time-shifted) `Ui`. This
  works with today's `svgr!` through `{ui.show(..)}`.
- **No upstream code changes.** The library, its tag macro, its gallery and its docs live in
  one fork-owned directory with its own Cargo workspace, like `desktop/`, and depend on
  fframes by path. Every API it uses exists in upstream fframes today; the fork-only Studio
  APIs sit behind a `studio` feature. Pulling upstream stays conflict-free (section 10).
- **Component tags** (`<LowerThird name="Ada" role="Mathematician" />`) are feasible without
  touching `svgr-macro`: the library's own `ui!` macro rewrites component tags into the struct
  form and hands the rest to `svgr!`. It is one milestone of its own, after the struct form
  works.
- **Editing in Studio** follows Remotion's zod schema idea: props are plain data, a `Props` derive
  emits a schema, and Studio shows a panel for the selected component through the existing
  `EditorObjectKey`. This is a later milestone, but the "props are plain data" rule applies from
  day one so it stays possible.
- **"Many components"** comes from three catalogs that already exist in the earlier reports: about
  60 general candidates from the examples, 11 shapes and 20 path utilities modelled on Remotion's
  packages, and about 75 figures across math, physics, CS, biology and chemistry plus the Manim
  mobject and animation tables. This report does not repeat the lists; section 6 says which
  package each one lands in.

## 1. What "like Remotion" means, and the fframes equivalent

| Remotion | What it does (verified) | fframes today | Component library |
| --- | --- | --- | --- |
| `<Composition id component defaultProps schema calculateMetadata durationInFrames fps width height>` | Registers a video; `defaultProps` must be "pure JSON-serializable values"; `calculateMetadata` is "called a single time" and its returned values "take precedence over the props passed directly" | `impl Video` with `FPS`/`WIDTH`/`HEIGHT` consts and `duration()` | unchanged; a video is still `impl Video` |
| `useCurrentFrame()` | the frame, 0-indexed; inside a `<Sequence>` it is shifted by `from` | `frame.index`, `frame.seconds()`; scenes get an offset frame | `ui.frame` |
| `<Sequence from durationInFrames layout name premountFor>` | children "assume the video starts" at `from`; nested sequences cascade | `Scene` plus `Frame::clone_with_scene_offset` (public) | `Sequence` component, section 3.4 |
| `<Series>` / `<Series.Sequence durationInFrames offset>` | plays children back to back; negative `offset` overlaps | `Scenes` with `Overlap` | `Series` component (thin wrapper over `Sequence`) |
| `<Loop durationInFrames times>` | repeats its children | `frame.animate_loop` | `Loop` component |
| `<Freeze frame active>` | children see a fixed frame | none | `Freeze` component |
| `interpolate(frame, [in], [out], {extrapolateLeft/Right: extend\|clamp\|wrap\|identity, easing})` | maps a frame range to a value range | `timeline!` + `frame.animate` (clamps outside the keyframes) | `Anim<T>` prop, section 3.5 |
| `spring({frame, fps, config: {mass 1, damping 10, stiffness 100, overshootClamping}, delay, durationInFrames, from, to})` | physical spring | `Easing::Spring { mass, stiffness, damping }` | same `Easing` |
| `<TransitionSeries>` with `.Sequence`, `.Transition(timing, presentation)`; presentations `fade()`, `slide()` (default), `wipe()`; timings `linearTiming`, `springTiming`; "Transition scenes must stay absolutely positioned" | cross-scene transitions | `Overlap` cross-fade only | `transitions` module: `Fade`, `Slide`, `Wipe`, plus `Timing` |
| `@remotion/shapes`: `Arrow, Callout, Circle, Ellipse, Heart, Pie, Polygon, Rect, Spark, Star, Triangle` and a `makeX` function for each | components and path builders | raw `svgr!` elements | `shapes` module, same 11 names plus Manim's |
| `@remotion/paths`: `getLength, getPointAtLength, getTangentAtLength, evolvePath, interpolatePath, translatePath, scalePath, reversePath, cutPath, warpPath, normalizePath, getBoundingBox, getSubpaths, extendViewBox, centerPath, parsePath, serializeInstructions, resetPath, reduceInstructions` | path utilities | `kurbo` (optional dep via `lottie`) | `paths` module over `kurbo::BezPath` |
| `@remotion/layout-utils`: `measureText, fitText, fitTextOnNLines, fillTextBox` | text measurement | `frame.text_width/text_fit/text_break_lines` | re-exported through `Ui` |
| `@remotion/motion-blur`: `Trail`, `CameraMotionBlur(shutterAngle, samples)` | "repeats earlier positions behind the current one" | none (3D has `render_motion_blur`) | `Trail` component |
| `@remotion/effects` light leak (`lightLeak({seed, hueShift, progress})`) | an overlay effect | examples have blur/glow one-offs | `effects` module |
| `@remotion/zod-types`: `zColor, zMatrix, zTextarea`; `schema` on a Composition "takes precedence over the inferred controls" | Studio props panel | `EditorObjectKey.style_tokens` | `Props` derive emitting a schema, section 5 |
| Remotion Agent Skills (`npx skills add remotion-dev/skills`, 12 skills) and `llms.txt` | guidance for coding agents; e.g. "Keep `useCurrentFrame()` in the smallest component that needs it" | `skills/fframes-video` with 4 references | `docs/components/components.md` compiled as doctests, section 7 |

Two Remotion ideas transfer unchanged and matter most:

1. **Everything is a pure function of the frame.** No component keeps state between frames. This is
   also fframes' rule for `render_frame` (multi-threaded, no I/O), so the model costs nothing.
2. **Time is local.** A component animates from its own zero; the parent decides when it starts.
   Remotion does this with `<Sequence from>`; fframes does it for scenes with offset frames. The
   component library does it for any subtree.

What does not transfer: React's reconciliation, hooks, memoization and the DOM. fframes has no
retained tree; the renderer caches static `svgr!` subtrees by `static_hash` instead. Section 3.6
says how expensive component work is cached without React.

## 2. What "like Manim" means, and what stays from the Manim research

Manim contributes three things, all of which survive the switch to components:

- **Names.** `Circle`, `Square`, `Arrow`, `Axes`, `NumberPlane`, `MathTex`, `Text`, `VGroup`,
  `Create`, `Write`, `FadeIn`, `GrowFromCenter`, `DrawBorderThenFill`, `Transform`,
  `Indicate`, `Circumscribe`, `Flash`, `Wiggle`, and the rate functions. Agents already know
  these from thousands of Manim examples, so the library uses them verbatim (section 7).
- **The catalog.** The animation, mobject and scene tables in the Manim research (section 5
  there) and the subject figures in the strategy (section 7 there) are the backlog.
- **Units and defaults.** The Manim research settled an 8-unit-tall, origin-centred, y-up space
  for the explain layer. It becomes a `Canvas` component that maps its children from Manim units
  to pixels, so general components stay in pixels like the rest of fframes and Manim-style
  components can be written in Manim's coordinates.

What changes: the recorded `Script` (`s.play(..)`, `s.wait(..)`) is no longer the main API. An
animation is a prop on a component (`enter={Create}`), and sequencing is done by `Sequence` and
`Series`. The script can stay as an optional helper for porting Manim examples line by line
(it would expand to the same components), but it is not needed for the first milestones.

## 3. The component contract

Nothing in this section exists yet. The sketches show the public API the library would commit to.

### 3.1 Import, use, change a parameter

```rust
use fframes_kit::prelude::*;

impl Video for MyVideo<'_> {
    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::new(frame, ctx, &self.theme, "main");
        fframes::svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080" width={Self::WIDTH} height={Self::HEIGHT}>
            {ui.show(Sequence { from: 1.0, duration: Some(4.0), children: Box::new(|ui| ui.show(LowerThird {
                name: "Ada Lovelace", role: "Mathematician", ..ui.defaults()
            })) })}
            {ui.show(Circle { cx: 960.0, cy: 540.0, radius: 120.0.into(), enter: Some(Create::default()), ..ui.defaults() })}
        </svg>)
    }
}
```

- `use ..::prelude::*` is the only import. Everything a video needs is in it.
- A component is a struct literal. You set the props you care about; `..ui.defaults()` fills the
  rest from the theme (`Themed::themed(&theme)`, one mechanism for every component; a component
  with no themed props still implements it).
- `ui.show(component)` renders it at the current time and returns an `Svgr`, so the call sits
  anywhere `{expr}` is allowed in `svgr!` today.
- A component has no `start` prop. Its parent decides when it starts by wrapping it in a
  `Sequence`, exactly as in Remotion. Inside, the component's time starts at zero.

### 3.2 The trait and the context

```rust
pub trait Component<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a>;    // consumes the props: owned Strings move into the Svgr
}

pub struct Ui<'a, 'c, 'm> {
    pub frame: Frame,                        // local time: shifted by every enclosing Sequence
    pub duration: Option<usize>,             // frames left in the enclosing Sequence, scene or video
    pub ctx: &'c FFramesContext<'a, 'm>,     // media, fonts, scenes (same shape as render_frame's ctx)
    pub theme: &'a Theme,                    // the Studio preset tokens, section 3.7
    pub scene_instance_key: &'a str,         // for EditorObjectKey, section 5
}

impl<'a> Ui<'a, '_, '_> {
    pub fn show(&mut self, c: impl Component<'a>) -> Svgr<'a> { c.render(self) }
    pub fn defaults<T: Themed>(&self) -> T { T::themed(self.theme) }
    pub fn seconds(&self) -> f32 { self.frame.seconds() }
    pub fn text_width(&mut self, font: FontQuery, text: &'a str) -> Option<usize> { self.frame.text_width(self.ctx, font, text) }
}
```

- `render` takes `self` by value because `Svgr<'a>` can own a `String` only by moving it
  (`From<String> for Svgr`, `fframes/src/svgr.rs:151`); with `&self` an owned text prop would
  have to be cloned every frame. Components are built per frame anyway, so nothing is lost. For
  a `Vec<Box<dyn Component>>` the method is called through `self: Box<Self>`, which keeps the
  trait object-safe.
- No `Sync` bound. `Ui` owns a `Frame`, and the frame's text cache is `Rc<RefCell<..>>`
  (`fframes/src/text.rs:200`), so a `Ui`, the components and the children closures live on one
  render thread inside one `render_frame` call. Only the `Video` has to be `Sync`, as today.
- `Frame` is passed by value and the text helpers need `&mut Frame`, so `Ui` owns a `Frame` and
  hands out `&mut self`. A child `Ui` for a `Sequence` is built with
  `Frame::clone_with_scene_offset(&ui.frame, offset_frames)` (`fframes/src/frame.rs:76`), which
  is public and keeps the text cache. It computes `index - offset`, so every time container
  must check `index >= offset` first (3.4 does).
- Lifetimes follow `Svgr<'a>`: `'a` is the video's lifetime. Text props are `&'a str` for
  borrowed text or `String` for text built per frame; both convert into `Svgr` (the `String`
  case is `{timer(t)}` in `examples/svg-in-3d/src/lib.rs:206`).
- **Text measurement without a core change.** `Frame::text_width`, `text_fit` and
  `text_break_lines` take `value: &'a str` with the context's lifetime
  (`fframes/src/frame.rs:354`), which is why `examples/fframes-intro/src/ui.rs` leaks per-frame
  strings. The library does not leak and does not change them. Each per-frame case has a route:
  - `Typewriter`: a prefix of a borrowed `&'a str` is itself `&'a str`, so
    `text_width(ctx, font, &self.text[..n])` compiles today.
  - `Counter`: the digits and separators are `'static` literals, measured once and summed per
    frame (tabular digits, which counters want anyway).
  - `Caption`: `Frame::get_subtitle_phrase` returns `Option<&'a str>` borrowed from the
    subtitles (`frame.rs:415`), so it is measurable as is.
  - Components that draw glyphs themselves (`Write` on text, formulas) measure from their own
    `ttf_parser::Face`, built in the constructor from `MediaProvider::get_all_font_data`. The
    width then matches what is drawn, because the library draws those paths.

### 3.3 Props rules (what Leptos, Dioxus and Yew agree on)

All three Rust UI frameworks reduce a component to a props struct plus a function, and all three
mark optional props and defaults on the struct: Leptos `#[prop(optional)]`, `#[prop(into)]`,
`#[prop(default = ..)]`; Dioxus `#[props(default = 42)]`, `#[props(into)]`; Yew
`#[prop_or_default]`, `#[prop_or(expr)]`, `#[prop_or_else(fn)]`. Leptos builds the props with a
typed builder so a missing required prop is a compile error; `typed-builder` and `bon` both
promise that ("Trying to build without setting one of the fields will also generate a
compile-time error"; "Any mistakes such as missing required fields ... will be reported as
compile-time errors").

Rules for fframes:

1. **Props are plain data**: numbers, `&str`/`String`, `Color`, `Transform`, enums, `Option<T>`,
   `Anim<T>` (3.5), and `children` closures. No trait objects, no handles. This keeps them
   serializable for the Studio panel (section 5), like Remotion's "JSON-serializable"
   `defaultProps`.
2. **Every prop has a default** except text content and data (`name`, `points`, `rows`). A
   component with missing required data renders nothing and reports
   `Diagnostic::RendererWarning` through `fframes::diagnostics::report`, a public upstream API
   (`fframes/src/diagnostics.rs:136`), so `inspect` lists it with no core change.
3. **Struct update syntax, no builder, in the first milestones.** `LowerThird { name, ..defaults }`
   is plain Rust, zero dependencies, and what the tag form expands to. If compile-time
   "required prop missing" errors turn out to matter for agents, `bon` can be added to the
   `Props` derive later without changing call sites in the tag form.
4. **Defaults come from the theme**, through `Themed::themed(&theme)` and `ui.defaults()`, not
   from constants or a second `Default` impl, so every component follows the Studio preset
   (section 3.7) and there is one way to fill a struct.
5. **Children are a closure** `Children<'a> = Box<dyn Fn(&mut Ui<'a, '_, '_>) -> Svgr<'a> + 'a>`
   (Leptos' `ChildrenFn`), because a container must render them with its own `Ui` (shifted time,
   its remaining duration). The `+ 'a` lets the closure capture borrowed props. The tag form
   builds the closure.
6. **Identity**: a component that should be editable or addressable sets `key: Option<&str>`,
   which becomes `EditorObjectKey.object_key`; `component_key` is the component's kebab name.
   No counters (they are unstable when items are inserted), as the strategy already said; a
   list item's `repeat_key` comes from the item's data (section 5).

### 3.4 Time: `Sequence`, `Series`, `Loop`, `Freeze`

```rust
pub struct Sequence<'a> {
    pub from: f32,                // seconds, in the parent's local time
    pub duration: Option<f32>,    // None: until the parent ends
    pub children: Children<'a>,
}

impl<'a> Component<'a> for Sequence<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let start = ui.frame.second_to_frame(self.from);          // frame.rs:118, truncating
        if ui.frame.index < start { return Svgr::empty(); }      // precondition of clone_with_scene_offset
        let length = self.duration.map(|d| ui.frame.second_to_frame(d)).or(ui.duration.map(|d| d.saturating_sub(start)));
        if let Some(len) = length && ui.frame.index >= start + len { return Svgr::empty(); }
        let mut child = Ui {
            frame: Frame::clone_with_scene_offset(&ui.frame, start),
            duration: length,
            ctx: ui.ctx, theme: ui.theme, scene_instance_key: ui.scene_instance_key,
        };
        (self.children)(&mut child)
    }
}
```

- Inside the children, `ui.seconds()` counts from `from`, exactly like Remotion's
  `useCurrentFrame()` inside `<Sequence>` and like fframes scenes. `ui.duration` is the Remotion
  `durationInFrames` of the enclosing sequence; `Ui::new` fills it from
  `ctx.duration_in_frames` (`fframes/src/fframes_context.rs:48`) or the scene's length, so an
  `exit` animation (3.5) always knows when its component ends.
- `Series { items: Vec<(f32, Children)>, overlap }` places children back to back and is sugar
  over `Sequence`. `Loop { duration, times, children }` wraps the local frame. `Freeze { at,
  children }` fixes it. All three keep the `index >= offset` check before building the child
  frame.
- Nothing in this section needs a core change. `Frame.index` is `usize`, so a sequence before
  its start returns empty instead of a negative frame; Remotion renders nothing before `from`
  as well.

### 3.5 Animated props and Manim-named animations

```rust
pub enum Anim<'a, T: Animatable> {
    Fixed(T),
    Keyframes(&'a KeyFramesAnimation<T>),   // a `timeline!` stored in the video, sampled with frame.animate
}
impl<T: Animatable> From<T> for Anim<'_, T> { .. }            // `radius: 120.0.into()` or via the tag form

pub trait Enter { fn apply<'a>(&self, ui: &Ui<'a, '_, '_>, content: Svgr<'a>, geometry: &Geometry) -> Svgr<'a>; }
pub struct Create { pub delay: f32, pub run_time: f32, pub rate_func: RateFunc }    // Default: 0, 1.0, Smooth
pub struct Write { .. } pub struct FadeIn { pub shift: (f32, f32), .. } pub struct GrowFromCenter { .. } pub struct DrawBorderThenFill { .. }
```

- Any numeric, colour or transform prop is `Anim<T>` so it can be a fixed value or a stored
  `timeline!`, and `ui.frame.animate` samples it. That is Remotion's `interpolate` and `spring`
  expressed as a prop type; `timeline!` already covers the tween, so no third variant is added
  until a gallery scene needs one.
- `enter` and `exit` props take Manim animation structs with Manim's fields (`run_time` default
  1.0, `rate_func` default smooth, `lag_ratio` for groups). Times are local to the enclosing
  `Sequence`. `Create` needs the component's geometry (path length), which is why `Enter::apply`
  gets a `Geometry` from the component, not just the `Svgr`.
- Group animations (`LaggedStart`, `AnimationGroup`, `Succession`) are containers that set the
  `delay` of each child, so they are components too.

### 3.6 Cost and caching without React

A component is constructed on every frame, so it must be cheap to build. The library adds no
cache of its own; it follows the existing fframes rule (AGENTS.md: precompute in the
constructor, memoise per-video data in a `OnceLock`):

- **Once per video**: `timeline!` animations, parsed data, 3D scenes, `Shader`s, glyph outlines
  for `Write`, and Typst formula layout (from `MediaProvider::get_all_font_data`, as the Manim
  research planned) are built in the video's constructor and borrowed by props (`&'a`), the way
  `Anim::Keyframes` borrows a `KeyFramesAnimation`.
- **Per frame**: only cheap arithmetic and small strings. If a measured need appears (C1
  `Write` outlines for text built per frame), the library adds a bounded `thread_local!` cache
  that compares full keys: per render worker and lock-free, like the `TextCache` that `Frame`
  carries (`fframes/src/text.rs:200`), with no core change. A shared locked cache in the video
  would be hit by every render thread on every frame and is ruled out.
- **Static subtrees**: a component's own `svgr!` keeps its decorative markup literal, so those
  parts still get a `static_hash` and the renderers cache them. A component call in the parent
  is an expression child, so the parent subtree around it is dynamic, exactly like any `{expr}`
  today. `just bench` is a timing recipe, not a gate, so C0 records a baseline of the
  hand-written example it ports and compares against it.

### 3.7 Theme as context

`Theme::from_styles(&fframes::Styles)` reads the Studio preset tokens (40 as counted in the
strategy report: colours, typography,
motion durations and easings, spacing, radius, stroke, shadow). `Ui.theme` is the Remotion
equivalent of a React context: every component reads its defaults from it and lists the tokens it
used in `EditorObjectKey.style_tokens`, so Studio can show which tokens a selected object follows.
A video without `fframes::Styles` uses `Theme::default()`. `fframes::Styles` exists only in
this fork, so `Theme::from_styles` is behind the `studio` feature (section 5).

### 3.8 User-defined components

A user component is just another struct implementing `Component`, built from library ones. With
the tag form (section 4) it reads like a React function component:

```rust
pub struct TitleCard<'a> { pub title: &'a str, pub width: f32 }

impl<'a> Component<'a> for TitleCard<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        fframes::svgr!(<g>
            <Text text={self.title} style={ui.theme.title} enter={Write::default()} />
            <Underline width={self.width} enter={Create { delay: 0.3, ..ui.defaults() }} />
        </g>)
    }
}
```

A `#[component]` attribute on a function (as in Leptos and Dioxus) can generate the struct and the
impl from a function signature later; it is sugar, not a prerequisite.

Because a library component is the same thing, one plain struct and one `render`, a video can
also copy a component's file into its own crate and edit it there (the shadcn way of "owning"
a component) when a prop is not enough.

## 4. Component tags

What the macro does today (`svgr-macro/src`):

- The parser accepts any tag name: a single identifier, a path, a dashed or a colon name, or
  `{expr}` (`parser.rs:459`, `node_name`). `<LowerThird ..>` therefore parses as a one-segment
  path with no error.
- Rejection happens in code generation: `parse_tag_name` (`nodes_to_svgtree.rs:949`) maps the
  name to a usvgr element id and fails with "element is not supported". The string path used on
  wasm32 (`nodes_to_format.rs:16`) writes any name verbatim.
- Attributes arrive as nodes with a name and an optional `Expr` (`attribute`,
  `parser.rs:414-455`; a `{expr}` value goes through `block_attribute_expr` at 394-412 and
  `process_block` at 153, which hoists `frame.animate(&timeline!(..))` into a `lazy_static`).
  A bare attribute has no value. Children are parsed as SVG nodes before code generation
  (`parser.rs:289-296`).
- An expression child is emitted as `Svgr::from(expr).as_subtree()` (`nodes_to_svgtree.rs:866`),
  and a subtree with an expression attribute or child is dynamic, with `static_hash: None`
  (`nodes_to_svgtree.rs:558, 578`).
- With the `compile-time-svgtree` feature, `create_svgr_ident` (`lib.rs:34-52`) runs both
  generators and emits a `cfg(target_arch)` branch for each, so the proc macro does not know the
  target; only a `default-features = false` build (the editor bridge) takes the string path
  alone.

**`svgr-macro` is not changed** (section 10). The library ships its own proc macro,
`fframes_kit::ui!`, that wraps `svgr!`. It rewrites component tags into the struct
form and passes everything else through, so the inner `svgr!` only ever sees ordinary SVG and
`{expr}` children:

1. It scans the token stream. A tag whose name is one identifier starting with an uppercase
   letter is a component tag. This is safe because every SVG element name starts lowercase
   (`linearGradient`, `clipPath`). Everything else, including SVG tags, attributes, text and
   `{expr}` blocks, is copied through unchanged.
2. A component tag's attributes become struct fields: `name="Ada"` and `radius={120.0}` become
   `name: "Ada".into()` and `radius: (120.0).into()`, so an `Anim<f32>` field accepts a number.
   Dashed names are rejected with a message naming the Rust field (`font-size` is `font_size`).
3. Its children, if any, become `children: Box::new(|ui| ::fframes_kit::ui!(<g>..</g>))`,
   so nested component tags are handled by the same macro, recursively.
4. The tag is replaced by `{ui.show(LowerThird { name: "Ada".into(), ..ui.defaults() })}` and
   the whole stream is emitted as `::fframes::svgr!(..)`. From there `svgr!` works as today:
   static subtrees keep their `static_hash`, and both code paths and the editor bridge are
   unaffected.
5. The macro has to know the name of the `Ui` value. Convention: it expects a variable named
   `ui` in scope, the way Remotion's hooks expect a React context, and the error message says
   so when it is missing. (Alternative: `ui!(ui; <Circle .. />)`, see the open questions.)
6. `timeline!` is not hoisted inside component props. `svgr!` turns
   `frame.animate(&timeline!(..))` into a `lazy_static` only for attribute blocks
   (`process_block` is called only from `block_attribute_expr`, `parser.rs:399`), and a
   component tag becomes a child block. This matches the design already: an animated prop
   borrows a `timeline!` stored in the video (`Anim::Keyframes`, 3.5).

Size: an estimate of a few hundred lines in the library's macro crate, plus tests in that
crate (expansion tests, and snapshots that render a scene in tag form and in struct form). The
gallery's editor bridge build exercises the wasm32 string path. This is milestone C2 in
section 8; the struct form is enough to build and use every component before it lands.

## 5. Editing in Studio

"Change the parameter" in code is covered by section 3. For editing without code, Remotion's
answer is a schema on the composition that the Studio turns into controls (`zColor`, `zTextarea`,
`zMatrix`); the schema "takes precedence over the inferred controls". fframes has the first half
already: `EditorObjectKey` (`fframes/src/editor.rs:19`) with four mandatory parts,
`scene_instance_key`, `component_key`, `object_key` and `repeat_key` (an empty one is rejected
by `EditorObjectKey::new`, editor.rs:79-107), plus `source_anchor` and `style_tokens`; it is
attached by `Svgr::with_editor_object`.

`EditorObjectKey`, `Svgr::with_editor_object`, `Scene::editor_instance_key` and
`fframes::Styles` are additions of this fork to `fframes/src` (`editor.rs`, `styles.rs`), not
upstream APIs. The library uses them only behind a `studio` cargo feature. Without it, `Ui::show`
attaches no key and the theme is `Theme::default()`, so the crate also compiles against vanilla
upstream fframes.

The component library adds the second half in a later milestone:

- `#[derive(Props)]` on a component struct emits `fn schema() -> PropsSchema` (field name, type,
  default, optional range and unit, theme token) and `fn to_values(&self)`.
- `Ui::show` attaches an `EditorObjectKey` when the component sets `key`: `scene_instance_key`
  comes from `Ui::new` (the scene's `editor_instance_key()`, or the video's name at top level),
  `component_key` is the kebab name, `object_key` is the component's `key`, and `repeat_key` is
  `"primary"` for a singleton (the convention in `fframes/tests/editor_identity.rs`) or a value
  from the item's data inside a list, never a counter.
- Studio looks up the schema by `component_key` and shows a panel. Writing a changed value back
  into the source (Remotion's `saveDefaultProps`, *unconfirmed* in current docs beyond the 4.0
  release notes) needs `source_anchor`, which `EditorObjectKey` already carries.

Design consequence now: props stay plain data (rule 1 in 3.3), so a schema can be derived later
without rewriting components.

## 6. Where "many components" come from

The counts below come from the three earlier reports and from Remotion's package indexes; the
full lists stay there.

| Package (feature) | Contents | Count | Source list |
| --- | --- | --- | --- |
| `core` (always) | `Ui`, `Component`, `Sequence`, `Series`, `Loop`, `Freeze`, `Fill` (Remotion's `AbsoluteFill`), `Anim`, `Enter`/`Exit` with the Manim names, `Theme`; `Canvas` (Manim units) joins at C4 | about 25 | this report; Manim research section 5 (animations) |
| `shapes` | Remotion's 11 (`Arrow, Callout, Circle, Ellipse, Heart, Pie, Polygon, Rect, Spark, Star, Triangle`) plus Manim's `Square, Line, DashedLine, Dot, Arc, Annulus, RoundedRectangle, Vector, CurvedArrow, Brace` | about 22 | Remotion shapes index; Manim research section 5 (mobjects) |
| `paths` | the 20 Remotion path functions over `kurbo::BezPath`, plus `align_points` for `Transform` | about 21 | Remotion paths index |
| `text` | `Text`, `Paragraph`, `Counter`, `Typewriter`, `Caption`, `Code`, `MarkupText` | 7 | overview map, text category |
| `general` | lower thirds, cards, progress, grids, mockups, audio bars, backgrounds, overlays | about 40 | overview map, 12 categories |
| `transitions` | `Fade`, `Slide`, `Wipe`, `Flip`, `ClockWipe`, `Iris` with `Timing::{Linear, Spring}` | 6 plus timings | Remotion transitions (the last three *unconfirmed* on the page read) |
| `effects` | `Trail`, `CameraMotionBlur`, `LightLeak`, `Glow`, `Blur`, `Noise` | 6 | Remotion motion-blur, effects; overview map decoration |
| `explain` | `Axes`, `NumberPlane`, `NumberLine`, `FunctionGraph`, `MathTex` (Typst), `Matrix`, `Table`, `Graph`, `VGroup` with `arrange`, `Brace`, `ValueTracker` as `Anim` | about 30 | Manim research section 5 |
| `math`, `physics`, `cs`, `biology`, `chemistry` | the subject figures | 12 + 12 + 13 + 11 + 11 | strategy section 7 |
| already in fframes | `lottie`, `three_d`, `Shader`, audio visualisation, subtitles | wrapped as components where it helps (`Lottie`, `Model3d`, `ShaderLayer`, `AudioBars`, `Subtitle`) | AGENTS.md |

Order of building follows value per effort: `core` and `shapes` first (everything else uses
them), then `text` and `general` (ports of existing example code, low risk), then `explain` and
`math` (the formula spike decides `MathTex`), then the other subjects, with `transitions` and
`effects` interleaved when a gallery scene needs them.

## 7. Easy for agents

Remotion's approach is explicit: an official skills repo (12 skills such as
`remotion-best-practices`, `remotion-markup`, `remotion-render`), `llms.txt` indexing every doc
page, and short rules ("Drive animations with `useCurrentFrame()` and `interpolate()`"; "Keep
`useCurrentFrame()` in the smallest component that needs it"). fframes already has the
`fframes-video` skill with four references. The library adds:

- `docs/components/components.md` (in the fork-owned `docs/`, not upstream's `skills/`):
  one import line, the `Ui` contract, the `Sequence` rule ("time is local"), the props rules,
  and a table of every component with its props and defaults. It is compiled as doctests
  (`#[cfg(doctest)] #[doc = include_str!(..)] struct ComponentsDocs;`) so examples cannot rot.
  One pointer line in the fork's `AGENTS.md` (already fork-modified) tells agents where it is.
- `components/gallery`: one scene per component, which doubles as the visual catalog, the
  snapshot test and the benchmark.
- **Exact Manim names** for anything Manim has, and Remotion's names for time primitives, so an
  agent's prior knowledge applies. Manim keyword arguments map to fields of the same name.
- **Errors that name the fix**: an unknown prop in the tag form says the valid fields; a
  dashed attribute says the Rust name; a component used without `ui` in scope says to create
  one. Missing required data is an `inspect` warning with the component and key.
- **One way to do each thing.** Time through `Sequence`, values through `Anim`, appearance through
  `enter`/`exit`, defaults through the theme. The strategy's rules 1 to 8 still apply.

## 8. Milestones (replaces the Manim research's M0 to M6)

| Milestone | Delivers | Done when |
| --- | --- | --- |
| C0 core | the `components/` workspace (section 10), crate `fframes-kit`, `Ui`, `Component`, `Sequence`, `Series`, `Loop`, `Freeze`, `Fill`, `Anim`, `Theme` (`studio` feature for `Styles` and editor keys), `FadeIn`/`FadeOut`, gallery crate with editor bridge, `components.md` doctests | gallery renders on CPU, the gallery's wasm check passes, the crate builds with and without `studio`, a copy of one example ported into the gallery renders within 10% of the original's recorded `just bench` time, and the boundary check of section 10 prints nothing |
| C1 shapes and enters | the 22 shapes, `paths` over kurbo, `Create`, `Write` (glyph outlines), `GrowFromCenter`, `DrawBorderThenFill`, `Transform` by point alignment | square-to-circle and the Manim "Create/Transform" gallery scenes |
| C2 component tags | the library's `ui!` macro wrapping `svgr!` (section 4), nested tags, error messages; animated props borrow stored timelines | the gallery scenes written in tag form render identically to the struct form (snapshot) |
| C3 text and general | ports of the overview map's text, overlays, data, audio, mockup components into the library, each with a gallery scene; upstream examples are not edited, only the fork-only `fframes-studio-intro` and `vinuni-future-rises` may switch to the library | each of the 6 duplicated helpers listed in the overview map has a library component and a gallery scene that reproduces its example's frame |
| C4 explain and math | `Canvas` (Manim units), `Axes`, `NumberPlane`, `FunctionGraph`, `MathTex` via Typst, `VGroup.arrange`, `LaggedStart`; the 12 math figures | the Manim gallery ports from the Manim research render |
| C5 Studio panel | `Props` derive, schema, panel in Studio keyed by `component_key` | editing a prop in Studio changes the preview without code |
| C6 other subjects and transitions | physics, CS, biology, chemistry figures; `transitions`; `effects` | each subject has a gallery scene per figure |

The math, LaTeX and science research (`research-261023-1718-math-latex-science-support.md`,
section 8) moves formulas into C1 and splits C6 into physics and chemistry, CS and biology,
and transitions and effects.

Location: the fork-owned `components/` directory of section 10, not the root workspace. A
later split into its own repository is then a directory move. The explain layer is a feature of
the same crate rather than a second crate until C4 shows it needs one.

## 10. No changes to upstream code

The fork pulls `dmtrKovalenko/fframes` into `main` regularly, so the library adds no lines to
any path upstream owns: the `fframes*` crates, `svgr-macro`, `media-dir-macro`, `examples/`,
`skills/`, `justfile`, `e2e/` and the root `Cargo.toml` members list. The fork already patches
`fframes/src` for Studio (`editor.rs`, `styles.rs`, `timing.rs`, parts of `svgr.rs`); the
library does not add to that.

Layout, the same pattern as `desktop/`:

```text
components/                       fork-owned, its own Cargo workspace
  Cargo.toml                      [workspace]; members below; fframes = { path = "../fframes" }
  Cargo.lock                      its own, like desktop/
  justfile                        check, test, gallery render, wasm check of the gallery bridge
  fframes-kit/                    the library (features: studio, formula, chemistry, physics, ...)
  fframes-kit-macros/             the ui! proc macro
  gallery/                        one scene per component, plus gallery/editor/editor-bridge
  (docs/components/components.md  the agent reference, compiled as doctests; docs/ is fork-owned)
```

The only edits outside `components/` are on lines the fork already owns: `components` added to
the root `Cargo.toml`'s existing `exclude = ["desktop"]` line, one pointer line in `AGENTS.md`,
and docs under `docs/`, which upstream does not have.

How each earlier core touch was replaced:

| Earlier plan | Replacement |
| --- | --- |
| relax `Frame` text helpers to `&str` | borrowed prefixes, precomputed digit widths, borrowed subtitle phrases, own glyph faces (3.2) |
| component tags as a pre-pass inside `svgr-macro` | the library's `ui!` macro wrapping `svgr!` (section 4) |
| extend `Frame`'s `TextCache` | a library `thread_local!` cache, only if measured (3.6) |
| `inspect` reports missing data | the public `fframes::diagnostics::report` (3.3) |
| de-duplicate helpers in upstream examples | gallery scenes inside `components/`; only fork-only examples may switch (C3) |
| `skills/fframes-video/references/components.md` (strategy section 5) | `docs/components/components.md` |
| `skills/fframes-video/references/manim.md` (Manim research section 6) | a section of `docs/components/components.md` |
| `examples/components-gallery` and `examples/manim-gallery` | `components/gallery` |
| crate as a root workspace member (strategy section 3) | member of the `components/` workspace |

APIs the library uses that exist upstream today: `Frame::clone_with_scene_offset`,
`second_to_frame`, `text_width`, `get_subtitle_phrase`, `FFramesContext::duration_in_frames`,
`ctx.get_image`, `MediaProvider::get_all_font_data`, `diagnostics::report` (not gated by target,
so it works in the editor build), the `ttf_parser` re-export, `KeyFramesAnimation`,
`timeline!` and `svgr!`. Fork-only APIs, behind the `studio` feature: `EditorObjectKey`,
`Svgr::with_editor_object`, `Scene::editor_instance_key`, `fframes::Styles`.

**Boundary check.** The fork already differs from upstream in `fframes/`, `fframes-media/`,
`Cargo.toml` and `AGENTS.md` (the Studio work), so comparing against `upstream/main` proves
nothing. Instead, record the commit before C0 starts and run, at every milestone:

```bash
git diff <commit-before-C0>..HEAD --stat -- fframes fframes-media fframes-editor-controller \
  fframes-skia-renderer fframes-three-d svgr-macro media-dir-macro examples skills e2e justfile
```

It must print nothing. The root `Cargo.toml` (one word on the `exclude` line) and `AGENTS.md`
(one pointer line) are checked by eye.

Risk that remains: upstream can still change a public API the library uses. That breaks the
library's build, not the merge, and the fix stays inside `components/`.

## 9. What this changes in the earlier reports

- Strategy section 4 (the component contract) is replaced by section 3 here: the struct is the
  props (no separate `Props` struct and `new(props)`), `render(self, &mut Ui)` instead of
  `render(&self, &mut Frame, ctx)` (rule 2 there), `Children` closures, `Anim<T>` props, and
  `Sequence` for time. The shared timing prop names of its rule 1 (`start`, `duration`, `exit`
  on every component) are withdrawn: time comes from the enclosing `Sequence`, and `exit` is an
  animation prop that reads `ui.duration`. Sections 2, 3, 5, 6, 7, 8, 9 and 11 of the strategy
  stand, except that every location they name (root workspace member, `examples/`, `skills/`,
  `just check-wasm`) moves into the `components/` workspace of section 10. Its section 10 roadmap (phases 0 to 7) is replaced by section 8 here, which puts
  shapes and enter animations (C1) before the general components (C3) because everything else
  uses them.
- The Manim research's section 2 (`Script`) becomes optional sugar; its sections 3 (units), 4
  (rendering) and 5 (mapping tables) stand and feed C1 and C4. Its rule that fonts and Typst
  layout are prepared at build time stands (section 3.6 here).
- The overview map is unchanged; its candidates land in `text` and `general` (C3).

## Unresolved questions

1. Struct update syntax with themed defaults (recommended, zero dependencies) or a builder
   (`bon` or `typed-builder`) so that a missing required prop is a compile error? The tag form
   works with either.
2. Should the tag form expect a variable named `ui`, or take it explicitly
   (`ui!(ui; <Circle .. />)`)? The implicit name is shorter; the explicit one has no hidden
   convention.
3. Crate name: answered by the plan (`fframes-kit`; `fframes-components` 0.1.2 on crates.io is a third-party crate). Was: `fframes-components` or `fframes-explain`?
   With the explain layer as a feature, `fframes-components` fits better.
4. `MathTex`: answered by the math research. LaTeX through MiTeX in `MathTex`, Typst in
   `MathTypst`.
5. Studio panel timing (C5): before or after the subject kits? It changes which milestone adds
   the `Props` derive.
6. Transition presentations beyond `fade`, `slide`, `wipe` and the exact `springTiming` options
   are unconfirmed in the Remotion pages read; verify before copying names.
7. Should the library ever run on vanilla upstream fframes, outside this fork? The report
   assumes yes and gates the fork-only Studio APIs behind a `studio` feature. If it only ever
   runs in this fork, the gate can go and the library can use `Styles` and editor keys
   directly.

## Sources

- Remotion: https://www.remotion.dev/docs/composition, /docs/calculate-metadata, /docs/sequence,
  /docs/series, /docs/loop, /docs/freeze, /docs/interpolate, /docs/spring, /docs/use-current-frame,
  /docs/transitions/transitionseries, /docs/transitions/presentations/custom, /docs/motion-blur/,
  /docs/light-leaks/, /docs/zod-types/, /docs/ai/coding-agents.md, https://www.remotion.dev/llms.txt
- Remotion package indexes: https://github.com/remotion-dev/remotion/tree/main/packages/{shapes,paths,animation-utils,layout-utils,zod-types}/src/index.ts
- Remotion skills: https://github.com/remotion-dev/skills (skills/remotion-markup/SKILL.md)
- Leptos: https://book.leptos.dev/view/03_components.html, https://docs.rs/leptos/latest/leptos/attr.component.html
- Dioxus: https://dioxuslabs.com/learn/0.6/reference/component_props, https://dioxuslabs.com/learn/0.6/essentials/rsx/
- Yew: https://yew.rs/docs/concepts/function-components/properties, /children, https://yew.rs/docs/concepts/html
- Builders: https://docs.rs/typed-builder, https://docs.rs/bon/latest/bon/attr.builder.html
- fframes: `svgr-macro/src/{parser.rs,node.rs,nodes_to_svgtree.rs,nodes_to_format.rs}`,
  `fframes/src/{svgr.rs,frame.rs,editor.rs,scenes.rs}`, `examples/svg-in-3d/src/lib.rs`,
  `skills/fframes-video/`
- Earlier reports: `components-261023-0753-overview-map.md`, `components-261023-0804-library-strategy.md`,
  `research-261023-1404-manim-api-for-fframes.md`
