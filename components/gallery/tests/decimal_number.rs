//! `DecimalNumber` puts every digit into a slot of one width, so the digits do not move
//! while the value changes.

use std::sync::Mutex;

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;

#[derive(Debug)]
struct Probe {
    theme: Theme,
    values: [f32; 2],
    anchor: TextAnchor,
    drawn: Mutex<Vec<String>>,
}

impl Video for Probe {
    const FPS: usize = 30;
    const WIDTH: usize = 640;
    const HEIGHT: usize = 360;

    fn duration(&self) -> Duration<'_> {
        Duration::Frames(2)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let value = self.values[frame.index.min(1)];
        let mut ui = Ui::new(frame, ctx, &self.theme);
        let number = ui.show(DecimalNumber {
            value: value.into(),
            x: 500.0,
            y: 200.0,
            anchor: self.anchor,
            ..ui.defaults()
        });
        self.drawn.lock().unwrap().push(number.to_string());
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">{number}</svg>)
    }
}

/// The text and x of every `<tspan>` of the number at both values.
fn glyphs(values: [f32; 2], anchor: TextAnchor) -> [Vec<(String, String)>; 2] {
    let media = GalleryMedia::prepare().expect("gallery media");
    let probe = Probe {
        theme: Theme::default(),
        values,
        anchor,
        drawn: Mutex::new(Vec::new()),
    };
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(&probe, &options).expect("previewer");
    previewer.svg(0).expect("frame");
    previewer.svg(1).expect("frame");
    drop(previewer);
    let drawn = probe.drawn.lock().unwrap();
    let between = |s: &str, start: &str, end: &str| -> String {
        let rest = s.split(start).nth(1).unwrap_or_else(|| panic!("{s}"));
        rest.split(end).next().unwrap().to_owned()
    };
    let parse = |tree: &String| -> Vec<(String, String)> {
        tree.split("tag_name: tspan")
            .skip(1)
            .map(|t| {
                (
                    between(t, "Text(Owned(\"", "\""),
                    between(t, "name: x, value: ", " }"),
                )
            })
            .collect()
    };
    [parse(&drawn[0]), parse(&drawn[1])]
}

/// Where the characters that are not digits are: the slots of the digits between them keep
/// one width, and each digit is centered in its slot.
fn separators(glyphs: &[(String, String)]) -> Vec<(String, String)> {
    glyphs
        .iter()
        .filter(|(text, _)| !text.chars().all(|c| c.is_ascii_digit()))
        .cloned()
        .collect()
}

#[test]
fn digits_keep_their_place() {
    let [a, b] = glyphs([1_111.11, 8_888.88], TextAnchor::Start);
    // "1", ",", "1", "1", "1", ".", "1", "1"
    assert_eq!((a.len(), b.len()), (8, 8), "{a:?}");
    assert_eq!(separators(&a), separators(&b));
    assert_eq!(separators(&a).len(), 2, "{a:?}");
}

#[test]
fn right_anchored_digits_stay_across_a_carry() {
    let [a, b] = glyphs([9.99, 10.0], TextAnchor::End);
    assert_eq!((a.len(), b.len()), (4, 5));
    assert_eq!(separators(&a), separators(&b));
}
