//! `ui!` in a real render: component tags draw the same tree as the struct form they expand
//! to, with literal, expression and bare attributes and nested container children.

use std::sync::Mutex;

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;

/// Draws a lower third, axes and a sequence holding a circle, written in tag form or in
/// struct form.
#[derive(Debug)]
struct Probe {
    theme: Theme,
    tags: bool,
    drawn: Mutex<String>,
}

impl Probe {
    fn with_tags<'a>(&'a self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let radius = 40.0 + 10.0 * ui.seconds();
        let fade: Option<EnterAnim> = Some(Create::default().into());
        ui!(
            <g>
                <LowerThird name="Ada" role={"Mathematician"} x={60.0} />
                <Axes tips x_length={4.0} />
                <Sequence from={0.5} duration={Some(1.0)}>
                    <Circle cx={320.0} cy={180.0} radius={radius} enter={fade} />
                    <rect width="10" height="10" />
                </Sequence>
            </g>
        )
    }

    fn with_structs<'a>(&'a self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let radius = 40.0 + 10.0 * ui.seconds();
        let fade: Option<EnterAnim> = Some(Create::default().into());
        let lower_third = ui.show(LowerThird {
            name: "Ada".into(),
            role: "Mathematician".into(),
            x: 60.0,
            ..ui.defaults()
        });
        let axes = ui.show(Axes {
            tips: true,
            x_length: 4.0,
            ..ui.defaults()
        });
        let sequence = ui.show(Sequence {
            from: 0.5,
            duration: Some(1.0),
            children: children(move |ui| {
                let circle = ui.show(Circle {
                    cx: 320.0,
                    cy: 180.0,
                    radius,
                    enter: fade,
                    ..ui.defaults()
                });
                svgr!(<g>{circle}<rect width="10" height="10" /></g>)
            }),
        });
        svgr!(<g>{lower_third}{axes}{sequence}</g>)
    }
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
        let tree = if self.tags {
            self.with_tags(&mut ui)
        } else {
            self.with_structs(&mut ui)
        };
        *self.drawn.lock().unwrap() = tree.to_string();
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">{tree}</svg>)
    }
}

/// The element tree of `frame`, as `Svgr` prints it.
fn drawn(tags: bool, frame: usize) -> String {
    let probe = Probe {
        theme: Theme::default(),
        tags,
        drawn: Mutex::new(String::new()),
    };
    let media = GalleryMedia::prepare().expect("gallery media");
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(&probe, &options).expect("previewer");
    previewer.svg(frame).expect("frame");
    probe.drawn.lock().unwrap().clone()
}

#[test]
fn tags_draw_what_their_structs_draw() {
    // before the sequence, while its circle is created, and after it ended
    let trees: Vec<String> = [0, 20, 40, 59]
        .into_iter()
        .map(|frame| {
            let tags = drawn(true, frame);
            assert_eq!(tags, drawn(false, frame), "frame {frame}");
            tags
        })
        .collect();
    // the circle is drawn inside the sequence (0.5 s to 1.5 s) and gone after it
    let rects = |tree: &str| tree.matches("tag_name: rect").count();
    assert_eq!(rects(&trees[0]), rects(&trees[3]));
    assert!(rects(&trees[1]) > rects(&trees[0]), "{}", trees[1]);
    assert_ne!(trees[1], trees[2]);
}
