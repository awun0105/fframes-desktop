use fframes::{Color, Duration, Frame, Scene, Scenes, Svgr, Video};

use crate::StudioVideo;

impl Video for StudioVideo {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;
    const BACKGROUND_COLOR: Color = Color::BLACK;

    fn editor_instance_key(&self) -> Option<&str> {
        Some("starter-video")
    }

    /// The sum of the scenes, in the order Studio's timeline.json gives them.
    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }

    fn audio(&self) -> fframes::AudioMap<'_> {
        fframes::AudioMap::none()
    }

    /// Lists each scene once. Studio's timeline orders, trims and splits them.
    fn define_scenes(&self) -> Scenes<'_> {
        let scenes: Vec<&dyn Scene> = self.scenes.iter().map(|scene| scene as _).collect();
        Scenes::from(scenes)
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &fframes::FFramesContext<'a, '_>) -> Svgr<'a> {
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080" width={Self::WIDTH} height={Self::HEIGHT}>
                {ctx.render_scenes(&frame)}
            </svg>
        )
    }
}
