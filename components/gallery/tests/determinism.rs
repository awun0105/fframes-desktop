//! Seeded simulations draw the same frames in two separately built videos: the gas and the
//! field lines are stepped in each video's constructor, the particles of a change of state are
//! placed from their seed, and their SVG, with every path's coordinates, must match exactly.

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;

/// A heated gas box over the field of a dipole, both built when the probe is, and a solid
/// melting next to them.
#[derive(Debug)]
struct Probe {
    theme: Theme,
    gas: GasMotion,
    field: Field,
}

impl Probe {
    fn new() -> Self {
        Self {
            theme: Theme::default(),
            gas: GasMotion::new(GasSetup {
                seed: 11,
                count: 40,
                seconds: 2.0,
                heat: Some(Heat {
                    at: 0.5,
                    to: 2.0,
                    over: 1.0,
                }),
                ..GasSetup::default()
            }),
            field: Field::new(
                vec![Charge::new(-1.5, 0.0, 1.0), Charge::new(1.5, 0.0, -1.0)],
                8,
                (-4.0, -2.5, 4.0, 2.5),
            ),
        }
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
        let field = ui.show(FieldLines {
            field: Some(&self.field),
            test_charge: Some(1),
            ..ui.defaults()
        });
        let gas = ui.show(GasBox {
            motion: Some(&self.gas),
            scale: 0.5,
            ..ui.defaults()
        });
        let states = ui.show(StatesOfMatter {
            seed: 5,
            at: 0.2,
            duration: 1.6,
            center: (2.5, 0.0),
            ..ui.defaults()
        });
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">{field}{gas}{states}</svg>)
    }
}

/// The SVG of `frame` in a freshly built probe.
fn drawn(frame: usize) -> String {
    let probe = Probe::new();
    let media = GalleryMedia::prepare().expect("gallery media");
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(&probe, &options).expect("previewer");
    previewer.svg(frame).expect("frame")
}

#[test]
fn two_builds_draw_the_same_frames() {
    // at the start, while heating and after it
    for frame in [0, 25, 59] {
        let first = drawn(frame);
        assert!(first.matches("<path").count() > 40, "{first}");
        assert_eq!(first, drawn(frame), "frame {frame}");
    }
}
