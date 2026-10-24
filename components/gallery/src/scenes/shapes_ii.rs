//! Manim's second set of shapes, one scene per group.

use fframes_kit::{kurbo, prelude::*, shapes::SMALL_BUFF};

use crate::{GalleryScene, scenes::core::title};

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the matchers draw around their words, the dashes and the curves are written
        GalleryScene::new("Matchers", 2.0, matchers),
    ]
}

fn create() -> EnterAnim {
    Create::default().into()
}

/// A word from `(x, y)` on its baseline, as glyph outlines, and the box of its ink.
fn word<'a>(
    ui: &mut Ui<'a, '_, '_>,
    text: &'static str,
    x: f32,
    y: f32,
) -> (Svgr<'a>, kurbo::Rect) {
    let word = TextPath {
        text,
        x,
        y,
        style: ui.theme.typography.title,
        fill: Some(ui.theme.colors.title),
        stroke: None,
        ..ui.defaults()
    };
    let bounds = word.bounds(ui.ctx);
    (ui.show(word), bounds)
}

fn matchers<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Matchers");
    let (surround_word, surround_box) = word(ui, "Surrounding", 200.0, 360.0);
    let surround = ui.show(SurroundingRectangle {
        around: surround_box,
        corner_radius: 12.0,
        enter: Some(create()),
        ..ui.defaults()
    });
    let (under_word, under_box) = word(ui, "Underline", 1100.0, 360.0);
    let under = ui.show(Underline {
        around: under_box,
        enter: Some(create()),
        ..ui.defaults()
    });
    // stripes that the background rectangle covers
    let stripes: Vec<Svgr<'a>> = (0..8)
        .map(|i| {
            let x = 200.0 + i as f32 * 80.0;
            let fill = ui.theme.colors.accent;
            svgr!(<rect x={x} y="520" width="40" height="200" fill={fill} />)
        })
        .collect();
    let (back_word, back_box) = word(ui, "Background", 230.0, 650.0);
    let back = ui.show(BackgroundRectangle {
        around: back_box,
        buff: SMALL_BUFF,
        ..ui.defaults()
    });
    let circle = Circle {
        cx: 1250.0,
        cy: 620.0,
        radius: 100.0,
        ..ui.defaults()
    };
    let crossed = Cross {
        around: circle.bbox(),
        enter: Some(create()),
        ..ui.defaults()
    };
    let circle = ui.show(circle);
    let crossed = ui.show(crossed);
    let wave = kurbo::BezPath::from_svg("M200 900 C 450 780 650 1020 900 900").unwrap_or_default();
    let dashed = ui.show(DashedVMobject {
        path: wave,
        num_dashes: 12,
        enter: Some(create()),
        ..ui.defaults()
    });
    let star = Star {
        cx: 1550.0,
        cy: 880.0,
        outer_radius: 120.0,
        inner_radius: 50.0,
        ..ui.defaults()
    };
    let c = ui.theme.colors;
    let curves = ui.show(CurvesAsSubmobjects {
        path: star.path(),
        colors: vec![c.accent, c.highlight],
        stroke_width: 6.0,
        enter: Some(Write::default().into()),
        ..ui.defaults()
    });
    svgr!(
        <g>
            {head}
            {surround_word}{surround}
            {under_word}{under}
            {stripes}{back}{back_word}
            {circle}{crossed}
            {dashed}{curves}
        </g>
    )
}
