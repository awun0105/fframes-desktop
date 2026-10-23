# fframes-kit: components

`fframes-kit` (crate `fframes_kit`, in `components/fframes-kit`) is a component library for
fframes videos: Remotion's model (props, children, time set by the parent) with Manim's names
for animations. Every code block in this file is compiled as a doctest of the kit.

## One import

```rust
use fframes_kit::prelude::*;
```

The prelude also re-exports the fframes types a video needs (`Video`, `Frame`, `Svgr`,
`Color`, `Transform`, `svgr!`, `timeline!`, `Easing`, `Scene`, `Duration`, ...).

## The model

- A component is a props struct. `ui.show(component)` renders it at the current time and
  returns an `Svgr`, so it goes anywhere `{expr}` is allowed in `svgr!`.
- Build a `Ui` once at the top of `render_frame`: `Ui::new(frame, ctx, &self.theme)` in a
  video, `Ui::for_scene(frame, ctx, &self.theme, self)` in a scene. Scenes get the video's
  context, so only `for_scene` knows the scene's length that exit animations end with.
- **Time is local.** A component has no start time. Its parent places it with a
  `Sequence { from, duration, children }`; inside, `ui.seconds()` starts at 0 and
  `ui.duration` is the sequence's length in frames. Sequences nest and their offsets add up.
- **Props are plain data**: numbers, `&str`/`String`, `Color`, `Transform`, enums, `Option`,
  `Anim<T>` (a fixed value or a stored `timeline!`) and `children` closures.
- **Defaults come from the theme**: set what you need and finish with `..ui.defaults()`.
- **SVG ids** (clip paths, gradients, filters) come from `ui.id(key, index, kind)`: from a
  `key: Option<&str>` prop when given, else from the item index. Never from a counter.
- Children are a closure that receives the container's own `Ui`: `children(|ui| ...)`.

```rust
use fframes_kit::prelude::*;

fn card<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let fade = FadeIn { shift: (0.0, -40.0), delay: 0.2, ..ui.defaults() };
    let accent = ui.theme.colors.accent;
    fade.apply(ui, svgr!(<rect x="100" y="100" width="400" height="200" fill={accent} />))
}

fn layout<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    // `card` starts at 1s and lasts 3s; inside it, time starts at 0
    ui.show(Sequence { from: 1.0, duration: Some(3.0), children: children(card) })
}
```

## Time containers and fades

| Component | Props | What it does |
| --- | --- | --- |
| `Sequence` | `from`, `duration: Option<f32>`, `children` | shows children from `from` seconds, for `duration` or until the parent ends |
| `Series` | `items: Vec<(f32, Children)>`, `overlap` | children back to back, each for its seconds; `overlap` starts each one earlier |
| `Loop` | `duration`, `times: Option<usize>`, `children` | repeats children every `duration` seconds |
| `Freeze` | `at`, `children` | children frozen at `at` seconds of their time |
| `Fill` | `background: Option<Color>`, `children` | full-canvas group (Remotion's `AbsoluteFill`) |
| `FadeIn` | `shift`, `delay`, `run_time`, `rate_func` | `Enter`: fades in while moving by `shift` into place; `fade.apply(ui, content)` |
| `FadeOut` | `shift`, `delay`, `run_time`, `rate_func` | `Exit`: fades out before the end of `ui.duration` (`delay` seconds before it) |

`RateFunc` has Manim's names: `Smooth` (default), `Linear`, `RushInto`, `RushFrom`,
`ThereAndBack`, `DoubleSmooth`, plus `Easing(fframes::animation::Easing)` for CSS curves and
springs. `motion` has `progress`, `clamp01`, `lerp`, `stagger(i, step)`, `spring`, `hash`,
`noise`, the easing curves `expo_in`, `expo_out`, `cubic_in_out` and `quart_out` on `0..=1`,
`timecode(frames, fps)` (`hh:mm:ss:ff`, an error for 0 fps) and `thousands(n)` (`12,480`).

```rust
use fframes_kit::prelude::*;

assert_eq!(motion::progress(0.5, 0.0, 2.0), 0.25);
assert_eq!(RateFunc::Linear.apply(0.3, 1.0), 0.3);
assert_eq!(motion::noise(7, 1.25), motion::noise(7, 1.25));
assert_eq!(motion::timecode(3_725, 60).unwrap(), "00:01:02:05");
assert_eq!(motion::thousands(12_480), "12,480");
```

For a video cut to music, `Beat { period, downbeat }` is the beat grid: beat 0 is the
first downbeat, at `downbeat` seconds, and beats are `period` seconds apart. It stores the
period rather than the BPM, so a measured period is used exactly; `Beat::from_bpm(bpm,
downbeat)` makes one from a tempo. `time(beat)` and `frame(beat, fps)` place things on the
grid (frames are rounded, so scene edges never drift), `beat_at(seconds)` reads it back, and
`snap(beats)` and `soft(beats)` are springs released `beats` ago (their `(stiffness,
damping)` are the fields `snap_spring` (320, 22) and `soft_spring` (170, 15)).
`Beat::pulse(beat, sharpness)` is 1 on every beat and decays until the next.

```rust
use fframes_kit::prelude::*;

const GRID: Beat = Beat::new(0.5, 2.0); // 120 BPM, first downbeat at 2 s

assert_eq!(GRID.time(4.0), 4.0);
assert_eq!(GRID.frame(4.0, 30), 120);
assert_eq!(GRID.beat_at(3.0), 2.0);
assert_eq!(Beat::pulse(3.0, 8.0), 1.0);
```

## Text

Text components measure with the font files of the video's media, so a layout is the same
in the editor and in the render. When a family cannot be measured (a system font, or the
editor before fonts load), each falls back to a plain `<text>`, as listed. `text` is a
`Cow<str>`: pass `"literal".into()` or `format!(..).into()`. `style` is a `TextStyle` from
`theme.typography`, and its `family`, `size` and `weight` are what is measured and drawn.
`anchor` is `TextAnchor::{Start, Middle, End}`.

| Component | Props | What it does | Without a font |
| --- | --- | --- | --- |
| `FitText` | `text`, `x`, `y`, `style`, `max_width`, `min_size` (24), `step` (4), `fill`, `anchor`, `opacity` | one line at the largest size up to `style.size` that fits `max_width`; `fit.size(ui)` gives the size | `style.size` |
| `Letters` | `text`, `x`, `y`, `style`, `fill`, `tracking`, `look: Box<dyn Fn(usize) -> LetterLook>` | each letter moved, faded and scaled about its center by `look(i)` (`LetterLook { dx, dy, opacity, scale }`) | one line with `look(0)` |
| `TextReveal` | `text`, `x`, `y`, `style`, `fill`, `by` (`RevealBy::Word`, `Letter`), `shift`, `delay`, `stagger`, `run_time`, `rate_func` | words or letters fade in one after another while they move by `shift` into place | the whole line as unit 0 |
| `Typewriter` | `text`, `x`, `y`, `style`, `fill`, `delay`, `chars_per_second` (24), `cursor: Option<Color>`, `cursor_width`, `blink` (0.5 s) | types one character at a time, with a cursor that blinks once the text is complete | no cursor |
| `Counter` | `from`, `to`, `x`, `y`, `style`, `fill`, `anchor` (`End`), `decimals`, `thousands` (true), `prefix`, `suffix`, `delay`, `run_time`, `rate_func` | counts from `from` to `to`; right aligned, so the digits stay in place as the number grows | the same |
| `Paragraph` | `text`, `x`, `y`, `width`, `style`, `fill`, `align: TextAlign`, `opacity` | wraps into lines of at most `width` pixels, `style.line_height` apart; `\n` starts a line | one unwrapped line |
| `HudLabel` | `text`, `x`, `y`, `style` (caption), `fill` (muted), `anchor`, `letter_spacing` (2.5), `uppercase` (true), `opacity` | a small tracked label, like `01 / RENDER` | the same |

```rust
use fframes_kit::prelude::*;

/// A stat: a label, then the number counting up under it.
fn stat<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let label = ui.show(HudLabel { text: "frames rendered".into(), x: 96.0, y: 400.0, ..ui.defaults() });
    let number = ui.show(Counter {
        to: 12_480.0,
        x: 96.0,
        y: 520.0,
        anchor: TextAnchor::Start,
        delay: 0.2,
        ..ui.defaults()
    });
    let title = ui.show(TextReveal { text: "Twelve thousand frames".into(), x: 96.0, y: 700.0, ..ui.defaults() });
    svgr!(<g>{label}{number}{title}</g>)
}

let c = Counter { to: 1_000.0, decimals: 1, suffix: " ms".into(), ..Counter::default() };
assert_eq!(c.format(c.value(100.0)), "1,000.0 ms");
```

`Letters` takes any per-letter animation. Here each letter drops in with a spring, 60 ms
after the one before:

```rust
use fframes_kit::prelude::*;

fn drop_in<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    ui.show(Letters {
        text: "LAUNCH".into(),
        x: 96.0,
        y: 600.0,
        look: Box::new(move |i| {
            let s = motion::spring(t - motion::stagger(i, 0.06), 260.0, 18.0);
            LetterLook { dy: (1.0 - s) * -120.0, opacity: motion::clamp01(s * 2.0), ..LetterLook::default() }
        }),
        ..ui.defaults()
    })
}
```

## Overlays

Overlays sit on top of a scene: who is speaking, what is said, where in the video we are.
They take their times from the container they are shown in (`ui.seconds()` and
`ui.duration`), so the same lower third works in any scene or `Sequence`, and the ones
that leave do so before the container ends.

| Component | Props | What it does |
| --- | --- | --- |
| `Appear` | `Appear::new(ui, from, center, children)`, then `until`, `rise` (60), `scale_from` (0.92), `enter`, `exit`, `rate_func` | children rise, grow about `center` and fade in at `from` seconds, and leave upward at `until` (or the end of the container); inside, time starts at `from` |
| `LowerThird` | `name`, `role`, `x`, `y`, `name_style`, `role_style`, `name_fill`, `role_fill`, `bar`, `bar_width` (6), `background: Option<Color>`, `min_width`, `padding`, `delay`, `stagger`, `run_time`, `exit`, `rate_func` | an accent bar grows, then the name and the role slide out from behind it; all of it leaves in reverse order before the end. With `background`, a plate as wide as the longer line |
| `Captions` | `file`, `x`, `y`, `width`, `style`, `fill`, `align`, `video_time` (true) | the cue of a WebVTT file that plays now, wrapped into `width`; cue times are video seconds, or container seconds with `video_time: false` |
| `CaptionStack` | `items: Vec<Caption>` (`Caption::new(at, tag, line)`), `x`, `y`, `gap`, `tag_style`, `line_style`, `tag_fill`, `line_fill`, `tag_spacing`, `rise`, `enter`, `leave`, `rate_func` | timed captions without a file: each rises in at its `at` and leaves upward while the next comes in; the last stays |
| `ChapterList` | `chapters: Vec<(f32, Cow<str>)>`, `x`, `y`, `width`, `row_height`, `gap`, `style`, `fill`, `active_fill`, `highlight`, `radius`, `spring` | an agenda whose highlight springs to the chapter playing now |
| `Annotation` | `target`, `label`, `text`, `color`, `style`, `letter_spacing`, `stroke_width`, `delay`, `run_time`, `rate_func` | a dot on `target`, an elbow line to `label` and the text at its end; the line draws on, the text fades in |
| `ProgressBar` | `x`, `y`, `width`, `height` (8), `radius`, `track`, `fill`, `progress: Option<f32>` | how far the container is (`None`), or a given value from 0 to 1 |
| `Check` | `x`, `y`, `size`, `color`, `stroke_width` (5), `delay`, `run_time`, `rate_func` | a check mark that draws itself on |

`Annotation` is the HUD callout with a line; `Callout` (under Shapes) is the speech bubble.
`Captions` has no glow of its own: wrap it in a `<g filter=..>` with the `Glow` decoration.

A speaker introduced over the first four seconds of a scene, with captions from a file
under it and the video's progress at the bottom:

```rust
use fframes_kit::prelude::*;

fn speaker<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let intro = Sequence { from: 0.5, duration: Some(4.0), children: children(|ui| {
        ui.show(LowerThird { name: "Ada Lovelace".into(), role: "Analyst".into(), ..ui.defaults() })
    }) };
    let intro = ui.show(intro);
    let captions = ui.show(Captions { file: "talk.vtt", y: 980.0, ..ui.defaults() });
    let progress = ui.show(ProgressBar { y: 1064.0, height: 4.0, ..ui.defaults() });
    svgr!(<g>{intro}{captions}{progress}</g>)
}

let list = ChapterList {
    chapters: vec![(0.0, "Intro".into()), (5.0, "Demo".into())],
    ..ChapterList::default()
};
assert_eq!(list.active(6.0), 1);
```

## Data, audio, decoration and media

| Component | Props | What it does |
| --- | --- | --- |
| `BarChart` | `bars: Vec<(Cow<str>, f64)>`, `x`, `y` (baseline), `bar_width` (126), `gap` (62), `max_height` (330), `max_value`, `radius`, `fill`, `highlight: Option<usize>`, `highlight_fill`, `value_style`, `value_fill`, `label_style`, `label_fill`, `decimals`, `delay`, `stagger`, `run_time`, `rate_func` | bars grow one after another, each value counting up above its bar and its label below; a bar starts as a hairline |
| `SpectrumBars` | `values`, `x`, `y`, `width`, `bar_width`, `gain` (60), `min_height`, `max_height` (720), `radius` (`None`: round ends), `fill`, `align` (`BarAlign::Center`, `Bottom`) | one rounded bar per value, `value * gain` pixels high; NaN draws the shortest bar |
| `Waveform` | `values`, `x`, `y` (baseline), `width`, `gain` (1200), `max_height` (600), `fill`, `stroke`, `stroke_width`, `opacity` | a smooth closed wave through the values, starting and ending on the baseline (`audio::wave_path` gives the path data) |
| `DrawOn` | `DrawOn::new(ui, shape)`, then `delay`, `run_time`, `rate_func` | `Create` on any `Shape` you built: an underline, a tick, a hand-drawn arrow draws on from its start |
| `CornerMarks` | `x`, `y`, `width`, `height`, `arm` (24), `color`, `stroke_width`, `enter: Option<EnterAnim>` | crop-mark corners around a rectangle, static or drawn on with `enter` |
| `Glow` | `id` (`kit-glow`), `shadow` (`theme.shadow.glow`) | a filter definition; draw it in `<defs>`, then `filter={glow.url()}` |
| `DropShadow` | `id` (`kit-card`), `shadow` (`theme.shadow.card`) | the same for the drop shadow under a card |
| `PhotoFrame` | `image`, `key`, `x`, `y`, `width`, `height`, `radius`, `stroke`, `stroke_width` (12), `opacity` | an image of the media clipped to a rounded, stroked frame |
| `KenBurns` | `image`, `key`, `x`, `y`, `width`, `height`, `from`, `zoom` (1.06), `zoom_per_second` (0.02), `drift`, `fade` | an image slowly pushed in and drifted sideways from `from` seconds, clipped to its box |

`audio::spectrum(ui, file, SampleSize::S256, smooth)` gives the values for the audio
components: the spectrum of `file` at the current frame, or `None` when the file is not in
the video's or the scene's `fn audio`. The frame is the container's, so a scene's file
should start with the scene. `PhotoFrame` and `KenBurns` draw nothing for a missing image;
`key` names their clip path when two are in one frame.

`HudLabel::draw()`, `CornerMarks::draw()` (the static marks) and `Annotation::draw(progress)`
need no `Ui`, so helpers of a video that only have a `Frame` can draw them too
(`examples/fframes-studio-intro/src/brand.rs` does).

A level meter on a podcast cover, a highlighted word that gets underlined, and a glowing
title:

```rust
use fframes_kit::prelude::*;

fn cover<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let values = audio::spectrum(ui, "episode.mp3", fframes::SampleSize::S256, 4)
        .map(|s| s.into_iter().take(48).collect())
        .unwrap_or_default();
    let meter = ui.show(SpectrumBars { values, x: 160.0, y: 820.0, width: 1600.0, ..ui.defaults() });

    let path = BezPath::from_svg("M200 660 H820").unwrap_or_default();
    let stroke = Some(ui.theme.colors.accent);
    let underline = Shape { path, fill: None, stroke, stroke_width: 8.0, dash: None, opacity: 1.0 };
    let underline = ui.show(DrawOn { delay: 0.4, ..DrawOn::new(ui, underline) });

    let glow = Glow::default();
    let url = glow.url();
    let defs = ui.show(glow);
    let title = ui.show(HudLabel { text: "Episode 12".into(), x: 200.0, y: 620.0, ..ui.defaults() });
    svgr!(<g><defs>{defs}</defs><g filter={url}>{title}</g>{underline}{meter}</g>)
}

let chart = BarChart { bars: vec![("a".into(), 10.0), ("b".into(), 20.0)], ..BarChart::default() };
assert_eq!(chart.grown(1, 0.0), 0.0);
```

## Formulas

Feature `latex`: `fframes-kit = { features = ["latex"] }`. The kit runs the TeX installed on
the machine, as Manim does: `latex` makes a DVI file, `dvisvgm --no-fonts` turns it into
paths, and each glyph becomes a `<path>`. Any package your TeX has works: amsmath, mhchem,
physics, siunitx, TikZ, chemfig, circuitikz. Installing and checking TeX on Windows, Ubuntu
and macOS is in [latex-setup.md](../latex-setup.md).

| Type | What it is |
| --- | --- |
| `TexEngine::new(cache_dir)` | finds `latex`, `xelatex`, `lualatex` and `dvisvgm`, runs them with a timeout (120 s, `with_timeout`) and keeps every SVG in `cache_dir` |
| `MathTex::new(tex)` | math mode in an `align*` environment (`&` aligns, `\\` breaks a line), `.template(..)`, `.typeset(&engine) -> Result<Formula, TexError>` |
| `Tex::new(text)` | text mode: `$..$`, whole TikZ pictures, anything a document body holds |
| `ChemEq::new("2H2 + O2 -> 2H2O")` | `\ce{..}` with the chemistry template, one part per species |
| `TexTemplate` | `compiler` (`TexCompiler::Latex`, `XeLatex`, `LuaLatex`), `preamble`, `body_wrapper` (with `YourTextHere`); `default()` (standalone, amsmath, amssymb), `chemistry()`, `physics()`, `tikz()`, `add_to_preamble` |
| `Formula` | the glyphs in TeX points, `part_bbox(part)`, `size_pt()`; implements `Geometry` |
| `FormulaView` | the component: `of`, `size` (pixels of 10 pt text, 48), `x`, `y`, `anchor` (`Anchor::TopLeft`, `Center`, ...), `color` (replaces black, keeps the colours the source sets), `color_map: &[(Part, Color)]`, `opacity`, `enter`, `exit`, `emphasis`, `emphasize: Option<Part>` (the part `emphasis` applies to), `key` |
| `TransformMatchingTex` | `from`, `to` (two `FormulaView`s), `delay`, `run_time`, `rate_func` |

Rules:

- Typeset in the video's constructor or a `OnceLock`, never in `render_frame`. A new formula
  takes about a second; one from the cache takes a file read.
- `{{ }}` marks a part, as in Manim: `{{a}}^2 + {{b}}^2 = {{c}}^2` has the parts 0, 1 and 2
  (`Part::Index`). `\part{name}{..}` names one (`Part::Label("name")`). A part is a braced
  group, so `{{+}}` is spaced like an ordinary symbol and a part cannot hold an `&`.
- `TransformMatchingTex` matches labels first, then parts with the same source, then glyphs
  outside parts with the same outline. Matched glyphs move, the rest fades out and in.
- `ChemEq` makes each species a part: separate `+` and arrows with spaces.
- Commit the cache directory. Open it at a path that does not depend on the working directory,
  `concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache")`, and machines without TeX (CI, other
  people) render from it. A formula that is neither cached nor typesettable is a
  `TexError::NotInstalled` that names it.
- The web editor cannot run TeX. Draw `Formula::unset(source)` when `typeset` fails: the
  view then shows the source as text, and `inspect` lists it as the missing font
  `fframes-kit formula not typeset`.
- With `emphasize` and a `Write` entrance, the part and the rest of the formula each stagger
  their glyphs over the whole `Write`.

```rust
# #[cfg(feature = "latex")]
# mod example {
use std::sync::OnceLock;

use fframes_kit::prelude::*;

struct Formulas {
    pythagoras: Formula,
    solved: Formula,
}

fn typeset() -> Result<Formulas, TexError> {
    let engine = TexEngine::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache"));
    Ok(Formulas {
        pythagoras: MathTex::new("{{a}}^2 + {{b}}^2 = {{c}}^2").typeset(&engine)?,
        solved: MathTex::new("{{a}}^2 = {{c}}^2 - {{b}}^2").typeset(&engine)?,
    })
}

// filled in the video's constructor: `FORMULAS.get_or_init(typeset)`
static FORMULAS: OnceLock<Result<Formulas, TexError>> = OnceLock::new();

fn solve<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let Ok(f) = FORMULAS.get_or_init(typeset) else {
        return Svgr::empty();
    };
    let theme = ui.theme;
    let view = |of| FormulaView {
        size: 120.0,
        x: 960.0,
        y: 540.0,
        anchor: Anchor::Center,
        ..FormulaView::new(of).themed(theme)
    };
    let map = [(Part::Index(0), theme.colors.accent)];
    let from = FormulaView { color_map: &map, ..view(&f.pythagoras) };
    ui.show(TransformMatchingTex { delay: 0.5, ..TransformMatchingTex::new(from, view(&f.solved)) })
}

# pub fn chem() {
let reaction = ChemEq::new("CH4 + 2O2 -> CO2 + 2H2O");
assert_eq!(fframes_kit::latex::chem_source(reaction.ce).matches("{{").count(), 4);
# }
# }
# fn main() {
# #[cfg(feature = "latex")]
# example::chem();
# }
```

## Shapes

Every shape is a component with its geometry props, the paint props `fill`, `stroke`,
`stroke_width`, `dash`, `opacity`, and the animation props `enter`, `exit`, `emphasis`.
Positions are canvas pixels with `y` down, angles are radians, counterclockwise on screen as in
Manim. Shapes are placed by their center (`cx`, `cy`) unless points give them. Manim shapes are
accent outlines by default, Remotion shapes (and Manim's `Dot`, `Annulus`, `Sector`) are filled.

| Shape | Props | Source |
| --- | --- | --- |
| `Circle` | `cx`, `cy`, `radius` | Manim, Remotion |
| `Dot` | `cx`, `cy`, `radius` (8) | Manim |
| `Ellipse` | `cx`, `cy`, `width`, `height` | Manim, Remotion |
| `Arc` | `cx`, `cy`, `radius`, `start_angle`, `angle` | Manim |
| `Annulus` | `cx`, `cy`, `inner_radius`, `outer_radius` | Manim |
| `Sector` | `cx`, `cy`, `radius`, `start_angle`, `angle` | Manim |
| `Line`, `DashedLine` | `x1`, `y1`, `x2`, `y2` (`dash` is `(12, 12)` on `DashedLine`) | Manim |
| `Polygon` | `points: Vec<(f32, f32)>` | Manim, Remotion |
| `RegularPolygon` | `cx`, `cy`, `radius`, `n`, `start_angle: Option<f32>` | Manim |
| `Triangle` | `cx`, `cy`, `radius` (equilateral, pointing up) | Manim, Remotion |
| `Rectangle`, `Square`, `RoundedRectangle` | `cx`, `cy`, `width`/`height` or `side_length`, `corner_radius` | Manim |
| `Star` | `cx`, `cy`, `n`, `outer_radius`, `inner_radius` | Manim, Remotion |
| `Arrow`, `DoubleArrow` | `x1`, `y1`, `x2`, `y2`, `tip_length`, `tip_width` | Manim |
| `Vector` | `x`, `y`, `dx`, `dy`, `tip_length`, `tip_width` | Manim |
| `CurvedArrow` | `x1`, `y1`, `x2`, `y2`, `angle`, `tip_length`, `tip_width` | Manim |
| `Brace` | `x1`, `y1`, `x2`, `y2`, `depth`; `brace.tip()` places a label | Manim |
| `Rect` | `cx`, `cy`, `width`, `height`, `corner_radius` | Remotion |
| `BlockArrow` | `cx`, `cy`, `length`, `head_width`, `head_length`, `shaft_width`, `angle` | Remotion `Arrow` |
| `Callout` | `cx`, `cy`, `width`, `height`, `corner_radius`, `tail_x`, `tail_y`, `tail_width` | Remotion |
| `Heart` | `cx`, `cy`, `height` | Remotion |
| `Pie` | `cx`, `cy`, `radius`, `progress`, `rotation`, `closed` | Remotion |
| `Spark` | `cx`, `cy`, `size`, `points`, `inner` | Remotion |
| `TextPath` | `text`, `x`, `y` (baseline start), `style: TextStyle` | Manim `Text` |

`shape.shape()` gives the `Shape` (path and paint) and every shape implements `Geometry`
(`path()`, `length()`, `bbox()`). The path is a `kurbo::BezPath`; `fframes_kit::kurbo` is
re-exported. `TextPath` draws glyph outlines from the font files of the video's media and
falls back to a `<text>` (faded in word by word by `Write`) when the family has none.

## Geometry animations

Animations are plain data. Put them into a shape's `enter`, `exit` or `emphasis` with
`.into()`. Each has Manim's fields: `delay`, `run_time`, `rate_func`, plus its own.

| Animation | Slot | What it does |
| --- | --- | --- |
| `FadeIn`, `FadeOut` | enter, exit | fade with an optional `shift` |
| `Create`, `Uncreate` | enter, exit | draws the outline from its start (and fades the fill in), or back |
| `DrawBorderThenFill` | enter | draws the outline, then fills (`run_time` 2) |
| `Write` | enter | `DrawBorderThenFill` per glyph of a `TextPath`, `lag_ratio` |
| `GrowFromCenter`, `GrowFromPoint { point }`, `GrowArrow` | enter | grows from nothing about the center, a point, or the path's start |
| `SpinInFromNothing { angle }` | enter | grows while turning into place |
| `Indicate { scale_factor, color }` | emphasis | scales up and tints, then back |
| `Circumscribe { color, buff }` | emphasis | a rectangle runs once around the shape |
| `Flash { point, num_lines, flash_radius, line_length }` | emphasis | lines shoot out once |
| `Wiggle { scale_value, rotation_angle, n_wiggles }` | emphasis | shakes the shape |

Enter and emphasis animations count from the start of the enclosing container plus `delay`;
exit animations end `delay` seconds before its end. They change only the transform, opacity
and drawn part of the outline, never the path.

| Component | Props | What it does |
| --- | --- | --- |
| `Morph` (Manim `Transform`) | `from: Shape`, `to: Shape`, `delay`, `run_time`, `rate_func` | morphs path, paint and width; `Morph::new(from, to)` |
| `ReplacementTransform` | the same | `Morph` that draws `to` itself once done |
| `MoveAlongPath` | `path: BezPath`, `delay`, `run_time`, `rate_func` | `Enter`: moves content from the path's start to its end |
| `LaggedStart` | `lag_ratio`, `run_time`, `children` | child `i` starts `i * lag_ratio * run_time` seconds in |
| `AnimationGroup` | `children` | every child at once |
| `Succession` | `run_time`, `children` | each child `run_time` after the one before |

The prelude calls Manim's `Transform` `Morph`, because `Transform` there is the SVG transform;
it is also `fframes_kit::animate::Transform`.

```rust
use fframes_kit::prelude::*;

/// Manim's SquareToCircle: create a square, morph it into a circle, fade it out.
fn square_to_circle<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let square = Square { cx: 960.0, cy: 540.0, side_length: 400.0, ..ui.defaults() };
    let circle = Circle {
        cx: 960.0,
        cy: 540.0,
        radius: 220.0,
        fill: Some(ui.theme.colors.highlight),
        ..ui.defaults()
    };
    let (from, to) = (square.shape(), circle.shape());
    let create = ui.show(Sequence {
        from: 0.0,
        duration: Some(1.2),
        children: children(move |ui| {
            ui.show(Square { enter: Some(Create::default().into()), ..square.clone() })
        }),
    });
    let morph = ui.show(Sequence {
        from: 1.2,
        duration: None,
        children: children(move |ui| {
            let morph = ui.show(Morph::new(from.clone(), to.clone()));
            FadeOut { run_time: 0.8, ..ui.defaults() }.apply(ui, morph)
        }),
    });
    svgr!(<g>{create}{morph}</g>)
}
```

## Paths

`fframes_kit::paths` has Remotion's `@remotion/paths` in snake_case over `kurbo::BezPath`:
`get_length`, `get_point_at_length`, `get_tangent_at_length`, `evolve_path`,
`interpolate_path`, `translate_path`, `scale_path`, `reverse_path`, `cut_path`, `warp_path`,
`normalize_path`, `get_bounding_box`, `get_subpaths`, `extend_view_box`, `center_path`,
`parse_path`, `serialize`, `reset_path`, `reduce_instructions`, and `align_points(a, b, n)`
(the same number of subpaths and cubic segments in both, turned to start at the nearest
points), which `Morph` runs on.

```rust
use fframes_kit::prelude::*;

let square = paths::parse_path("M0 0 L10 0 L10 10 L0 10 Z").unwrap();
assert_eq!(paths::get_length(&square), 40.0);
assert_eq!(paths::get_length(&paths::cut_path(&square, 20.0)), 20.0);
let half = paths::interpolate_path(0.5, &square, &paths::scale_path(&square, 2.0, 2.0));
assert!((paths::get_bounding_box(&half).width() - 15.0).abs() < 1e-9);
```

## Transitions

`TransitionSeries` is Remotion's `<TransitionSeries>`: sequences one after another, with a
transition between two of them. A transition overlaps its two sequences by its duration,
so both show during it and the series is shorter than its sequences by the sum of the
transitions. The entering sequence is drawn on top.

| Item or component | Props | What it does |
| --- | --- | --- |
| `TransitionSeries` | `key: Option<&str>`, `items` | the series; `key` keeps clip ids apart when two series share a frame |
| `TransitionSeries::sequence(seconds, children)` | | a sequence; inside, time starts at 0 and `ui.duration` is its length |
| `TransitionSeries::transition(timing, presentation)` | | a transition between the sequences on either side |
| `Fade` | `should_fade_out_exiting_scene` (false) | the entering sequence fades in over the exiting one (Remotion `fade()`) |
| `Slide` | `direction` (`FromLeft`), `size` (1920 x 1080) | the entering sequence pushes the exiting one out (`slide()`) |
| `Wipe` | `direction` (`FromLeft`), `size` | an edge moves across and uncovers the entering sequence (`wipe()`) |
| `Timing::Linear` | `duration`, `easing: RateFunc` | Remotion's `linearTiming`; the theme default is `motion.duration.slow` with `motion.easing.standard` |
| `Timing::Spring` | `mass`, `stiffness`, `damping`, `duration: Option<f32>` | `springTiming`: lasts until the spring settles, or is squeezed into `duration` |
| `SceneFade` | `fade_in`, `fade_out`, `rate_func`, `children` | fades at both edges of the enclosing sequence or scene; `SceneFade::new(ui, children)` takes the theme's `motion.duration.fast` |

`direction` is `FromLeft`, `FromRight`, `FromTop` or `FromBottom`. `size` is the canvas in
view-box units. Change it when the view box is not 1920 x 1080. A transition that is longer
than a sequence next to it is shortened to that sequence. A transition that is first, last
or next to another transition is skipped. `inspect` warns about both.

```rust
use fframes_kit::prelude::*;

fn panel<'a>(ui: &mut Ui<'a, '_, '_>, background: Color) -> Svgr<'a> {
    ui.show(Fill { background: Some(background), children: children(|_| Svgr::empty()) })
}

/// Two 1.5 s panels with a 0.8 s wipe: 2.2 s in all.
fn wipe<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let timing: Timing = ui.defaults();
    ui.show(TransitionSeries {
        key: None,
        items: vec![
            TransitionSeries::sequence(1.5, children(move |ui| panel(ui, c.surface))),
            TransitionSeries::transition(timing, Wipe { direction: Direction::FromRight, ..Wipe::default() }),
            TransitionSeries::sequence(1.5, children(move |ui| panel(ui, c.accent))),
        ],
    })
}
```

A presentation of your own implements `Presentation`. `apply` gets the progress, 0 when the
transition starts and 1 when it ends, for both sides. A spring may overshoot 1. `side` is
`Side::Entering` or `Side::Exiting`, and `id` is unique to the transition in the frame, for
clip paths, masks and gradients.

```rust
use fframes_kit::prelude::*;

/// The entering sequence grows from the center of the canvas.
struct Zoom;

impl Presentation for Zoom {
    fn apply<'a>(&self, progress: f32, side: Side, content: Svgr<'a>, _ui: &Ui<'a, '_, '_>, _id: &str) -> Svgr<'a> {
        if side == Side::Exiting {
            return content;
        }
        let s = motion::lerp(0.6, 1.0, progress);
        let shift = Transform::translate(960.0 * (1.0 - s), 540.0 * (1.0 - s));
        svgr!(
            <g transform={shift} opacity={motion::clamp01(progress)}>
                <g transform={Transform::scale(s)}>{content}</g>
            </g>
        )
    }
}

let spring = Timing::Spring { mass: 1.0, stiffness: 100.0, damping: 10.0, duration: None };
assert!(spring.duration() > 0.3);
assert_eq!(spring.progress(0.0, spring.duration()), 0.0);
assert_eq!(spring.progress(spring.duration(), spring.duration()), 1.0);
```

Core scenes can transition without a series through their `Overlap`. In `render_frame`, wrap
the content with `scene_transition(ui, &presentation, timing, side, key, content)`, using a
`Ui` made with `Ui::for_scene`. `Side::Entering` runs over the scene's first
`timing.duration()` seconds, so give the scene `Overlap::Previous` of the same length.
`Side::Exiting` runs over the last seconds, and goes on the scene before it.

```rust
use fframes_kit::prelude::*;

#[derive(Debug)]
struct Second {
    theme: Theme,
}

const SLIDE: Timing = Timing::Linear { duration: 0.5, easing: RateFunc::Smooth };

impl Scene for Second {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(3.0)
    }

    fn overlap(&self) -> Overlap {
        Overlap::Previous(0.5) // starts under the end of the scene before
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::for_scene(frame, ctx, &self.theme, self);
        let content = ui.show(Fill { background: Some(ui.theme.colors.accent), children: children(|_| Svgr::empty()) });
        let slide = Slide { direction: Direction::FromRight, ..Slide::default() };
        scene_transition(&ui, &slide, SLIDE, Side::Entering, Some("second"), content)
    }
}
# let _ = Second { theme: Theme::default() };
```

## Animated props

`Anim<T>` is a fixed value or a `timeline!` stored once in the video; `ui.sample(&anim)`
reads it at the current time.

```rust
use fframes::animation::KeyFramesAnimation;
use fframes_kit::prelude::*;

struct Pulse {
    radius: KeyFramesAnimation<f32>,
}

impl Pulse {
    fn new() -> Self {
        Self { radius: timeline!(at 0.0 => 1.0, animate 40.0_f32 => 80.0, Easing::EaseInOut) }
    }

    fn draw<'a>(&'a self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let radius: Anim<'a, f32> = (&self.radius).into();
        let fixed: Anim<'a, f32> = 20.0.into();
        let (r, small) = (ui.sample(&radius), ui.sample(&fixed));
        svgr!(<g><circle cx="200" cy="200" r={r} /><circle cx="400" cy="200" r={small} /></g>)
    }
}
# let _ = Pulse::new();
```

## Theme

`Theme` holds typed groups: `colors`, `typography`, `spacing`, `radius`, `stroke`, `shadow`,
`motion`. Override with struct update syntax:

```rust
use fframes_kit::prelude::*;

let theme = Theme {
    colors: Colors { accent: Color::hex("#ff5500"), ..Colors::default() },
    motion: Motion { stagger: Stagger { tight: 0.04, loose: 0.1 }, ..Motion::default() },
    ..Theme::default()
};
assert_eq!(theme.typography.body.font().family, "DM Sans");
```

The default typography uses DM Sans: ship the font in the video's media folder.

## Your own component

A component is a struct and one `render`. Components with theme defaults implement `Themed`.

```rust
use fframes_kit::prelude::*;

pub struct Badge<'a> {
    pub label: &'a str,
    pub fill: Color,
    pub x: f32,
    pub y: f32,
}

impl Themed for Badge<'_> {
    fn themed(theme: &Theme) -> Self {
        Self { label: "", fill: theme.colors.accent, x: 0.0, y: 0.0 }
    }
}

impl<'a> Component<'a> for Badge<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let s = ui.theme.typography.caption;
        let enter: FadeIn = ui.defaults();
        let badge = svgr!(
            <g transform={Transform::translate(self.x, self.y)}>
                <rect x="0" y="0" width="240" height="56" rx="28" fill={self.fill} />
                <text x="24" y="38" font-family={s.family} font-size={s.size} fill="#fff">{self.label}</text>
            </g>
        );
        enter.apply(ui, badge)
    }
}

// A list of different components is a `Vec<BoxedComponent>`.
fn badges<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let list: Vec<BoxedComponent<'a>> = vec![
        Box::new(Badge { label: "new", ..ui.defaults() }),
        Box::new(Badge { label: "beta", x: 300.0, ..ui.defaults() }),
    ];
    ui.show(list)
}
```

## A whole video

```rust
use fframes_kit::prelude::*;

#[derive(Debug)]
pub struct MyVideo {
    pub theme: Theme,
}

impl Video for MyVideo {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(6.0)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::new(frame, ctx, &self.theme);
        let c = self.theme.colors;
        let title = self.theme.typography.title;
        let content = ui.show(Series {
            items: vec![
                (3.0, children(move |ui| {
                    let fade = FadeIn { shift: (0.0, -40.0), ..ui.defaults() };
                    let out: FadeOut = ui.defaults();
                    let text = svgr!(
                        <text x="96" y="540" font-family={title.family} font-size={title.size}
                              font-weight={title.weight} fill={c.title}>"Hello"</text>
                    );
                    let entered = fade.apply(ui, text);
                    out.apply(ui, entered)
                })),
                (3.0, children(move |ui| {
                    let fade: FadeIn = ui.defaults();
                    fade.apply(ui, svgr!(<circle cx="960" cy="540" r="200" fill={c.accent} />))
                })),
            ],
            overlap: 0.0,
        });
        svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080"
                 width={ctx.current_video_size.width} height={ctx.current_video_size.height}>
                <rect width="1920" height="1080" fill={c.background} />
                {content}
            </svg>
        )
    }
}
# let _ = MyVideo { theme: Theme::default() };
```

The gallery (`components/gallery`) has one scene per component; render one with
`cargo run --release -p components-gallery -- frame Sequence@50%` from `components/`.
