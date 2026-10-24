# Getting started with fframes-kit

Add the kit to a video made by `cargo fframes new` and show a title and a lower third.

## 1. Create the project

The kit is built against the fframes of this repository, so the project must take fframes from
the same place: the same checkout (`--fframes-path`) or the same git repository. Two copies of
fframes do not mix; their types differ and the project does not compile.

```bash
# from a checkout of this repository
cargo run -p cargo-fframes -- fframes new my-video --dir ../my-video \
  --fframes-path "$PWD" --backend cpu --yes
```

- `--backend cpu` renders with the CPU backend. Leave it out on a machine with a GPU to get
  the Skia Vulkan (or Metal) backend.
- On Windows, check the `fframes = { path = ... }` lines of the new `Cargo.toml`: a path
  written with backslashes is not valid TOML (`\U` reads as an escape). Write it with forward
  slashes, `C:/Users/me/fframes-desktop/fframes`.
- Without a checkout, generate the project with `--git` and change the URL of every
  `fframes` line to this repository's, the one the kit comes from below.

## 2. Add the kit

One dependency line in `[dependencies]`, from the same place as fframes:

```toml
# next to a checkout
fframes-kit = { path = "/path/to/fframes-desktop/components/fframes-kit" }
# or from git, with fframes = { git = <the same URL>, ... }
fframes-kit = { git = "https://github.com/khanhthanhdev/fframes-desktop" }
```

The kit is 0.x: its API may still change between versions. Pin a `rev` in the git form to
keep a project on one version. Explainer kits are features, for example
`fframes-kit = { ..., features = ["math"] }`; see [components.md](components.md).

## 3. A first video

Replace `src/lib.rs` with the following (the names are those of a project called
`kit-sample`; `main.rs` stays as generated). One import brings the kit, `svgr!` and the fframes
types a video needs:

```rust,ignore
use fframes::include_media_dir;
use fframes_kit::prelude::*;

// Every file of `media/` is embedded into the binary. Fonts are used by family name; the
// kit's default theme uses DM Sans, which `cargo fframes new` already put there.
include_media_dir!(pub struct KitSampleMedia, "media");

#[derive(Debug)]
pub struct KitSampleVideo<'a> {
    pub media: &'a KitSampleMedia,
    title: String,
    theme: Theme,
}

impl<'a> KitSampleVideo<'a> {
    pub fn new(media: &'a KitSampleMedia, title: &str) -> Self {
        // The kit's defaults with an orange accent and a light background.
        let theme = Theme {
            colors: Colors {
                accent: Color::hex("#ff5500"),
                background: Color::hex("#f8fafc"),
                title: Color::hex("#0f172a"),
                text: Color::hex("#0f172a"),
                muted: Color::hex("#475569"),
                ..Colors::default()
            },
            ..Theme::default()
        };
        Self { media, title: title.to_owned(), theme }
    }
}

impl Video for KitSampleVideo<'_> {
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
        let style = self.theme.typography.title;
        let title = self.title.as_str();

        // The title fades in and out over the whole video ...
        let heading = ui.show(SceneFade::new(
            &ui,
            children(move |_| {
                svgr!(
                    <text x="96" y="420" font-family={style.family} font-size={style.size}
                          font-weight={style.weight} fill={c.title}>{title}</text>
                )
            }),
        ));
        // ... and a lower third shows from 1 s to 5 s.
        let lower_third = ui.show(Sequence {
            from: 1.0,
            duration: Some(4.0),
            children: children(|ui| {
                ui.show(LowerThird {
                    name: "Ada Lovelace".into(),
                    role: "Mathematician".into(),
                    ..ui.defaults()
                })
            }),
        });

        svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080"
                 width={ctx.current_video_size.width} height={ctx.current_video_size.height}>
                <rect width="1920" height="1080" fill={c.background} />
                {heading}
                {lower_third}
            </svg>
        )
    }
}
```

- `Ui` carries the frame, the context and the theme. `ui.show(component)` renders a component,
  and `..ui.defaults()` fills the props you do not set from the theme.
- `Sequence` shifts time: inside it, the lower third starts at its own 0 s, one second into
  the video, and is gone after `duration`.
- The `Theme` is a plain value. Build it once in the constructor, override the fields you need
  (`colors`, `typography`, `spacing`, ...) and keep the rest with `..Theme::default()`.

## 4. Check it

```bash
cargo run --release -- inspect        # "no problems found"
cargo run --release -- strip -n 8     # one image of the whole video
cargo run --release -- render         # the video file
```

[components.md](components.md) goes on from here: the time containers, fades, `Anim`,
transitions, effects and writing your own components.
