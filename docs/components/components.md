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
- Build a `Ui` once at the top of `render_frame` (of the video or a scene):
  `let mut ui = Ui::new(frame, ctx, &self.theme);`.
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

## Phase-one components

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
