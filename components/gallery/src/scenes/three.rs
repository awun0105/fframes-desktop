//! Manim's cameras and 3D mobjects.

use fframes_kit::explain::auto_zoom;
use fframes_kit::kurbo;
use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

/// The scene with `title`.
macro_rules! scenes {
    ($($name:literal, $seconds:expr => |$ui:ident| $body:expr,)*) => {
        vec![$(GalleryScene::new($name, $seconds, |$ui| {
            let head = title($ui, $name);
            let body: Svgr<'_> = $body;
            svgr!(<g>{head}{body}</g>)
        }),)*]
    };
}

/// A square, a circle and a small labelled dot to look at through the cameras.
fn still_life<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let square = ui.show(Square {
        cx: 560.0,
        cy: 580.0,
        side_length: 300.0,
        fill: Some(c.accent),
        ..ui.defaults()
    });
    let circle = ui.show(Circle {
        cx: 1340.0,
        cy: 580.0,
        radius: 160.0,
        stroke: Some(c.highlight),
        stroke_width: 6.0,
        ..ui.defaults()
    });
    let dot = ui.show(Dot {
        cx: 1340.0,
        cy: 580.0,
        radius: 6.0,
        fill: Some(c.title),
        ..ui.defaults()
    });
    let ring = ui.show(Circle {
        cx: 1340.0,
        cy: 580.0,
        radius: 24.0,
        stroke: Some(c.accent),
        stroke_width: 2.0,
        ..ui.defaults()
    });
    svgr!(<g>{square}{circle}{dot}{ring}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "MovingCamera", 2.0 => |ui| {
            // from the whole canvas onto the circle and back, like `self.camera.frame.animate`
            let space = ui.space;
            let full = fframes_kit::explain::full_frame_width(space);
            let circle = kurbo::Rect::new(1180.0, 420.0, 1500.0, 740.0);
            let (target, width) = auto_zoom(space, &[circle], 0.5).unwrap_or(((0.0, 0.0), full));
            let p = RateFunc::ThereAndBack.apply(motion::progress(ui.seconds(), 0.25, 1.75), 1.5);
            let camera = MovingCamera {
                center: (motion::lerp(0.0, target.0, p), motion::lerp(0.0, target.1, p)).into(),
                width: motion::lerp(full, width, p).into(),
                children: children(still_life),
            };
            ui.show(camera)
        },
        "ZoomedInset", 2.0 => |ui| {
            let inset = ZoomedInset {
                frame: kurbo::Rect::new(1290.0, 540.0, 1390.0, 600.0),
                display: kurbo::Rect::new(1400.0, 120.0, 1840.0, 384.0),
                delay: 0.25,
                run_time: 1.0,
                children: children(still_life),
                ..ui.defaults()
            };
            ui.show(inset)
        },
    ]
}
