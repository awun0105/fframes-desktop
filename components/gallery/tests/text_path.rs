//! `TextPath` draws glyph outlines from the media's font files, and falls back to a `<text>`
//! that `Write` fades in word by word when the family has no file (system fonts, the editor).

use std::sync::Mutex;

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;

#[derive(Debug)]
struct Probe {
    theme: Theme,
    family: &'static str,
    drawn: Mutex<String>,
}

impl Video for Probe {
    const FPS: usize = 30;
    const WIDTH: usize = 640;
    const HEIGHT: usize = 360;

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(2.0)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::new(frame, ctx, &self.theme);
        let mut style = self.theme.typography.title;
        style.family = self.family;
        let text = ui.show(TextPath {
            text: "two words",
            x: 20.0,
            y: 200.0,
            style,
            enter: Some(Write::default().into()),
            ..ui.defaults()
        });
        *self.drawn.lock().unwrap() = text.to_string();
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">{text}</svg>)
    }
}

/// The `TextPath` element tree half way through its `Write`, as `Svgr` prints it with
/// the `compile-time-svgtree` feature of the `renderer` build.
fn drawn(family: &'static str) -> String {
    let media = GalleryMedia::prepare().expect("gallery media");
    let probe = Probe {
        theme: Theme::default(),
        family,
        drawn: Mutex::new(String::new()),
    };
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(&probe, &options).expect("previewer");
    previewer.svg(15).expect("frame");
    probe.drawn.lock().unwrap().clone()
}

#[test]
fn media_fonts_draw_one_path_per_glyph() {
    let tree = drawn("DM Sans");
    assert_eq!(
        tree.matches("tag_name: path").count(),
        "twowords".len(),
        "{tree}"
    );
    assert!(!tree.contains("tag_name: text"), "{tree}");
}

#[test]
fn unknown_family_falls_back_to_text_by_word() {
    let tree = drawn("No Such Font");
    assert!(tree.contains("tag_name: text"), "{tree}");
    assert_eq!(tree.matches("tag_name: tspan").count(), 2, "{tree}");
    assert!(tree.contains("Text(Owned(\"two \"))"), "{tree}");
}
