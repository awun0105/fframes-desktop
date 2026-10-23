//! The fframes-kit gallery: one scene per component, named after the component, so
//! `frame Sequence@50%` shows `Sequence` and the e2e harness addresses scenes by name.

use fframes::include_media_dir;
use fframes_kit::prelude::*;

pub mod scenes;

include_media_dir!(pub struct GalleryMedia, "gallery/media");

/// Draws one gallery scene.
pub type Draw = for<'a, 'c, 'm> fn(&mut Ui<'a, 'c, 'm>) -> Svgr<'a>;

/// A scene that shows one component; `name()` is the component name.
#[derive(Debug)]
pub struct GalleryScene {
    pub name: &'static str,
    pub seconds: f32,
    pub theme: Theme,
    pub draw: Draw,
}

impl GalleryScene {
    pub fn new(name: &'static str, seconds: f32, draw: Draw) -> Self {
        Self {
            name,
            seconds,
            theme: Theme::default(),
            draw,
        }
    }
}

impl Scene for GalleryScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(self.seconds)
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::for_scene(frame, ctx, &self.theme, self);
        (self.draw)(&mut ui)
    }
}

#[derive(Debug)]
pub struct GalleryVideo<'a> {
    pub media: &'a GalleryMedia,
    pub scenes: Vec<GalleryScene>,
}

impl<'a> GalleryVideo<'a> {
    pub fn new(media: &'a GalleryMedia) -> Self {
        Self {
            media,
            scenes: scenes::all(),
        }
    }
}

impl Video for GalleryVideo<'_> {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;
    const BACKGROUND_COLOR: Color = Color::hex("#0b0d12");

    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn define_scenes(&self) -> Scenes<'_> {
        let scenes: Vec<&dyn Scene> = self.scenes.iter().map(|s| s as &dyn Scene).collect();
        Scenes::from(scenes)
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        svgr!(
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 1920 1080"
                width={ctx.current_video_size.width}
                height={ctx.current_video_size.height}
            >
                {ctx.render_scenes(&frame)}
            </svg>
        )
    }
}
