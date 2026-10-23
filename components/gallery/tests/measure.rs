//! `Ui::measure` (any string) against `Frame::text_width` (borrowed strings) with the
//! gallery's DM Sans: both sum character advances, so they agree exactly.

use std::sync::Mutex;

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;

#[derive(Debug)]
struct Probe {
    theme: Theme,
    texts: [&'static str; 3],
    widths: Mutex<Vec<(Option<f32>, Option<usize>)>>,
}

impl Video for Probe {
    const FPS: usize = 30;
    const WIDTH: usize = 640;
    const HEIGHT: usize = 360;

    fn duration(&self) -> Duration<'_> {
        Duration::Frames(1)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::new(frame, ctx, &self.theme);
        let font = self.theme.typography.body.font();
        let widths = self
            .texts
            .iter()
            .map(|text| {
                let built = (*text).to_owned();
                (ui.measure(font, &built), ui.text_width(font, text))
            })
            .collect();
        *self.widths.lock().unwrap() = widths;
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360" />)
    }
}

#[test]
fn measure_matches_text_width() {
    let media = GalleryMedia::prepare().expect("gallery media");
    let probe = Probe {
        theme: Theme::default(),
        texts: ["Ada Lovelace", "t = 12.34s, WAVE", ""],
        widths: Mutex::new(Vec::new()),
    };
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(&probe, &options).expect("previewer");
    previewer.svg(0).expect("frame");

    let widths = probe.widths.lock().unwrap().clone();
    assert_eq!(widths.len(), 3);
    for (measured, core) in widths {
        let core = core.expect("DM Sans resolves");
        assert_eq!(measured, Some(core as f32));
    }
    assert!(probe.widths.lock().unwrap()[0].1.unwrap() > 100);
}
