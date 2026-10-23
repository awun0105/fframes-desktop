//! One scene per shape: Manim outlines are drawn with `Create`, Remotion fills grow in.

use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

const CX: f32 = 960.0;
const CY: f32 = 580.0;

fn create() -> EnterAnim {
    Create::default().into()
}

fn grow() -> EnterAnim {
    GrowFromCenter::default().into()
}

macro_rules! scenes {
    ($($name:literal => |$ui:ident| $shape:expr,)*) => {
        vec![$(GalleryScene::new($name, 2.0, |$ui| {
            let head = title($ui, $name);
            let shape = $shape;
            let body = $ui.show(shape);
            svgr!(<g>{head}{body}</g>)
        }),)*]
    };
}

#[allow(clippy::too_many_lines)]
pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "Circle" => |ui| Circle { cx: CX, cy: CY, radius: 200.0, enter: Some(create()), ..ui.defaults() },
        "Dot" => |ui| Dot { cx: CX, cy: CY, radius: 24.0, enter: Some(grow()), ..ui.defaults() },
        "Ellipse" => |ui| Ellipse { cx: CX, cy: CY, width: 600.0, height: 300.0, enter: Some(create()), ..ui.defaults() },
        "Arc" => |ui| Arc {
            cx: CX, cy: CY, radius: 200.0, angle: 1.5 * std::f32::consts::PI, enter: Some(create()), ..ui.defaults()
        },
        "Annulus" => |ui| Annulus {
            cx: CX, cy: CY, inner_radius: 120.0, outer_radius: 200.0, enter: Some(grow()), ..ui.defaults()
        },
        "Sector" => |ui| Sector {
            cx: CX, cy: CY, radius: 220.0, start_angle: 0.3, angle: 1.2, enter: Some(grow()), ..ui.defaults()
        },
        "Line" => |ui| Line { x1: 560.0, y1: CY + 150.0, x2: 1360.0, y2: CY - 150.0, enter: Some(create()), ..ui.defaults() },
        "DashedLine" => |ui| DashedLine {
            x1: 560.0, y1: CY, x2: 1360.0, y2: CY, enter: Some(create()), ..ui.defaults()
        },
        "Polygon" => |ui| Polygon {
            points: vec![(760.0, 380.0), (1240.0, 460.0), (1160.0, 760.0), (820.0, 720.0), (700.0, 560.0)],
            enter: Some(create()),
            ..ui.defaults()
        },
        "RegularPolygon" => |ui| RegularPolygon { cx: CX, cy: CY, radius: 220.0, n: 6, enter: Some(create()), ..ui.defaults() },
        "Triangle" => |ui| Triangle { cx: CX, cy: CY + 40.0, radius: 240.0, enter: Some(create()), ..ui.defaults() },
        "Rectangle" => |ui| Rectangle { cx: CX, cy: CY, width: 640.0, height: 320.0, enter: Some(create()), ..ui.defaults() },
        "Square" => |ui| Square { cx: CX, cy: CY, side_length: 400.0, enter: Some(create()), ..ui.defaults() },
        "RoundedRectangle" => |ui| RoundedRectangle {
            cx: CX, cy: CY, width: 640.0, height: 320.0, corner_radius: 48.0, enter: Some(create()), ..ui.defaults()
        },
        "Star" => |ui| Star {
            cx: CX, cy: CY, outer_radius: 240.0, inner_radius: 96.0, enter: Some(create()), ..ui.defaults()
        },
        "Arrow" => |ui| Arrow {
            x1: 560.0, y1: CY, x2: 1360.0, y2: CY, tip_length: 48.0, tip_width: 40.0,
            enter: Some(GrowArrow::default().into()),
            ..ui.defaults()
        },
        "Vector" => |ui| Vector {
            x: CX, y: CY, dx: 300.0, dy: -200.0, tip_length: 40.0, tip_width: 32.0,
            enter: Some(GrowArrow::default().into()),
            ..ui.defaults()
        },
        "DoubleArrow" => |ui| DoubleArrow {
            x1: 560.0, y1: CY, x2: 1360.0, y2: CY, tip_length: 48.0, tip_width: 40.0,
            enter: Some(grow()),
            ..ui.defaults()
        },
        "CurvedArrow" => |ui| CurvedArrow {
            x1: 600.0, y1: CY - 100.0, x2: 1320.0, y2: CY - 100.0, tip_length: 40.0, tip_width: 32.0,
            enter: Some(create()),
            ..ui.defaults()
        },
        "Brace" => |ui| Brace { x1: 660.0, y1: CY, x2: 1260.0, y2: CY, depth: 48.0, enter: Some(create()), ..ui.defaults() },
        "Rect" => |ui| Rect {
            cx: CX, cy: CY, width: 480.0, height: 320.0, corner_radius: 24.0, enter: Some(grow()), ..ui.defaults()
        },
        "BlockArrow" => |ui| BlockArrow { cx: CX, cy: CY, length: 600.0, head_length: 200.0, head_width: 320.0, shaft_width: 140.0, enter: Some(grow()), ..ui.defaults() },
        "Callout" => |ui| Callout {
            cx: CX, cy: CY - 60.0, width: 640.0, height: 280.0, tail_x: CX - 160.0, tail_y: CY + 220.0,
            enter: Some(grow()),
            ..ui.defaults()
        },
        "Heart" => |ui| Heart { cx: CX, cy: CY, height: 400.0, enter: Some(grow()), ..ui.defaults() },
        "Pie" => |ui| {
            let progress = motion::progress(ui.seconds(), 0.0, 1.5);
            Pie { cx: CX, cy: CY, radius: 220.0, progress, ..ui.defaults() }
        },
        "Spark" => |ui| Spark { cx: CX, cy: CY, size: 240.0, enter: Some(SpinInFromNothing::default().into()), ..ui.defaults() },
    ]
}
