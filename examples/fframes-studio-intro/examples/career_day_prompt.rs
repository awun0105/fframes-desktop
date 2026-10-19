//! A settled fframes-components prompt, exported as vector artwork for the flyers.
use std::sync::LazyLock;

use fframes::{
    AudioMap, Color, CombinedMediaProvider, Duration, FFramesContext, Frame, MediaProvider,
    RenderOptions, StaticMediaProvider, Svgr, Video,
};
use fframes_components::components::showcase::prompt_zoom::{PromptZoom, PromptZoomConfig};
use fframes_components::ui::Mode;
use fframes_studio_intro::{StudioIntroMedia, scenes::keynote::comp};

static PROMPT: LazyLock<PromptZoom> = LazyLock::new(|| {
    PromptZoom::new(PromptZoomConfig {
        greeting: "What would you like to make?".into(),
        text: "Make a 15-second promo for our club's welcome night.".into(),
        placeholder: "Describe your video".into(),
        model: "AI".into(),
        effort: "Create video".into(),
        type_start: 0.0,
        cut_at: 100.0,
        chars_per_second: 100.0,
        accent_color: Some("#2f80ff".into()),
        mode: Mode::Dark,
        font_family: Some("IBM Plex Mono".into()),
        ..Default::default()
    })
});

#[derive(Debug)]
struct PromptArtwork;
impl Video for PromptArtwork {
    const FPS: usize = 1;
    const WIDTH: usize = 810;
    const HEIGHT: usize = 450;
    const BACKGROUND_COLOR: Color = Color::BLACK;
    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }
    fn duration(&self) -> Duration<'_> {
        Duration::Frames(1)
    }
    fn render_frame<'a>(&'a self, frame: Frame, _ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let prompt = comp::show(&*PROMPT, &frame, 3.0, (0.0, 0.0, 810.0, 450.0));
        fframes::svgr!(<svg xmlns="http://www.w3.org/2000/svg" width="810" height="450">{prompt}</svg>)
    }
}

fn main() -> std::process::ExitCode {
    let fonts = StudioIntroMedia::prepare().expect("flyer fonts");
    let components = fframes_components::ComponentsMedia::prepare().expect("component fonts");
    let media = CombinedMediaProvider::from([&fonts as &dyn MediaProvider, &components]);
    fframes::cli::new(
        &PromptArtwork,
        RenderOptions {
            media: Some(&media),
            ..Default::default()
        },
    )
    .run()
}
