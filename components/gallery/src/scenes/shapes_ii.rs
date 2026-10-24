//! Manim's second set of shapes, one scene per group.

use std::f32::consts::{FRAC_PI_2, PI};

use fframes_kit::{kurbo, prelude::*};

use crate::{
    GalleryScene,
    scenes::{core::title, latex::formulas},
};

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the matchers draw around their words, the dashes and the curves are written
        GalleryScene::new("Matchers", 2.0, matchers),
        // the arcs and curves are created, the dots grow and the labels are written
        GalleryScene::new("Arcs", 2.0, arcs),
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

/// A caption under a cell centered at `x`.
fn caption<'a>(ui: &Ui<'a, '_, '_>, text: &'static str, x: f32, y: f32) -> Svgr<'a> {
    let s = ui.theme.typography.caption;
    let fill = ui.theme.colors.muted;
    svgr!(
        <text x={x} y={y} text-anchor="middle" font-family={s.family} font-size={s.size}
              font-weight={s.weight} fill={fill}>{text}</text>
    )
}

#[allow(clippy::too_many_lines)]
fn arcs<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Arcs");
    let enter = Some(create());
    let grow = Some(GrowFromCenter::default().into());
    let write = Some(Write::default().into());
    // first row: arcs between points
    let between = ui.show(ArcBetweenPoints {
        x1: 140.0,
        y1: 330.0,
        x2: 440.0,
        y2: 330.0,
        angle: -PI * 0.75,
        enter,
        ..ui.defaults()
    });
    let double = ui.show(CurvedDoubleArrow {
        x1: 580.0,
        y1: 380.0,
        x2: 880.0,
        y2: 380.0,
        angle: -FRAC_PI_2,
        enter,
        ..ui.defaults()
    });
    let muted = ui.theme.colors.muted;
    let guides = svgr!(
        <path d="M1040 440 L1340 440 M1040 440 L1040 240" stroke={muted} stroke-width="2" fill="none" />
    );
    let tangent = ui.show(TangentialArc {
        line1: ((1040.0, 440.0), (1340.0, 440.0)),
        line2: ((1040.0, 440.0), (1040.0, 240.0)),
        radius: 120.0,
        enter,
        ..ui.defaults()
    });
    let handles = svgr!(
        <path d="M1480 440 L1560 260 M1780 260 L1720 440" stroke={muted} stroke-width="2" fill="none" />
    );
    let bezier = ui.show(CubicBezier {
        start: (1480.0, 440.0),
        start_handle: (1560.0, 260.0),
        end_handle: (1780.0, 260.0),
        end: (1720.0, 440.0),
        enter,
        ..ui.defaults()
    });
    // second row: shapes of arcs
    let sector = ui.show(AnnularSector {
        cx: 290.0,
        cy: 680.0,
        inner_radius: 70.0,
        outer_radius: 140.0,
        start_angle: 0.3,
        angle: 2.2,
        enter: grow,
        ..ui.defaults()
    });
    let polygon = ui.show(ArcPolygon {
        points: vec![(730.0, 560.0), (850.0, 760.0), (610.0, 760.0)],
        angle: -0.8,
        enter,
        ..ui.defaults()
    });
    let lens = ui.show(ArcPolygonFromArcs {
        arcs: vec![
            ArcBetweenPoints {
                x1: 1060.0,
                y1: 680.0,
                x2: 1320.0,
                y2: 680.0,
                angle: 1.4,
                ..ui.defaults()
            },
            ArcBetweenPoints {
                x1: 1320.0,
                y1: 680.0,
                x2: 1060.0,
                y2: 680.0,
                angle: 1.4,
                ..ui.defaults()
            },
        ],
        fill: Some(ui.theme.colors.highlight.with_alpha(80)),
        enter,
        ..ui.defaults()
    });
    let annotation = ui.show(AnnotationDot {
        cx: 1600.0,
        cy: 680.0,
        enter: grow,
        ..ui.defaults()
    });
    // third row: labels
    let labeled_dot = ui.show(LabeledDot {
        label: "A".into(),
        cx: 290.0,
        cy: 930.0,
        enter: grow,
        ..ui.defaults()
    });
    let labeled_line = ui.show(LabeledLine {
        label: "d = 5".into(),
        x1: 560.0,
        y1: 930.0,
        x2: 900.0,
        y2: 930.0,
        enter: write,
        ..ui.defaults()
    });
    let labeled_arrow = ui.show(LabeledArrow {
        label: (&formulas().euler).into(),
        x1: 1040.0,
        y1: 930.0,
        x2: 1780.0,
        y2: 930.0,
        label_style: TextStyle {
            size: 40,
            ..ui.theme.typography.body
        },
        enter: write,
        ..ui.defaults()
    });
    let captions: Vec<Svgr<'a>> = [
        ("ArcBetweenPoints", 290.0, 470.0),
        ("CurvedDoubleArrow", 730.0, 470.0),
        ("TangentialArc", 1190.0, 470.0),
        ("CubicBezier", 1630.0, 470.0),
        ("AnnularSector", 290.0, 830.0),
        ("ArcPolygon", 730.0, 830.0),
        ("ArcPolygonFromArcs", 1190.0, 830.0),
        ("AnnotationDot", 1600.0, 830.0),
        ("LabeledDot", 290.0, 1030.0),
        ("LabeledLine", 730.0, 1030.0),
        ("LabeledArrow", 1410.0, 1030.0),
    ]
    .into_iter()
    .map(|(text, x, y)| caption(ui, text, x, y))
    .collect();
    svgr!(
        <g>
            {head}
            {between}{double}{guides}{tangent}{handles}{bezier}
            {sector}{polygon}{lens}{annotation}
            {labeled_dot}{labeled_line}{labeled_arrow}
            {captions}
        </g>
    )
}
