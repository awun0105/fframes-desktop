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

### Numbers and Manim's text mobjects

| Component | Props | What it does | Without a font |
| --- | --- | --- | --- |
| `DecimalNumber` | `value: ValueTracker`, `x`, `y`, `style` (title), `fill`, `anchor` (`Start`), `num_decimal_places` (2), `include_sign`, `group_with_commas` (true), `unit`, `opacity` | the value with every digit in a slot as wide as the widest digit, so digits do not jitter; `anchor: End` keeps them in place across a carry | one line |
| `Integer` | as `DecimalNumber` without `num_decimal_places` | the value rounded to a whole number | one line |
| `Variable` | `label`, `value`, `x`, `y`, `style` (heading), `fill`, `value_fill` (accent), `num_decimal_places` (2), `unit`, `opacity` | `label = value`, the label in place and the number after it | one line |
| `BulletedList` | `items: Vec<Cow<str>>`, `x`, `y`, `style` (body), `fill`, `bullet_fill` (accent), `indent`, `line_spacing`, `opacity` | one line per item after a round bullet (a circle, no glyph needed); `baseline(i)` gives a line's `y` | the same |
| `Title` | `text`, `x` (960), `y` (130), `style`, `fill`, `include_underline` (true), `match_underline_width_to_text`, `underline_width` (1720), `underline_buff`, `underline_stroke`, `underline_stroke_width`, `opacity` | a centered heading over an underline | the underline `underline_width` wide |
| `MarkupText` | `markup`, `x`, `y`, `style` (heading), `fill`, `anchor`, `opacity` | one line with Pango tags: `<b>`, `<i>`, `<u>`, `<span>` with `foreground`, `size` (pixels or `xx-small` to `xx-large`), `weight`, `style`, `underline` | the same |

`value` is a `ValueTracker`: a fixed number (`3.5.into()`), a stored `timeline!`
(`(&self.price).into()`), or a value computed from time in `render_frame`
(`f(ui.seconds()).into()`), which is Manim's `always_redraw`. It is an `f32`, exact to about
seven significant digits.

```rust
use fframes_kit::prelude::*;

fn readout<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let title = ui.show(Title { text: "Free fall".into(), ..ui.defaults() });
    let height = ui.show(Variable {
        label: "h".into(),
        value: (100.0 - 4.905 * t * t).max(0.0).into(),
        x: 200.0,
        y: 400.0,
        unit: " m",
        ..ui.defaults()
    });
    let steps = ui.show(BulletedList {
        items: vec!["drop the ball".into(), "read the height".into()],
        x: 200.0,
        y: 560.0,
        ..ui.defaults()
    });
    let note = ui.show(MarkupText {
        markup: "<b>no</b> air <span foreground='#ff5500'>resistance</span>".into(),
        x: 200.0,
        y: 760.0,
        ..ui.defaults()
    });
    svgr!(<g>{title}{height}{steps}{note}</g>)
}

let n = DecimalNumber { include_sign: true, unit: " m", ..DecimalNumber::default() };
assert_eq!(n.format(1234.5), "+1,234.50 m");
```

Bold and italic in `MarkupText` draw with the family's bold and italic faces, so put those
font files into the media. Unknown tags and attributes are drawn without their style, and
markup that is not well formed is drawn as plain text; `inspect` warns about both.
`inspect --pops` reports changing digits as pops "like a counter"; that is the number
counting, not a layout jump.

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

## Coordinate systems

The explain layer measures in units of `ui.space`, 8 units tall with `y` up, like Manim
(see the `explain` module docs for `NumberLine`, `Axes`, `NumberPlane` and their graphs).
These build on it:

| Component | Props | What it does |
| --- | --- | --- |
| `Scaling` | `Linear`, `LogBase(base)` on `Axes::x_scaling`, `y_scaling` and `NumberLine::scaling` | a logarithmic axis; its `Range` is in exponents (`Range::new(0.0, 3.0, 1.0)` runs from 1 to 1000), everything else takes values: `c2p(250.0, ..)`, the numbers to write, a graph's `x_range` |
| `NumberLine::unit_interval(theme)` | a `NumberLine` | Manim's `UnitInterval`: 0 to 1 in tenths, 10 units long |
| `ComplexPlane` | `plane: NumberPlane` | the imaginary numbers written `i`, `2i`, `-i`; `n2p((re, im))`, `p2n`, `label(z, decimals)` writes `3+2i` at its point (`format_complex`) |
| `PolarPlane` | `center`, `size`, `radius_max` (4), `radius_step` (1), `azimuth_step` (12 spokes), `azimuth_units` (`PiRadians`, `TauRadians`, `Degrees`, `Gradians`, `None`), `azimuth_offset`, `azimuth_direction` (`Ccw`, `Cw`), `radius_numbers`, line and number paints, `enter`, `exit`, `labels_enter`, `labels_exit` | circles and spokes with the radii and azimuths written; `pr2pt((r, θ))`, `pt2pr` |
| `SampleSpace` | `columns: Vec<SampleColumn>` (each `p`, `label`, `rows: Vec<SampleRow>`), `center`, `width`, `height`, `colors`, `fill_opacity`, `stroke`, `label_style`, `brace_depth`, `enter`, `exit`, `labels_enter`, `labels_exit` | a rectangle split into columns by probability and each column into rows, so a cell's area is the joint probability; column labels under braces; `cells_in(space)` gives the rectangles |

`ComplexValueTracker` is a `(re, im)` that follows a `timeline!` of `(f32, f32)`, read with
`ui.sample` like a `ValueTracker`.

```rust
use fframes_kit::prelude::*;

fn planes<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let plane = ComplexPlane::default();
    let z = (2.0, 1.0 + f64::from(ui.seconds()));
    let (x, y) = plane.n2p(ui, z);
    let label = plane.label(ui, z, 1);
    let plane = ui.show(plane);
    let dot = ui.show(Dot { cx: x, cy: y, ..ui.defaults() });
    let label = ui.show(label);
    let growth = Axes {
        x_range: Range::new(0.0, 10.0, 1.0),
        y_range: Range::new(0.0, 3.0, 1.0),
        y_scaling: Scaling::LogBase(10.0),
        y_numbers: vec![10.0, 100.0, 1000.0],
        ..ui.defaults()
    };
    let curve = growth.plot(ui, |x| 2f64.powf(x));
    let growth = ui.show(growth);
    let curve = ui.show(curve);
    svgr!(<g>{plane}{dot}{label}{growth}{curve}</g>)
}

let polar = PolarPlane::default();
let p = polar.pr2pt_in(Space::FULL_HD, (2.0, 1.0));
let (r, theta) = polar.pt2pr_in(Space::FULL_HD, p);
assert!((r - 2.0).abs() < 1e-9 && (theta - 1.0).abs() < 1e-9);
assert_eq!(fframes_kit::explain::format_complex((0.0, -1.0), 0), "-i");
```

The `PiRadians` azimuths need a font with `π`, `TauRadians` one with `τ` (DM Sans has no
`τ`, and `inspect` reports the fallback).

### Implicit curves and vector fields

The functions are plain `fn`s (closures that capture nothing), so what they trace is cached
by the function's address and the props, once per render and not per frame.

| Component | Props | What it does |
| --- | --- | --- |
| `ImplicitFunction` | `f: fn(x, y) -> f64`, `x_range`, `y_range`, `resolution` (128 cells across), `axes`, `space`, paint, `enter`, `exit`, `emphasis` | the curve where `f(x, y) = 0`, by marching squares; `ImplicitFunction::new(ui, f)` over the frame, `axes.plot_implicit_curve(ui, f)` over the axes' ranges |
| `VectorField` | `f: fn(x, y) -> (f64, f64)`, `x_range`, `y_range` (the grid, as `Range`s), `colors`, `min_magnitude` (0), `max_magnitude` (2), `color` | not drawn itself: the field, its grid and the colour of a vector by its length (`color_for`, `color_at`) |
| `ArrowVectorField` | `field`, `length: fn(f64) -> f64` (`sigmoid_length`, Manim's), `stroke_width`, `tip_length`, `tip_width`, `opacity`, `enter`, `exit`, `emphasis` | an arrow from every grid point along the vector, coloured by its length; zero vectors draw nothing |
| `StreamLines` | `field`, `dt` (0.05), `virtual_time` (3), `max_anchors_per_line` (100), `padding` (3), `n_repeats`, `noise_factor` (half the grid step), `seed`, `stroke_width`, `opacity`, `flow: Option<Flow { time_width, speed }>`, `enter`, `exit` | a line from every grid point, moved by seeded noise, along the field until it leaves the grid by `padding`; each line takes the colour of the field at its middle. `flow` is Manim's `start_animation`: a piece `time_width` of the line long runs along it `speed` times a second, every line from its own start |

```rust
use fframes_kit::prelude::*;

fn fields<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let axes = Axes::default();
    let curve = ui.show(axes.plot_implicit_curve(ui, |x, y| x * y * y - x * x * y - 2.0));
    let axes = ui.show(axes);
    let lines = ui.show(StreamLines {
        field: VectorField {
            f: |x, y| ((x / 2.0).sin() - (y / 2.0).cos(), (x / 2.0).sin()),
            ..ui.defaults()
        },
        flow: Some(Flow::default()),
        ..ui.defaults()
    });
    svgr!(<g>{lines}{axes}{curve}</g>)
}
```

### Graphs

Manim's `Graph` is `explain::Graph`; the prelude's `Graph` is the computer science kit's
searchable graph, so write the module path.

| Component | Props | What it does |
| --- | --- | --- |
| `explain::Graph` | `vertices` (names), `edges: Vec<(usize, usize)>`, `layout: GraphLayout` (spring), `layout_scale` (2 units), `center`, `labels`, `radius` (0.1 units), `vertex_fill`, `vertex_stroke`, `label_style`, `label_fill`, `edge_color`, `edge_width`, `edge_bend` (pixels), `tip_length`, `vertex_styles: Vec<VertexStyle>`, `edge_styles: Vec<EdgeStyle>`, `enter`, `exit`, `labels_enter`, `labels_exit` | vertices placed by the layout, joined by straight or bent edges; with `labels` each vertex holds its name. `Graph::new(ui, &names, &edges)`, `vertex_point(space, v)` gives a vertex's pixel |
| `DiGraph` | `graph: Graph` | the same with a tip on every edge; two edges between the same vertices bend apart |
| `VertexStyle` | `vertex`, `fill`, `stroke`, `radius`, `label` | one vertex's look and text |
| `EdgeStyle` | `edge: (from, to)`, `color`, `width`, `bend`, `label` | one edge's look, and a label beside its middle |

`GraphLayout` (in the prelude) has networkx's layouts: `Circular`, `KamadaKawai`, `Planar`,
`Random { seed }`, `Shell { shells }`, `Spectral`, `Spiral`, `Spring { seed, iterations }`
(the default), `Partite { parts }`, `Tree { root }` and `Function(fn(count, edges, scale))`
for your own positions. `layout::graph_layout(&layout, count, &edges, scale)` returns the
positions alone, centered and scaled to `scale` and pushed `0.4 * scale / √count` apart. The
same arguments give the same positions everywhere, and a graph computes its layout once.
`Planar` computes no embedding: it is `Spring` with seed 0, so crossings can stay.
`Spectral` is `Spring` above 400 vertices.

```rust
use fframes_kit::{explain, prelude::*};

fn graphs<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let tree = ui.show(explain::Graph {
        layout: GraphLayout::Tree { root: 0 },
        labels: true,
        enter: Some(Create::default().into()),
        ..explain::Graph::new(ui, &["a", "b", "c", "d"], &[(0, 1), (0, 2), (2, 3)])
    });
    let flow = ui.show(DiGraph {
        graph: explain::Graph {
            layout: GraphLayout::Circular,
            center: (4.0, 0.0),
            edge_styles: vec![EdgeStyle {
                edge: (0, 1),
                label: Some("start".into()),
                ..EdgeStyle::default()
            }],
            ..explain::Graph::new(ui, &["x", "y", "z"], &[(0, 1), (1, 2), (2, 1)])
        },
    });
    svgr!(<g>{tree}{flow}</g>)
}
```

### Typed matrices and tables

Each wraps the explain layer's `Matrix` or `Table`, which gives the place, gaps, labels,
lines and animations; the wrapper fills the entries.

| Component | Props | What it does |
| --- | --- | --- |
| `DecimalMatrix`, `DecimalTable` | `values: Vec<Vec<f64>>`, `num_decimal_places` (1), `include_sign`, `matrix` or `table` | numbers written like `DecimalNumber` (`1,234.5`); `to_matrix()` / `to_table()` give the text version, for `cell` and `bounds` |
| `IntegerMatrix`, `IntegerTable` | `values: Vec<Vec<i64>>`, `matrix` or `table` | whole numbers |
| `MobjectMatrix`, `MobjectTable` | `entries: Vec<Vec<Mobject>>`, `matrix` or `table` | drawn entries, each in the middle of its cell, a column as wide as its widest entry; a table's labels stay text. `cell(..)` gives a cell's area |
| `Mobject` | `svgr`, `width`, `height` | something drawn around the origin and the box it takes; `Mobject::from_shapes(&shapes)` centers shapes and measures them |
| `MathTable` (feature `latex`) | `formulas: Vec<Vec<&Formula>>`, `size`, `color`, `table` | typeset formulas in a table; typeset them once with a `TexEngine`, like `MathTex` |

```rust
use fframes_kit::prelude::*;

fn tables<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let odds = ui.show(DecimalTable {
        values: vec![vec![0.25, 0.75], vec![0.6, 0.4]],
        num_decimal_places: 2,
        table: Table {
            col_labels: vec!["heads".into(), "tails".into()],
            row_labels: vec!["fair".into(), "bent".into()],
            ..ui.defaults()
        },
        ..ui.defaults()
    });
    let dot = Mobject::from_shapes(&[Shape {
        path: kurbo::Shape::to_path(&kurbo::Circle::new((0.0, 0.0), 20.0), 0.1),
        fill: Some(ui.theme.colors.accent),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity: 1.0,
    }]);
    let shapes = ui.show(MobjectMatrix {
        entries: vec![vec![dot.clone(), dot]],
        matrix: Matrix { x: 1400.0, ..ui.defaults() },
    });
    svgr!(<g>{odds}{shapes}</g>)
}
```

### Cameras

`MovingCamera` is Manim's `MovingCameraScene`: `center` and `width` (units of `ui.space`)
say which part of the canvas fills the picture, and both are `Anim`s, so the frame pans and
zooms. `full_frame_width(space)` is the frame of the whole canvas (14.2 units on 1080p);
`auto_zoom(space, &boxes, margin)` returns the center and width that show some pixel boxes,
like `self.camera.auto_zoom`. `ZoomedInset` is Manim's `ZoomedScene`: the children, plus a
copy of what is inside `frame` magnified into `display`, popping out of the frame over
`run_time` (0 shows it at once). Both are `<g transform>`s; the inset renders its children
twice.

```rust
use fframes_kit::prelude::*;
use fframes_kit::{explain::{auto_zoom, full_frame_width}, kurbo::Rect};

fn zoom_on_a_box<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let full = full_frame_width(ui.space);
    let (center, width) = auto_zoom(ui.space, &[Rect::new(1200.0, 400.0, 1500.0, 700.0)], 0.5)
        .unwrap_or(((0.0, 0.0), full));
    let p = RateFunc::Smooth.apply(motion::progress(ui.seconds(), 0.0, 1.0), 1.0);
    ui.show(MovingCamera {
        center: (center.0 * p, center.1 * p).into(),
        width: motion::lerp(full, width, p).into(),
        children: children(|ui| ui.show(Circle { cx: 1350.0, cy: 550.0, radius: 150.0, ..ui.defaults() })),
    })
}
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
| `Arrow`, `DoubleArrow` | `x1`, `y1`, `x2`, `y2`, `tip_length`, `tip_width`, `tip_start`, `tip_end` | Manim |
| `Vector` | `x`, `y`, `dx`, `dy`, `tip_length`, `tip_width`, `tip_start`, `tip_end` | Manim |
| `CurvedArrow` | `x1`, `y1`, `x2`, `y2`, `angle`, `tip_length`, `tip_width`, `tip_start`, `tip_end`; the shaft an outline, the tip filled | Manim |
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

### Matchers and dashes

Matchers draw around, behind, across or under a box: a `kurbo::Rect` in canvas pixels. Take it
from `shape.bbox()` of any shape, `text_path.bounds(ui.ctx)` (the inked glyphs, descenders
included, as Manim measures text), `label.bounds(ui)` (a `Label`'s cap-height box) or
`view.bounds()` of a formula.

| Shape | Props (defaults) | Source |
| --- | --- | --- |
| `SurroundingRectangle` | `around`, `buff` (`SMALL_BUFF`, 13.5), `corner_radius` (0); highlight outline | Manim |
| `BackgroundRectangle` | `around`, `buff` (0), `corner_radius` (0); the background colour at 75% alpha | Manim |
| `Cross` | `around`, `scale_factor` (1); red, thick | Manim |
| `Underline` | `around`, `buff` (`SMALL_BUFF`) | Manim |
| `DashedVMobject` | `path`, `num_dashes` (15), `dashed_ratio` (0.5), `dash_offset` (0) | Manim |
| `CurvesAsSubmobjects` | `path`, `colors` (a gradient along the curves), `stroke_width` | Manim |

`SMALL_BUFF` is Manim's 0.1 units on a 1080 px tall canvas. `DashedVMobject` spaces its dashes
by arc length like Manim: an open path starts and ends with a dash, a closed one has as many
gaps as dashes, and every dash is its own subpath (`dash_path` builds the path alone).
`CurvesAsSubmobjects` draws each curve of the path as its own part, so `Write` draws them one
after another; `curves(path)` gives the parts.

```rust
use fframes_kit::prelude::*;

fn boxed<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let word = TextPath { text: "Energy", x: 200.0, y: 300.0, ..ui.defaults() };
    let around = word.bounds(ui.ctx);
    let word = ui.show(word);
    let frame = ui.show(SurroundingRectangle {
        around,
        corner_radius: 12.0,
        enter: Some(Create::default().into()),
        ..ui.defaults()
    });
    let circle = Circle { cx: 960.0, cy: 540.0, radius: 120.0, ..ui.defaults() };
    let dashed = ui.show(DashedVMobject { path: circle.path(), num_dashes: 24, ..ui.defaults() });
    svgr!(<g>{word}{frame}{dashed}</g>)
}
```

### Arcs, dots and labels

| Shape | Props (defaults) | Source |
| --- | --- | --- |
| `ArcBetweenPoints` | `x1`, `y1`, `x2`, `y2`, `angle` (a quarter turn), `radius: Option<f32>` | Manim |
| `CurvedDoubleArrow` | as `CurvedArrow`, a tip at both ends | Manim |
| `TangentialArc` | `line1`, `line2` (two points each), `radius` (50), `corner` ((1, 1)) | Manim |
| `CubicBezier` | `start`, `start_handle`, `end_handle`, `end` | Manim |
| `AnnularSector` | `cx`, `cy`, `inner_radius`, `outer_radius`, `start_angle`, `angle`; filled | Manim |
| `ArcPolygon` | `points`, `angle` (an eighth turn), `radius: Option<f32>` | Manim |
| `ArcPolygonFromArcs` | `arcs: Vec<ArcBetweenPoints>` | Manim |
| `AnnotationDot` | `cx`, `cy`, `radius` (10.4); accent fill, light outline | Manim |
| `LabeledDot` | `label`, `cx`, `cy`, `radius: Option<f32>` (fits the label), `label_style`, `label_color` | Manim |
| `LabeledLine`, `LabeledArrow` | `label`, `x1`, `y1`, `x2`, `y2`, `label_position` (0.5), `label_style`, `label_color`, `label_box` (true) | Manim |

A positive `angle` turns counterclockwise on screen, so an arc drawn left to right sags below
its chord. A `radius` takes the shorter arc of that radius instead, on the other side when it is
negative. `TangentialArc` rounds the corner where two lines cross: `corner` picks the side of
each line, `1` along its direction and `-1` against it. `ArcPolygonFromArcs` joins arcs that
do not meet with straight lines.

A `ShapeLabel` is `"text".into()`, drawn in a font from the media, or, with the `latex`
feature, `(&formula).into()`. The label is drawn as outlines with the shape, so `Create` and
`Write` draw both. Without a font file it falls back to a `<text>` element. The labelled lines
put a box of the background colour behind the label, so the line stops at it. Curved arrows
draw their shaft as an outline and their tips filled.

```rust
use fframes_kit::prelude::*;

fn distance<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let a = ui.show(LabeledDot { label: "A".into(), cx: 400.0, cy: 540.0, ..ui.defaults() });
    let d = ui.show(LabeledLine {
        label: "d = 5".into(),
        x1: 440.0,
        y1: 540.0,
        x2: 1100.0,
        y2: 540.0,
        enter: Some(Write::default().into()),
        ..ui.defaults()
    });
    let bend = ui.show(ArcBetweenPoints { x1: 400.0, y1: 600.0, x2: 1100.0, y2: 600.0, ..ui.defaults() });
    svgr!(<g>{a}{d}{bend}</g>)
}
```

### Angles, polygrams and tips

| Shape | Props (defaults) | Source |
| --- | --- | --- |
| `Angle` | `line1`, `line2` (two points each), `radius` (54), `quadrant` ((1, 1)), `other_angle`, `dot`, `dot_radius` (5), `dot_distance` (0.55), `label`, `label_buff` (`3 * SMALL_BUFF`) | Manim |
| `RightAngle` | as `Angle`, drawn as a square corner | Manim |
| `Elbow` | `x`, `y`, `width` (27), `angle` | Manim |
| `TangentLine` | `path`, `alpha` (0.5, by arc length), `length` (135) | Manim |
| `Polygram` | `vertex_groups: Vec<Vec<(f32, f32)>>`, each group a closed polygon | Manim |
| `RegularPolygram` | `cx`, `cy`, `radius` (100), `num_vertices` (5), `density` (2), `start_angle: Option<f32>` | Manim |
| `ConvexHull` | `points` | Manim |
| `Cutout` | `outer`, `holes` (paths); filled | Manim |
| `ScreenRectangle`, `FullScreenRectangle` | `cx`, `cy` (the canvas centre), `height` (540, 1080), `aspect_ratio` (16:9) | Manim |
| `LabeledPolygram` | `vertex_groups`, `label`, `precision` (1 px), `label_style`, `label_color` | Manim |

An angle runs counterclockwise on screen from `line1` to `line2` around the point where they
cross. `quadrant` picks the half of each line the mark sits on, `1` along its direction and
`-1` against it, and `other_angle` marks the rest of the turn instead. `value()` gives the
marked angle in radians. `TangentLine` touches any path at `alpha` of its length;
`kits::math::Tangent` is the tangent of a function graph on `Axes`. `RegularPolygram` with
`density` `d` joins every `d`-th vertex, `{6/2}` gives two triangles. `Cutout` reverses the
holes, so they stay empty under either fill rule. `LabeledPolygram` puts its label at the
inside point furthest from every edge (`polylabel`), Manim's pole of inaccessibility.

Every arrow (`Arrow`, `Vector`, `DoubleArrow`, `CurvedArrow`, `CurvedDoubleArrow`,
`LabeledArrow`) has `tip_start` and `tip_end`, an `Option<ArrowTip>`: `Triangle`,
`TriangleFilled` (the default), `Circle`, `CircleFilled`, `Square`, `SquareFilled` and
`Stealth`, Manim's `ArrowTriangleTip` to `StealthTip`. The tip ends at the arrow's point, `tip_length` along the shaft and `tip_width`
across it. Outline tips are drawn unfilled on their own, and the shaft runs into the notch
of a stealth tip.

```rust
use fframes_kit::prelude::*;

fn corner<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let (a, b) = (((400.0, 600.0), (800.0, 600.0)), ((400.0, 600.0), (700.0, 380.0)));
    // a Greek letter needs a font that has it, or a formula with the `latex` feature
    let theta = ui.show(Angle { line1: a, line2: b, label: "a".into(), ..ui.defaults() });
    let arrow = ui.show(Arrow {
        x1: 900.0,
        y1: 600.0,
        x2: 1300.0,
        y2: 600.0,
        tip_start: Some(ArrowTip::Circle),
        tip_end: Some(ArrowTip::Stealth),
        enter: Some(GrowArrow::default().into()),
        ..ui.defaults()
    });
    let star = ui.show(RegularPolygram { cx: 1600.0, cy: 600.0, num_vertices: 7, density: 3, ..ui.defaults() });
    svgr!(<g>{theta}{arrow}{star}</g>)
}
```

### Braces, boolean operations, SVG and images

| Shape | Props (defaults) | Source |
| --- | --- | --- |
| `ArcBrace` | `cx`, `cy`, `radius` (135), `start_angle` (-1), `angle` (2), `depth` (24), `inside` | Manim |
| `Union`, `Intersection` | `paths: Vec<BezPath>`; filled | Manim |
| `Difference`, `Exclusion` | `subject`, `clip`; filled | Manim |
| `SvgShape` | `parts` (from `SvgShape::parse`), `cx`, `cy` (the canvas centre), `height` (270), `fill`, `stroke` | Manim `SVGMobject` |
| `SvgPath` | `path` (from `paths::parse_path`) | Manim `VMobjectFromSVGPath` |

`Brace` is Manim's `BraceBetweenPoints`: its tip points to the right of the direction from
`(x1, y1)` to `(x2, y2)`, below for a brace drawn left to right; a negative `depth` or
swapped points put it on the other side. `BraceLabel` is `BraceLabel` and `BraceText`.
`ArcBrace` bends a brace round an arc, its tip outward, or toward the center when `inside`;
`tip()` is where its label goes.

The boolean operations read their paths with the nonzero fill rule, as SVG and Manim do, so
a self-crossing star keeps its middle. Curves are flattened to `BOOLEAN_TOLERANCE` (0.25 px)
and the result is straight lines, holes running against their outline. Combining detailed
paths takes time: when the inputs do not change, compute the path once with
`boolean(a, b, BooleanOp::..)` and keep it in `self`.

`SvgShape::parse` reads an SVG file once, keep the result in `self`; a clone shares the
parts. Every path is a part with the `id` of the path or of the nearest group around it, its
fill and its outline, and keeps the file's paint unless `fill` or `stroke` replace it.
`fill_part(key, color)` and `stroke_part(key, color)` paint the parts with an `id` or the
one at an index, `only(|index, part| ..)` keeps some, and `part(key)` reads one. A gradient
draws in its first stop's colour and a pattern black. Text and images in the file are not
drawn: convert text to paths before export. `inspect` warns about all four.

Manim's `ImageMobject` is an `<image>` of a picture from the media; `PhotoFrame` adds a
border and rounded corners, and `KenBurns` a slow zoom.

```rust
use fframes_kit::{paths, prelude::*};

fn drawings<'a>(ui: &mut Ui<'a, '_, '_>, logo: &SvgShape) -> Svgr<'a> {
    let accent = ui.theme.colors.accent;
    let logo = ui.show(SvgShape {
        cx: 400.0,
        height: Some(300.0),
        enter: Some(Write::default().into()),
        ..logo.clone().fill_part("roof", accent)
    });
    let a = Circle { cx: 900.0, cy: 540.0, radius: 120.0, ..ui.defaults() }.path();
    let b = Circle { cx: 1000.0, cy: 540.0, radius: 120.0, ..ui.defaults() }.path();
    let lens = ui.show(Intersection { paths: vec![a, b], ..ui.defaults() });
    let wave = paths::parse_path("M1200 540 C1300 440 1400 640 1500 540").unwrap_or_default();
    let wave = ui.show(SvgPath { path: wave, ..ui.defaults() });
    let Some(photo) = ui.ctx.get_image("photo.png") else {
        return svgr!(<g>{logo}{lens}{wave}</g>);
    };
    let photo = svgr!(<image href={photo.href()} x="1560" y="440" width="320" height="200" />);
    svgr!(<g>{logo}{lens}{wave}{photo}</g>)
}
```

## Geometry animations

Animations are plain data. Put them into a shape's `enter`, `exit` or `emphasis` with
`.into()`. Each has Manim's fields: `delay`, `run_time`, `rate_func`, plus its own.

| Animation | Slot | What it does |
| --- | --- | --- |
| `FadeIn`, `FadeOut` | enter, exit | fade with an optional `shift` |
| `Create`, `Uncreate` | enter, exit | draws the outline from its start (and fades the fill in), or back |
| `DrawBorderThenFill` | enter | draws the outline, then fills (`run_time` 2) |
| `Write` | enter | `DrawBorderThenFill` per glyph of a `TextPath`, `lag_ratio` |
| `Unwrite { reverse }` | exit | `Write` backwards; with `reverse` (the default) the first glyph goes first |
| `ShowPartial { start, end }` | enter | the drawn part of the outline moves from one window to another |
| `ShowIncreasingSubsets`, `ShowSubmobjectsOneByOne` | enter | the parts appear one more at a time, or one at a time with the last staying |
| `SpiralIn { scale_factor, fade_in_fraction }` | enter | the parts fly in on a spiral around the center of the whole |
| `AddTextLetterByLetter`, `RemoveTextLetterByLetter` | enter, exit | glyphs appear, or vanish from the last, `time_per_char` each (or in `run_time`) |
| `GrowFromCenter`, `GrowFromPoint { point }`, `GrowArrow` | enter | grows from nothing about the center, a point, or the path's start |
| `GrowFromEdge { edge: Anchor }` | enter | grows from a point of the bounding box (`Anchor::Bottom` is Manim's `DOWN`) |
| `SpinInFromNothing { angle }` | enter | grows while turning into place |
| `Indicate { scale_factor, color }` | emphasis | scales up and tints, then back |
| `Circumscribe { color, buff }` | emphasis | a rectangle runs once around the shape |
| `Flash { point, num_lines, flash_radius, line_length }` | emphasis | lines shoot out once |
| `Wiggle { scale_value, rotation_angle, n_wiggles }` | emphasis | shakes the shape |
| `Blink { blinks, time_on, time_off, hide_at_end }` | emphasis | on, then off, `blinks` times (a step: no `rate_func`) |
| `FocusOn { point, color, opacity }` | emphasis | a frame-sized disc shrinks onto the point and vanishes |
| `Rotate { angle, about_point }` | emphasis | turns by `angle` (a half turn; positive is counterclockwise, as in Manim) and stays |
| `Rotating { angle, about_point, forever }` | emphasis | turns at an even speed, `angle` per `run_time` (a full turn in 5 s), once or `forever` |
| `FadeToColor { color }` | emphasis | fill and outline turn into `color` and stay |
| `ScaleInPlace { scale_factor }` | emphasis | scales about the center (twice) and stays |
| `ShrinkToCenter` | exit | shrinks to nothing about the center |

Enter and emphasis animations count from the start of the enclosing container plus `delay`;
exit animations end `delay` seconds before its end. They change only the transform, opacity
and drawn part of the outline, never the path. The parts are the glyphs of a `TextPath`, or
the shapes given to `animate::draw_animated`; a single shape is one part. For a `<text>`
line, `AddTextLetterByLetter` is a `Typewriter { cursor: None, .. }`, `AddTextWordByWord` a
`TextReveal { by: RevealBy::Word, run_time: 0.0, shift: (0.0, 0.0), .. }`, and Manim's
`TypeWithCursor` and `UntypeWithCursor` are a `Typewriter`, with `erase: Some(seconds)`
for the second.

| Component | Props | What it does |
| --- | --- | --- |
| `Morph` (Manim `Transform`) | `from: Shape`, `to: Shape`, `delay`, `run_time`, `rate_func` | morphs path, paint and width; `Morph::new(from, to)` |
| `ReplacementTransform` | the same | `Morph` that draws `to` itself once done |
| `MoveAlongPath` | `path: BezPath`, `delay`, `run_time`, `rate_func` | `Enter`: moves content from the path's start to its end |
| `ShowPassingFlash` | `shape`, `time_width`, timing | a piece of the outline runs along it once; `::new(shape)` |
| `ShowPassingFlashWithThinningStrokeWidth` | `shape`, `n_segments`, `time_width`, timing | the same with a wide head and a thinning tail |
| `Broadcast` | `point`, `radius`, `color`, `n_mobs`, `lag_ratio`, opacities, timing | rings grow out of a point one after another and fade |
| `AnimatedBoundary` | `shape`, `colors`, `max_stroke_width`, `cycle_rate`, `back_and_forth` | the outline redraws itself in one colour after another, forever; `::new(shape, colors)` |
| `ApplyWave` | `shape`, `direction`, `amplitude` (px), `ripples`, `time_width`, timing | a wave crosses the shape left to right and bends it; `::new(shape)` |
| `Homotopy`, `SmoothedVectorizedHomotopy` | `shape`, `homotopy: fn(x, y, t) -> (x, y)`, timing | moves every point while `t` runs from 0 to 1, in `ui.space` units; the smoothed one redraws the handles |
| `ComplexHomotopy` | `shape`, `homotopy: fn((re, im), t) -> (re, im)`, timing | the same on complex numbers |
| `PhaseFlow` | `shape`, `function: fn(x, y) -> (dx, dy)`, `virtual_time`, timing | the points flow along a field for `virtual_time` |
| `TracedPath` | `point: &dyn Fn(f32) -> (f32, f32)`, `stroke`, `stroke_width`, `dissipating_time`, `delay` | the line a moving point has drawn so far |
| `TransformFromCopy` | `from`, `to`, timing | morphs a copy while `from` stays |
| `ClockwiseTransform`, `CounterclockwiseTransform` | `from`, `to`, `path_arc`, timing | morphs with every point on an arc (a half turn) |
| `CyclicReplace`, `Swap` | `shapes` (`first`, `second`), `path_arc`, timing | each shape moves on an arc (a quarter turn) to the next one's place |
| `FadeTransform`, `FadeTransformPieces` | `from`, `to` (lists for the pieces), `stretch`, timing | `from` fades out moving onto `to`'s box while `to` fades in from `from`'s; pieces pair by index and the rest fade in place (Manim splits pieces until the counts match) |
| `TransformMatchingShapes` | `from: Vec<Shape>`, `to: Vec<Shape>`, timing | pieces with the same `shape_key` (form, whatever the place and size) morph, the others fade; pieces of one key pair in order, where Manim morphs them as a group |
| `TransformAnimations` | `from`, `from_anim: EnterAnim`, `to`, `to_anim`, timing | morphs while the look goes from the first animation's to the second's |
| `ApplyMatrix` | `shape`, `matrix: [[f64; 2]; 2]`, `about_point`, timing (3 s) | maps every point by the matrix in `ui.space` units, `y` up |
| `ApplyPointwiseFunction`, `ApplyPointwiseFunctionToCenter` | `shape`, `function: fn(x, y) -> (x, y)`, timing (3 s) | moves every point, or the whole shape by its center |
| `ApplyComplexFunction` | `shape`, `function: fn((re, im)) -> (re, im)`, timing | the same on complex numbers, on arcs as wide as the angle of `function(1)` |
| `ApplyFunction` | `shape`, `function: fn(Shape) -> Shape`, timing | `Morph` into the shape the function returns |
| `LaggedStart` | `lag_ratio`, `run_time`, `children` | child `i` starts `i * lag_ratio * run_time` seconds in |
| `AnimationGroup` | `children` | every child at once |
| `Succession` | `run_time`, `children` | each child `run_time` after the one before |
| `LaggedStart::map` (Manim `LaggedStartMap`) | `items`, `f(ui, &item)` | a `LaggedStart` with one child per item, lag ratio 0.05 |
| `ChangeSpeed` | `run_time`, `speed: Vec<(fraction, speed)>`, `children` | the children play slower or faster over `run_time` and still start and end on time |
| `ChangeDecimalToValue` | `number: DecimalNumber`, `target`, timing | counts the number from its tracker's value to `target` |
| `ChangingDecimal` | `number`, `number_update_func: fn(alpha) -> value`, timing | shows the function of the progress |

The homotopies cut the path into about `HOMOTOPY_SEGMENTS` (256) pieces and map every point
each frame; `PhaseFlow` takes up to `PHASE_FLOW_STEPS` (120) Runge-Kutta steps per point.
`TracedPath` calls `point` again for every frame of the trail, at most `MAX_TRACE_SAMPLES`
(240) times, on a fixed grid of times so the drawn part does not shimmer; keep `point`
cheap, or give it a `dissipating_time`.

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

### Manim's imperative animations

Manim changes a mobject in place and plays the change. In the kit a frame is a function of
its time, so there is no mobject to change: these names become the components above, or a
value computed in `render` from `ui.seconds()`.

| Manim | fframes-kit |
| --- | --- |
| `self.wait(1)` | nothing to call: start what follows 1 second later (`Sequence { from }`) |
| `ApplyMethod(m.shift, RIGHT)`, `m.animate.shift(RIGHT)` | `Morph::new(shape.clone(), moved)`, `moved` the shape with the change made |
| `m.generate_target()`, `MoveToTarget(m)` | the same: the target is a second `Shape` |
| `m.save_state()`, `Restore(m)` | keep a clone of the shape, `Morph::new(changed, saved)` |
| `UpdateFromFunc(m, f)`, updaters | compute the prop from `ui.seconds()` in `render`, which runs every frame |
| `UpdateFromAlphaFunc(m, f)` | the same with `alpha = rate.apply(motion::progress(t, start, end), run_time)` |
| `MaintainPositionRelativeTo(m, leader)` | place `m` at the leader's position, computed from the same time, plus the offset |

```rust
use fframes_kit::prelude::*;

/// `ApplyMethod(square.shift, 2 * RIGHT)`, then `Restore(square)`: morph to a moved copy and
/// back to the saved one. `self.wait(0.5)` is the half second between them.
fn shift_and_restore<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let saved = Square { cx: 660.0, cy: 540.0, side_length: 200.0, ..ui.defaults() }.shape();
    let shifted = Shape { path: paths::translate_path(&saved.path, 270.0, 0.0), ..saved.clone() };
    let (a, b) = (shifted.clone(), saved.clone());
    let restore = children(move |ui| ui.show(Morph::new(a.clone(), b.clone())));
    if ui.seconds() < 1.5 {
        ui.show(Morph::new(saved, shifted))
    } else {
        ui.show(Sequence { from: 1.5, duration: None, children: restore })
    }
}

/// `UpdateFromFunc`, `UpdateFromAlphaFunc` and `MaintainPositionRelativeTo`: a dot crosses in 2 seconds from
/// second 1, and a second dot follows it 80 pixels above.
fn follow<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let alpha = RateFunc::Smooth.apply(motion::progress(ui.seconds(), 1.0, 3.0), 2.0);
    let x = motion::lerp(360.0, 1560.0, alpha);
    let leader = ui.show(Dot { cx: x, cy: 540.0, ..ui.defaults() });
    let follower = ui.show(Dot { cx: x, cy: 460.0, ..ui.defaults() });
    svgr!(<g>{leader}{follower}</g>)
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
| `Flip` | `direction` (`FromLeft`), `size` | the exiting sequence turns half a turn and the entering one is on its back (`flip()`); drawn flat as a scale, so `direction` only picks the axis and there is no perspective |
| `ClockWipe` | `size` | a hand sweeps clockwise from twelve o'clock and uncovers the entering sequence (`clockWipe()`) |
| `Iris` | `size` | the entering sequence shows through a circle that opens from the middle (`iris()`) |
| `CircleWipe` | `center` (0.5, 0.5, a share of `size`), `color` (`colors.title`), `size` | a disc of `color` grows from `center` over the exiting sequence, then the entering one opens from the same point |
| `SlantWipe` | `color` (`colors.background`), `edge` (`colors.highlight`), `back` (`colors.accent`), `skew` (320), `size` | a slanted panel with a stripe in front and behind sweeps across, covers everything at the middle and leaves the entering sequence behind it |
| `NoTransition` | | a hard cut at the middle of the overlap (`none()`); the series keeps the overlap, so the sequences can animate it themselves with `ui.transition_progress()` |
| `PushCut` | `cut_progress` (5/11), `outgoing_scale` (1.04), `incoming_start_scale` (1.04), `incoming_end_scale` (1.07), `origin` (0.5, 0.5), `flash_color` (#f5f2ed), `flash_opacity` (0.2), `flash_frames` (2), `size` | a hard editorial cut with a punch-in on both sides and a short flash at the edit (`pushCut()`); use a linear timing |
| `ZoomInOut` | `scale` (1.5), `size` | the exiting sequence zooms in and cross-fades into the entering one, which zooms back out (`zoomInOut()`); `scale` is the kit's, Remotion has no options |
| `DreamyZoom` | `rotation` (6°), `scale` (1.2), `flash_color` (white), `blur` (12), `size` | both sides zoom, turn and blur through a flash at the middle (`dreamyZoom()`) |
| `LinearBlur` | `intensity` (0.1, a share of the width), `axes` (1, 1), `size` | the entering sequence fades in while both blur, most at the middle (`linearBlur()`); `axes` weighs the blur along x and y, a slanted direction is not available |
| `BlurSlide` | `direction` (`FromLeft`), `blur` (0.5, a share of the size), `size` | both sides pan like a fast camera move with a motion blur along it (`blurSlide()`) |
| `CrossZoom` | `strength` (0.4), `size` | both sides zoom about a centre that moves across and blur, cross-fading at the middle (`crossZoom()`); an approximation: the blur is even, not along rays |
| `Dissolve` | `line_width` (0.1), `spread_color` (#ff0000), `hot_color` (#e6e633), `pow` (5), `intensity` (1), `seed`, `frequency` (0.004), `size` | the exiting sequence burns away with a glowing edge (`dissolve()`); the burn follows seeded `feTurbulence` noise, not the luminance of the pixels as in Remotion |
| `FilmBurn` | `seed` (2.31), `blur` (6), `size` | a burning film overlay flares over a blurred cross-fade (`filmBurn()`); an approximation of the GL original from a radial gradient and noise |
| `SwapCards` | `reflection` (0.4), `perspective` (0.2), `depth` (3), `floor` (`colors.background`), `size` | the two sequences trade places like cards on a floor, with reflections (`swap()`); a 2D approximation of the 3D original. Manim's `Swap` is another thing |
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

The blurring presentations (`DreamyZoom`, `LinearBlur`, `BlurSlide`, `CrossZoom`,
`FilmBurn`) run an `feGaussianBlur` over each side during the transition, up to about 60 ms
more a frame than a fade on the CPU backend at 1080p. They blur only what is on the canvas (`size`), so the
CPU and Skia backends draw the same, and they keep the alpha of the blurred side, so the
canvas does not darken at its edges. They expect sequences that cover the canvas, as a
`Fill` does: a translucent sequence turns opaque where the blur leaves it a quarter opaque.

Not yet available, because they need pixel shaders: `crosswarp`, `ripple`, `bookFlip`,
`cube` and `custom-html-in-canvas` (see the
[usage report](../../plans/reports/research-261024-1748-manim-remotion-usage.md)).

Inside a sequence, `ui.transition_progress()` is Remotion's `useTransitionProgress()`:
`entering` runs from 0 to 1 while the sequence comes in and is 1 otherwise, `exiting` runs
from 0 to 1 while it goes away and is 0 otherwise, and `in_series` is false outside a
series (1 and 0 then). `ui.transition_seconds(side)` is the length of a running transition.

The kit cannot change `fn audio`, so it gives the times to fade a sequence's sound with
instead: `audio_fades(&lengths, fps)` (or `series.audio_fades(fps)`) returns, per sequence,
where it starts and ends and how long its fades are, in seconds from the series start.
`fade.apply(track)` sets them on an `AudioTrack`.

```rust
use fframes::{AudioMap, AudioTimestamp::Second, AudioTrack};
use fframes_kit::prelude::*;

const SERIES_START: f32 = 2.0;

fn audio() -> AudioMap<'static> {
    let fades = audio_fades(
        &[
            SeriesLength::Sequence(4.0),
            SeriesLength::Transition(Timing::default()),
            SeriesLength::Sequence(4.0),
        ],
        30,
    );
    let track = |file, fade: &AudioFade| {
        let range = Second(SERIES_START + fade.start)..Second(SERIES_START + fade.end);
        fade.apply(AudioTrack::new(file, range))
    };
    AudioMap::from([track("one.wav", &fades[0]), track("two.wav", &fades[1])])
}

/// A bar that fills while the sequence comes in.
fn bar<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let width = 1200.0 * ui.transition_progress().entering;
    svgr!(<rect x="96" y="700" width={width.max(1.0)} height="24" fill="#fff" />)
}
# let _ = (audio, bar);
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

## Effects

| Component | Props | What it does |
| --- | --- | --- |
| `Trail` | `samples` (6), `spacing` (0.05 s), `opacity` (0.5), `children` | the children at earlier times behind themselves, each copy fainter (Remotion's `<Trail>`) |
| `CameraMotionBlur` | `shutter_angle` (180°), `samples` (10), `children` | the children averaged over the time the shutter is open before the frame (`<CameraMotionBlur>`) |
| `LightLeak` | `seed`, `hue_shift` (degrees), `length` (`None`: the container's), `opacity` (0.85), `size`, `key` | warm blobs of light flare up and fade out over `length` seconds, in screen blending (`<LightLeak>`) |
| `Blur` | `radius: Anim<f32>` (8), `key`, `children` | the children blurred by a Gaussian; animate `radius` for a focus pull |
| `Noise` | `seed`, `opacity` (0.12), `frequency` (0.8), `octaves` (2), `animated` (true), `size`, `key` | film grain from `feTurbulence`; `animated` moves the seed every frame |
| `ShaderNoise` | `seed`, `opacity`, `scale` (2 px), `animated`, `size` | the grain as an SkSL shader on the GPU (feature `skia`); the CPU backend draws nothing and the editor a placeholder |

`Trail` and `CameraMotionBlur` render their children once per copy, at most
`effects::MAX_SAMPLES` (16) times a frame. The bench card behind an eight-copy `Trail`
(`BenchTrail` in the gallery) renders its five seconds in about 1.8 times the time of
`BenchKit` on the CPU backend. The copies run at a finer frame rate than the video's, so the
children should count in local seconds (`ui.seconds()`, `ui.frames()`);
`frame.global_index` keeps the video frame. Before the local start there is nothing to repeat.
A clip path, gradient or filter inside the children has the same id in every copy, so all
copies draw one copy's definition: fine while it stays the same, wrong for one that animates.
`key` names the SVG ids when two of the same effect share a frame. The glow and the drop
shadow are the `Glow` and `DropShadow` filter definitions above.

A ball with a trail over a light leak, and a title that comes into focus:

```rust
use fframes_kit::prelude::*;

fn ball<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let x = 200.0 + 600.0 * motion::clamp01(ui.seconds() / 2.0);
    svgr!(<circle cx={x} cy="540" r="40" fill={ui.theme.colors.accent} />)
}

fn shot<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let leak = ui.show(LightLeak { seed: 3, length: Some(3.0), ..ui.defaults() });
    let trail = ui.show(Trail { samples: 8, children: children(ball), ..ui.defaults() });
    let focus = motion::clamp01(ui.seconds() - 0.5);
    let title = ui.show(Blur {
        radius: Anim::Fixed(16.0 * (1.0 - focus)),
        key: Some("title"),
        children: children(|ui| ui.show(HudLabel { text: "Launch".into(), x: 200.0, y: 300.0, ..ui.defaults() })),
    });
    svgr!(<g>{trail}{leak}{title}</g>)
}

let blur = CameraMotionBlur { samples: 4, ..CameraMotionBlur::themed(&Theme::default()) };
assert_eq!(blur.offsets(30).len(), 4);
```
### Colour effects

Each colour effect is a wrapper, `Effect { children, ..ui.defaults() }`, drawn through one SVG
filter in sRGB, so the CPU and Skia backends agree. The main amount is an `Anim<f32>`; to
drive it from time, compute it from `ui.seconds()` and pass `Anim::Fixed`. `key` names the
filter when two effects of one kind share a frame.

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `Brightness` | `amount` (1.3) | CSS `brightness()`: the colours times `amount` |
| `Contrast` | `amount` (1.4) | CSS `contrast()` about mid grey |
| `Saturation` | `amount` (1.6) | SVG `saturate`: 0 grey, 1 unchanged |
| `HueRotate` | `degrees` (90) | the hues turned round the colour wheel (Remotion's hue) |
| `Grayscale` | `amount` (1) | CSS `grayscale()` |
| `Invert` | `amount` (1) | CSS `invert()` |
| `Tint` | `color` (accent), `amount` (0.8) | the picture in shades of `color` |
| `Duotone` | `dark` (background), `light` (accent), `amount` (1) | dark tones `dark`, light tones `light` |
| `Exposure` | `stops` (0.5) | every stop doubles the light |
| `WhiteBalance` | `temperature` (0.5, -1 cold to 1 warm), `tint` (0, -1 green to 1 magenta) | gains of up to a fifth on red, green and blue |
| `Vibrance` | `amount` (0.8) | half as strong as `Saturation`; approximate, a matrix cannot spare colours that are already saturated |
| `ColorCorrection` | `brightness` (1), `contrast` (1.1), `saturation` (1.2), `hue` (0) | the four in that order, one matrix |
| `ColorStack` | `matrices: Vec<ColorMatrix>` | several matrices composed into one `feColorMatrix`: a stack costs one filter |
| `Levels` | `input` (0.08, 0.92), `gamma` (1.2), `output` (0, 1) | Photoshop's levels on every channel |
| `Curves` | `points` (an S curve), `red`, `green`, `blue` (`None`) | a smooth monotone curve through the points, then per channel |
| `Lut` | `red`, `green`, `blue` (lists of outputs) | a 1D table per channel; approximate, a `.cube` LUT maps colours in 3D |
| `ShadowsHighlights` | `shadows` (0.6), `highlights` (0.3) | lifts dark tones, pulls light ones down; approximate, a curve on the pixel's own value |
| `ThermalVision` | `palette` (`THERMAL_PALETTE`) | the luminance mapped onto the palette, cold to hot |
| `ColorKey` | `color` (green), `tolerance` (0.4), `softness` (0.2) | a chroma key; linear in the colour, so it suits saturated screens |

`ColorMatrix` holds the matrix maths (`brightness`, `contrast`, `exposure`, `saturation`,
`grayscale`, `hue_rotate`, `invert`, `tint`, `duotone`, `white_balance`, `vibrance`, `mix`,
`then`, `apply`). `a.then(b)` is one matrix that does `a`, then `b`. Unlike two nested
effects, it does not clamp the colours in between. The table effects (`Levels`, `Curves`,
`Lut`, `ShadowsHighlights`, `ThermalVision`, `ColorKey`) do not reduce to a matrix: nest them
around or inside a stack, one filter each.

```rust
use fframes_kit::prelude::*;

fn photo<'a>(_ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    svgr!(<rect x="0" y="0" width="1920" height="1080" fill="#f08a4b" />)
}

// curves inside a two-matrix stack that warms up over two seconds: two filters
fn graded<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let warmth = motion::clamp01(ui.seconds() / 2.0);
    ui.show(ColorStack {
        matrices: vec![ColorMatrix::exposure(0.3), ColorMatrix::white_balance(warmth, 0.0)],
        children: children(|ui| ui.show(Curves { children: children(photo), ..ui.defaults() })),
        ..ui.defaults()
    })
}

let red = ColorMatrix::grayscale(1.0).apply([1.0, 0.0, 0.0, 1.0]);
assert!((red[0] - 0.213).abs() < 1e-6);
```

### Edges and shape

These work on the alpha edges of their children: text, logos and shapes on a transparent
background. Lengths are in the children's user units; props follow Remotion's names and
defaults.

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `Outline` | `width` (8), `color` (white), `opacity` (1), `outline_only` (false) | a solid outline from `feMorphology`; the region grows a tenth of the box, wider outlines are cut |
| `Emboss` | `amount` (0.7), `size` (26), `line_width` (7), `depth` (0.75), `angle` (0), `light_angle` (135, from the top left), `offset` (0), `area` | Remotion's raised dash relief: a lit and shaded dash pattern masked by the children's alpha; renders the children twice |
| `BevelAlpha` | `edge_thickness` (2), `light_angle` (-60, clockwise from above), `light_color` (white), `light_intensity` (0.4) | highlights and shadows along the alpha edges, from the slope of the blurred alpha |
| `LongShadows` | `length` (240), `angle` (45, clockwise from the right), `color` (`#777777`), `opacity` (1), `area` | the silhouette as a solid directional shadow, clipped to `area` |
| `RoughenEdges` | `amount` (1), `border` (26.5), `scale` (0.07), `seed` (231.2) | a torn-paper edge: blurred alpha shifted by noise and cut at a threshold |
| `Shrinkwrap` | `amount` (1), `highlight_intensity` (0.75), `wrinkle_density` (0.42), `edge_tension` (0.45), `seed` | plastic wrap glints on `feTurbulence` wrinkles and a highlight along the edges |

`LongShadows` doubles its copies: at most eight merges give 256 copies, one unit apart for a
shadow up to 255 units long. Overlapping copies of a semi-transparent silhouette add up.
`BevelAlpha` and `Shrinkwrap` measure slopes with `feConvolveMatrix` over neighbouring
device pixels, so a draft render at a smaller scale shows a stronger bevel. The SVG lighting
filters would scale correctly, but at half size the CPU and Skia backends draw them about
0.6% apart. Remotion's `shrinkwrap()` also displaces the picture along the wrinkles. That
needs `feDisplacementMap`, so it waits with the other displacement effects.

```rust
use fframes_kit::prelude::*;

fn logo<'a>(_ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    svgr!(<circle cx="960" cy="540" r="200" fill="#e63946" />)
}

fn badge<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let grow = motion::clamp01(ui.seconds());
    ui.show(LongShadows {
        length: Anim::Fixed(240.0 * grow),
        children: children(|ui| ui.show(Outline { width: Anim::Fixed(6.0), children: children(logo), ..ui.defaults() })),
        ..ui.defaults()
    })
}
```

### Noise

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `WhiteNoise` | `amount` (1), `seed` (0), `grain` (2), `area` | grey static over the opaque parts; derive `seed` from the frame to make it move |
| `FractalNoise` | `fractal_type` (`Basic`, or `Turbulent`), `scale` (100), `complexity` (6), `evolution` (0°), `cycle_evolution`, `cycle` (1), `contrast` (1), `brightness` (0), `offset_x`, `offset_y`, `seed`, `invert`, `amount` (1), `area` | layered grey noise for clouds and smoke, mixed over the opaque parts |

Both draw `feTurbulence` on a rectangle over `area`, masked by the children's alpha, so the
children render twice. `FractalNoise::evolution` cross-fades to the next seed on every full
turn; with `cycle_evolution` the seeds repeat after `cycle` turns. Remotion's other fractal
types, `noiseType`, `subInfluence` and `subScaling` have no `feTurbulence` equivalent.

Noise detail near the pixel size samples differently on the CPU and Skia backends. A
white noise of one-unit grain in a fifth-size thumbnail differed in 1.2% of the pixels; at
about five output pixels per speck it differed in 0.07%. Keep `grain` and the finest
`FractalNoise` layer (`scale / 2^(complexity - 1)`) at a few output pixels where the backends
must agree.

`motion::noise_2d`, `noise_3d` and `noise_4d` are simplex noise for driving motion
(Remotion's `noise2D`, `noise3D`, `noise4D`). They return -1 to 1, the same for the same seed
and coordinates. The seed is a number or a string.

```rust
use fframes_kit::prelude::*;

fn bobbing<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let y = 540.0 + 80.0 * motion::noise_2d("buoy", 0.0, ui.seconds() * 0.6);
    svgr!(<circle cx="960" cy={y} r="40" fill="#e63946" />)
}

fn storm<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let turn = 90.0 * ui.seconds();
    ui.show(FractalNoise {
        evolution: Anim::Fixed(turn),
        amount: Anim::Fixed(0.6),
        children: children(bobbing),
        ..ui.defaults()
    })
}
```

The displacement effects (noise displacement, turbulent displace, wave, waves, zigzag) are
not yet available. They need `feDisplacementMap`, which the CPU backend draws wrongly: the
`svgr` crate applies its `scale` twice and reads the map premultiplied (resvg issue 1160).
They come back when a fixed `svgr` arrives with an upstream update.

### Pattern overlays

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `Scanlines` | `amount` (0.15), `spacing` (4), `thickness` (1), `offset`, `premultiply`, `area` | CRT lines: the lines light the picture and the gaps darken it; animate `offset` to roll them |
| `DotGrid` | `dot_size` (16), `grid_size` (20), `invert` | the children seen through round holes, or with the holes cut out |
| `Checkerboard` | `colors` (two pale blues), `cell_size` (80), `gap` (0), `angle`, `offset_x`, `offset_y`, `mask_to_source_alpha` | cells over the children |
| `Gridlines` | `grid_size` (64), `line_width` (2), `line_color` (white), `background_color` (transparent), `rotation`, `offset_x`, `offset_y`, `mask_to_source_alpha` | a square grid over the children |
| `Lines` | `colors`, `direction` (`Horizontal` or `Vertical`), `thickness` (40), `gap` (0), `angle`, `offset`, `mask_to_source_alpha` | stripes over the children |
| `Rings` | `colors`, `center` (0.5, 0.5), `thickness` (40), `gap` (0), `offset`, `mask_to_source_alpha` | concentric rings; animate `offset` to make them expand |
| `LinearGradientTint` | `start` (0, 0.5), `end` (1, 0.5), `start_color` (black), `end_color` (white), `amount` (0.5) | the colours pulled towards a gradient, the alpha kept |
| `LinearGradient` | `start`, `end`, `start_color`, `end_color` | a gradient filling the area; it replaces the layer, so it has no children |
| `Vignette` | `amount` (0.5), `radius` (0.65), `center`, `feather` (0.35), `roundness` (1), `color` (black), `mode` (`Color` or `Alpha`) | darkened edges, or edges fading out with `Alpha` |

Positions such as `start`, `end` and `center` run from 0 to 1 across `area` (1920 by 1080
by default). With `mask_to_source_alpha` a pattern covers only the opaque parts of the
children, which then render twice; without it, it covers the whole area.

The patterns are drawn as paths, not SVG `<pattern>` tiles: the backends round a small tile
to whole pixels differently, so its repeats drift apart across the picture. One pattern draws
at most `MAX_PATTERN_SHAPES` (20,000) stripes, cells or dots, and a finer one is drawn
coarser. `Rings` draws at most `MAX_RINGS` (1,024). As with noise, keep the lines and dots at
a few output pixels where the backends must agree.

`Vignette` is drawn with gradients, so it needs no filter. Its curve is a smoothstep sampled
at 16 points. Between `roundness` 0 and 1 the rectangle and the ellipse are mixed; Remotion
mixes the distances instead, so those shapes differ a little. `Gridlines` cannot tilt in 3D:
Remotion's `rotationX`, `rotationY` and `perspective` are not available.

```rust
use fframes_kit::prelude::*;

fn crt<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let roll = 8.0 * ui.seconds();
    ui.show(Vignette {
        amount: Anim::Fixed(0.7),
        children: children(move |ui| {
            ui.show(Scanlines {
                amount: Anim::Fixed(0.3),
                offset: roll,
                children: children(|_| svgr!(<rect x="0" y="0" width="1920" height="1080" fill="#2a9d8f" />)),
                ..ui.defaults()
            })
        }),
        ..ui.defaults()
    })
}
```

### Textures

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `Pattern` | `scale` (0.1), `crop` (left, top, right, bottom insets), `gap_x`, `gap_y`, `offset_u`, `offset_v`, `row_offset`, `row_offset_every` (0: never repeats), `column_offset`, `column_offset_every`, `origin` (0, 0), `wrap` (true) | the children repeated as a grid of small copies, staggered by row or column |
| `Speckle` | `density` (0.08), `size` (4), `randomness` (1) | small round holes punched through the children |
| `Flannel` | `amount` (0.7), `size` (96), `softness` (0.18), `base_color`, `stripe_color` | plaid cloth printed on the children, shaded by their brightness |
| `Burlap` | `amount` (0.55), `size` (5), `roughness` (0.7), `seed`, `color` (black) | woven sacking: strand gaps and blotches darken the picture |
| `Paper` | `amount` (1), `color_front`, `color_back`, `contrast` (0.3), `roughness` (0.4), `fiber` (0.3), `fiber_size` (0.2), `crumples` (0.3), `crumple_size` (0.35), `folds` (0.65), `fold_count` (5), `seed` (6), `scale` (0.6) | a lit relief of fibres, crumples and creases shading and tinting the picture |

`Pattern` renders the children once and places the copies with `<use>`, at most `MAX_TILES`
(1,024); a smaller `scale` is drawn larger. It replaces the children and is transparent
between the tiles. The other textures keep the children's alpha.

These are approximations of Remotion's per-pixel shaders:

- `Flannel` leaves out the pixel-sized thread ripple.
- `Burlap` leaves out the over and under weave, the strand jitter and the dashes; its strands
  are repeating gradients.
- `Paper` lights a relief built from `feTurbulence` and drawn creases. It does not displace the
  picture along the folds (that needs `feDisplacementMap`), its tint uses the average light,
  and `drops` and `fade` are not available.

Keep strands, fibres and holes a few output pixels wide, as with noise. `Paper`'s strong light
magnifies how differently the CPU and Skia backends sample `feTurbulence`: the gallery's paper
differs in about 1% of the pixels at full size and 6.6% at half size. That is why it has no e2e
frame.

```rust
use fframes_kit::prelude::*;

fn photo<'a>(_: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    svgr!(<rect x="0" y="0" width="1920" height="1080" fill="#f6d365" />)
}

fn wallpaper<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Paper {
        children: children(|ui| {
            ui.show(Pattern {
                scale: 0.2,
                row_offset: 192.0,
                row_offset_every: 2,
                children: children(photo),
                ..ui.defaults()
            })
        }),
        ..ui.defaults()
    })
}
```

### Halftone and light

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `Halftone` | `shape` (`Circle`, `Square`, `Line`), `dot_size` (20), `dot_spacing` (`None`: the dot size), `rotation` (degrees), `offset`, `color` (`Solid`, `Source`), `dot_color` (red), `invert` | the children printed as dots that grow where they are dark; the dots replace them |
| `HalftoneLinearGradient` | `first_dot_size` (0), `second_dot_size` (40), `first_stop` (0, 0.5), `second_stop` (1, 0.5), `grid_size` (24), `color`, `dot_color` (black), `mask_to_source_alpha` | dots growing along a gradient over the children, or the children seen only through them |
| `Starburst` | `rays` (12, 2 to 100), `colors` (accent and background), `rotation` (degrees), `smoothness` (0 to 1), `origin` (0.5, 0.5) | rays of colour from a point; it replaces the layer and takes no children |
| `LightTrail` | `direction` (180, degrees clockwise from the right), `distance` (80, animatable), `intensity` (1), `decay` (0.9), `threshold` (0), `samples` (32, at most `MAX_TRAIL_SAMPLES`), `color` (white) | a streak of light added behind the children's silhouette |

`Halftone` compares every pixel's brightness with a field of gradients that peaks in the dot
middles, inside one filter over the `area`. Remotion sizes a whole dot by the brightness at
its middle, so here a dot on an edge in the picture takes the edge's shape. Circles are drawn
one by one, at most `MAX_PATTERN_SHAPES`; squares and lines are gradients. `Starburst` blends
a ray into the next in twelve steps, smoothly across the boundary. `LightTrail` averages
copies of the silhouette by doubling, like `LongShadows`, and its threshold looks only at the
alpha (Remotion also lets bright semi-transparent pixels through).

```rust
use fframes_kit::prelude::*;

fn photo<'a>(_: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    svgr!(<rect x="0" y="0" width="1920" height="1080" fill="#f6d365" />)
}

fn poster<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let rays = ui.show(Starburst {
        rays: 16,
        rotation: 10.0 * ui.seconds(),
        smoothness: 0.4,
        ..ui.defaults()
    });
    let dots = ui.show(Halftone {
        dot_size: 32.0,
        rotation: 45.0,
        color: HalftoneColor::Source,
        children: children(photo),
        ..ui.defaults()
    });
    let trail = ui.show(LightTrail {
        distance: Anim::Fixed(240.0),
        children: children(|_| svgr!(<circle cx="1400" cy="540" r="120" fill="#ffffff" />)),
        ..ui.defaults()
    });
    svgr!(<g>{rays}{dots}{trail}</g>)
}
```

### Geometry and reveals

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `Mirror` | `direction` (`Horizontal`, `Vertical`), `position` (0.5, animatable), `invert` | the part past `position` shows the other side reflected |
| `Tile` | `rect` (x, y, width, height: 640, 360, 640, 360), `horizontal` (true), `vertical` (true) | `rect` of the children repeated across the area, every other copy mirrored |
| `Skew` | `x` (20 degrees), `y` (0), `origin` (0.5, 0.5) | slanted like CSS `skewX` then `skewY` |
| `Scale` | `scale` (1), `horizontal` (true), `vertical` (true) | scaled about the middle of the area |
| `Translate` | `x`, `y` (0), `uv` (false: units; true: shares of the area) | moved |
| `VenetianBlinds` | `progress` (0.5), `direction` (`Vertical`, `Horizontal`), `slats` (12) | shown through blinds opening from the middle of each slat |
| `Evolve` | `progress` (0.5), `direction` (`Left`, `Right`, `Top`, `Bottom`), `feather` (0.1) | revealed by an edge sweeping across, softened over `feather` |
| `TvSignalOff` | `amount` (1) | television test bars mixed in inside the children's alpha |

All of them except `TvSignalOff` work inside the `area` from the origin and cut what leaves it,
like Remotion's canvas. `progress`, `position`, `scale`, `amount` and the offsets and angles are animatable.
Remotion's `tile()` finds the rectangle to repeat from the opaque pixels; fframes cannot
measure the children, so `Tile` takes it as `rect`. Where a mirrored side reaches past the
area, Remotion stretches the edge pixels and `Mirror` leaves it transparent. `TvSignalOff`
draws the bars over the children, so semi-transparent parts mix a little differently.

`Mirror` and `Tile` lay a slightly larger copy under the exact one so that no seam shows
where copies meet; they render their children twice.

```rust
use fframes_kit::prelude::*;

fn photo<'a>(_: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    svgr!(<rect x="0" y="0" width="1920" height="1080" fill="#f6d365" />)
}

fn intro<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let progress = Anim::Fixed((ui.seconds() / 1.5).min(1.0));
    ui.show(VenetianBlinds {
        progress,
        slats: 8,
        children: children(|ui| {
            ui.show(Mirror {
                children: children(photo),
                ..ui.defaults()
            })
        }),
        ..ui.defaults()
    })
}
```

### Region and progressive blur

| Component | Props (defaults) | What it does |
| --- | --- | --- |
| `RegionBlur` | `top_left` (0.25, 0.25), `bottom_right` (0.75, 0.75), `radius` (40, animatable), `feather` (0 units), `roundness` (0 to 1) | the children blurred inside a rectangle with soft edges and round corners |
| `LinearProgressiveBlur` | `start` (0, 0.5), `end` (1, 0.5), `start_blur` (0), `end_blur` (50) | the blur grows along a line |
| `RadialProgressiveBlur` | `center` (0.5, 0.5), `width` (1), `height` (1), `rotation` (degrees), `start` (0), `start_blur` (0), `end_blur` (50) | the blur grows from the centre out to an ellipse |

Positions are shares of the `area`. Radii are Remotion's: a Gaussian with a standard deviation
of a third of the radius, so `RegionBlur { radius: 30.0, .. }` blurs like `Blur { radius:
10.0, .. }`. The progressive blurs draw blurred copies under graded masks: one copy for every 6
units of radius, at most `MAX_BLUR_LEVELS` (8) after the sharp one, cross-faded in between.
`blur_levels(start, end)` returns the radii. Near the edges of the area a blurred copy fades
out, where Remotion repeats the edge pixels, so the less blurred copy below shows through.

What they cost, from the bench card filling a 1080p frame (`BenchKit`, 5 seconds, CPU backend,
the median of three `render` runs):

| Scene | Seconds | Against `BenchKit` |
| --- | --- | --- |
| `BenchKit` | 1.05 | 1.0 |
| `BenchRegionBlur` | 1.80 | 1.7 |
| `BenchLinearProgressiveBlur` | 7.39 | 7.0 |
| `BenchRadialProgressiveBlur` | 7.38 | 7.0 |

Each blurred copy is a blur of the whole area, about 5 ms a frame at 1080p on the CPU backend;
the defaults (0 to 50) take eight, a smaller `end_blur` fewer.

```rust
use fframes_kit::prelude::*;

fn photo<'a>(_: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    svgr!(<rect x="0" y="0" width="1920" height="1080" fill="#f6d365" />)
}

fn tilt_shift<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(RadialProgressiveBlur {
        width: 1.2,
        height: 0.5,
        start: 0.4,
        end_blur: 36.0,
        children: children(|ui| {
            ui.show(RegionBlur {
                top_left: (0.7, 0.05),
                bottom_right: (0.95, 0.2),
                feather: 24.0,
                roundness: 0.5,
                children: children(photo),
                ..ui.defaults()
            })
        }),
        ..ui.defaults()
    })
}
```

### Not yet available

These `@remotion/effects` effects move or sample single pixels, which SVG filters cannot do.
They need per-pixel code and are not in the kit yet: fisheye, barrel distortion, sphere,
cylinder, pixelate, linear and radial progressive pixelate, pixel dissolve, chromatic
aberration, zoom blur, lens blur, page turn, shatter, tear, corner pin, contour lines, liquid
contours and shine. `Halftone` approximates `halftone()` without its `sampling`, and `Lut`
approximates a LUT with per-channel tables instead of reading a LUT file. The displacement
effects wait for a fixed `svgr` (see "Noise").

### Remotion animation utilities

`makeTransform` from `@remotion/animation-utils` is `fframes::Transform`, which the prelude
exports. It writes its parts in a fixed order: translate, rotate, scale, skew X, skew Y.
`interpolateStyles` is `motion::progress` over the input range, then `motion::lerp` for each
value.

```rust
use fframes_kit::prelude::*;

// makeTransform([translate(40, 0), rotate(15), scale(1.5)])
let transform = Transform {
    translate_x: 40.0,
    rotate: 15.0_f64.into(),
    scale: 1.5_f64.into(),
    ..Default::default()
};
assert_eq!(transform.to_string(), "translate(40 0) rotate(15) scale(1.5)");

// interpolateStyles(frame, [0, 20], [{ opacity: 0, translateY: 40 }, { opacity: 1, translateY: 0 }])
let t = motion::progress(5.0, 0.0, 20.0);
let (opacity, y) = (motion::lerp(0.0, 1.0, t), motion::lerp(40.0, 0.0, t));
assert_eq!((opacity, y), (0.25, 30.0));
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

## Tag form (`ui!`)

`ui!` is `svgr!` markup that also takes component tags. Each tag is expanded to the struct
form above, so the two forms draw the same frames. The struct form stays the main documented
way. The tag form is shorter for markup that nests containers.

- A tag whose name starts with an uppercase letter is a component; everything else is passed
  to `svgr!` unchanged.
- Attributes are fields:
  - a quoted value or an `{expr}` goes through `.into()`: `name="Ada"`, `enter={fade}`;
  - a number or `true`/`false` alone in braces is written as is, so the field's type decides
    it: `x={60.0}`, `times={3}`;
  - a bare attribute is `true`: `<Axes tips />`.

  Quoted values suit text fields. A number field needs braces: `cx={320.0}`, not `cx="320"`.
- Children fill the component's `children: Children` field. They are a closure over the
  container's `ui`, so tags inside them see its local time.
  - The closure runs every frame, so build what it draws inside it.
  - A value it uses must be `Copy`, a reference, or `'a`. A `Svgr` made outside cannot be
    moved in.
  - Containers whose children are a `Vec` (`LaggedStart`, `AnimationGroup`) keep the
    struct form.
- Every component with theme defaults works as a tag, your own included. That is every
  component that implements `Themed`; the time containers do too.
- It needs a variable `ui: Ui` in scope.

```rust
use fframes_kit::prelude::*;

fn card<'a>(frame: Frame, ctx: &FFramesContext<'a, '_>, theme: &'a Theme) -> Svgr<'a> {
    let mut ui = Ui::new(frame, ctx, theme);
    let create: Option<EnterAnim> = Some(Create::default().into());
    ui!(
        <g>
            <LowerThird name="Ada" role="Mathematician" min_width={300.0} />
            <Sequence from={0.5} duration={Some(2.0)}>
                <Circle cx={960.0} cy={540.0} radius={120.0} enter={create} />
                <text x="96" y="140" fill="#fff">{format!("{:.1} s", ui.seconds())}</text>
            </Sequence>
        </g>
    )
}
```

A dashed attribute is an error that names the field (`min-width` is `min_width`):

```rust,compile_fail
use fframes_kit::prelude::*;

fn card<'a>(frame: Frame, ctx: &FFramesContext<'a, '_>, theme: &'a Theme) -> Svgr<'a> {
    let mut ui = Ui::new(frame, ctx, theme);
    ui!(<LowerThird name="Ada" min-width={300.0} />)
}
```

Without a `ui` in scope the compiler reports "cannot find value `ui`" at the first tag.
Create one with `let mut ui = Ui::new(frame, ctx, &self.theme)`:

```rust,compile_fail
use fframes_kit::prelude::*;

fn card<'a>(frame: Frame, ctx: &FFramesContext<'a, '_>, theme: &'a Theme) -> Svgr<'a> {
    ui!(<LowerThird name="Ada" />)
}
```

## Editable props (fframes Studio)

fframes Studio can change a component's props without code edits. In the Props panel, select
the component on the canvas, change a value, and the preview updates. The values are saved in
the project's `props.json`.

A component is editable when it has a key:
- in the struct form, `ui.show(LowerThird { .. }.editable("speaker"))`;
- in a tag, `<LowerThird editable="speaker" .. />`.

Keys must be unique within a scene.
- Without the kit feature `studio`, `editable` renders the component unchanged. Code that marks
  components therefore builds the same everywhere.
- With `studio`, every kit component implements `Props`. Its editable fields are numbers,
  `bool`, text, `Color`, `Option<Color>` (`null` is none) and a fixed `Anim<f32>`.
  - Children, slices, keyframes and styles stay in code.
  - Your own components get it with `#[derive(Props)]`. Use `#[props(skip)]` to keep a field
    out, and `#[props(min = 0.0, max = 100.0, step = 1.0, unit = "px")]` to bound a number.
  - Enums whose variants have no fields get it with `#[derive(PropChoice)]`.

The video keeps a `PropsStore` and renders with it through `Ui::with_props`:

```rust
use fframes_kit::prelude::*;

fn title<'a>(frame: Frame, ctx: &FFramesContext<'a, '_>, theme: &'a Theme) -> Svgr<'a> {
    let mut ui = Ui::new(frame, ctx, theme);
    ui.show(
        LowerThird {
            name: "Ada".into(),
            ..ui.defaults()
        }
        .editable("speaker"),
    )
}

# #[cfg(feature = "studio")]
fn with_store<'a>(
    frame: Frame,
    ctx: &FFramesContext<'a, '_>,
    theme: &'a Theme,
    store: &'a PropsStore, // `Arc<PropsStore>` in the video, `PropsStore::load("props.json")`
) -> Svgr<'a> {
    let mut ui = Ui::new(frame, ctx, theme).with_props(store);
    ui!(<LowerThird editable="speaker" name="Ada" />)
}
```

When the component renders with a store:
- the document's values for it replace the code's values;
- it is drawn inside a `<g>` whose id is its editor identity: the scene's
  `editor_instance_key` (else its name; `video` outside scenes), the type name, the key, and
  `main`;
- a value that does not fit its field is skipped with a warning in `inspect`.

A CLI render reads the same document with `PropsStore::load`, so exports match the preview.
The project's Studio worker hands the store to the preview worker:

```rust,ignore
props.listen(); // record each editable object's schema and values for the panel
let (apply, info) = (props.clone(), props.clone());
let config = config.with_props_args(&args).with_props(PropsHooks::new(
    move |doc| apply.set(doc),
    move |identity| info.info(identity),
));
// export: run_export_with_props(&video, &options, &args, &hooks)
// export capabilities: print_export_capabilities_with_props::<MyVideo>()
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
