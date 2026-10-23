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
springs. `motion` has `progress`, `clamp01`, `lerp`, `stagger(i, step)`, `spring`, `hash` and
`noise`.

```rust
use fframes_kit::prelude::*;

assert_eq!(motion::progress(0.5, 0.0, 2.0), 0.25);
assert_eq!(RateFunc::Linear.apply(0.3, 1.0), 0.3);
assert_eq!(motion::noise(7, 1.25), motion::noise(7, 1.25));
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
