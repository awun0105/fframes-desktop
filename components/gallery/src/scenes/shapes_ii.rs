//! Manim's second set of shapes, one scene per group.

use std::{
    f32::consts::{FRAC_PI_2, PI},
    sync::OnceLock,
};

use fframes_kit::{kurbo, prelude::*, shapes::Segment};

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
        // the angle marks and the tangent are created, the tips grow with their arrows
        GalleryScene::new("Angles", 2.0, angles),
        // the polygrams are created and the label is written into the widest place
        GalleryScene::new("Polygrams", 2.0, polygrams),
        // the braces are created and their labels fade in at the tips
        GalleryScene::new("Braces", 2.0, braces),
        // a circle and a square combined four ways, the results grow from their centers
        GalleryScene::new("Boolean", 2.0, boolean),
        // one SVG file drawn three ways and a path from path data, written on
        GalleryScene::new("SvgShape", 2.0, svg_shape),
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

/// Two lines from `corner` along the directions `a` and `b`, `length` long.
fn rays(corner: (f32, f32), a: (f32, f32), b: (f32, f32), length: f32) -> (Segment, Segment) {
    let end = |d: (f32, f32)| (corner.0 + d.0 * length, corner.1 + d.1 * length);
    ((corner, end(a)), (corner, end(b)))
}

#[allow(clippy::too_many_lines)]
fn angles<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Angles");
    let enter = Some(create());
    let muted = ui.theme.colors.muted;
    let mut lines: Vec<Svgr<'a>> = Vec::new();
    let mut line = |ui: &mut Ui<'a, '_, '_>, l: Segment| {
        lines.push(ui.show(Line {
            x1: l.0.0,
            y1: l.0.1,
            x2: l.1.0,
            y2: l.1.1,
            stroke: Some(muted),
            ..ui.defaults()
        }));
    };
    // an angle with a label, the other angle with a dot, a right angle
    let (a1, a2) = rays((160.0, 440.0), (1.0, 0.0), (0.8, -0.6), 300.0);
    line(ui, a1);
    line(ui, a2);
    let theta = ui.show(Angle {
        line1: a1,
        line2: a2,
        radius: 90.0,
        label: (&formulas().theta).into(),
        enter,
        ..ui.defaults()
    });
    let (b1, b2) = rays((620.0, 440.0), (1.0, 0.0), (0.8, -0.6), 300.0);
    line(ui, b1);
    line(ui, b2);
    let other = ui.show(Angle {
        line1: b1,
        line2: b2,
        radius: 36.0,
        other_angle: true,
        dot: true,
        enter,
        ..ui.defaults()
    });
    let (c1, c2) = rays((1080.0, 440.0), (1.0, 0.0), (0.0, -1.0), 240.0);
    line(ui, c1);
    line(ui, c2);
    let right = ui.show(RightAngle {
        line1: c1,
        line2: c2,
        radius: 48.0,
        dot: true,
        enter,
        ..ui.defaults()
    });
    let elbow = ui.show(Elbow {
        x: 1480.0,
        y: 440.0,
        width: 80.0,
        angle: 0.4,
        enter,
        ..ui.defaults()
    });
    let circle = Circle {
        cx: 1720.0,
        cy: 340.0,
        radius: 90.0,
        stroke: Some(muted),
        ..ui.defaults()
    };
    let tangent = ui.show(TangentLine {
        path: circle.path(),
        alpha: 0.15,
        length: 240.0,
        enter,
        ..ui.defaults()
    });
    let circle = ui.show(circle);
    // every tip kind on an arrow, growing with it
    let tips = [
        ArrowTip::Triangle,
        ArrowTip::TriangleFilled,
        ArrowTip::Circle,
        ArrowTip::CircleFilled,
        ArrowTip::Square,
        ArrowTip::SquareFilled,
        ArrowTip::Stealth,
    ];
    let arrows: Vec<Svgr<'a>> = tips
        .iter()
        .enumerate()
        .map(|(i, &tip)| {
            let x = 160.0 + i as f32 * 240.0;
            ui.show(Arrow {
                x1: x,
                y1: 900.0,
                x2: x + 160.0,
                y2: 700.0,
                tip_length: 36.0,
                tip_width: 32.0,
                tip_end: Some(tip),
                enter: Some(GrowArrow::default().into()),
                ..ui.defaults()
            })
        })
        .collect();
    let captions: Vec<Svgr<'a>> = [
        ("Angle", 310.0, 520.0),
        ("Angle, other_angle, dot", 770.0, 520.0),
        ("RightAngle", 1200.0, 520.0),
        ("Elbow", 1500.0, 520.0),
        ("TangentLine", 1720.0, 520.0),
        ("Triangle", 240.0, 960.0),
        ("TriangleFilled", 480.0, 960.0),
        ("Circle", 720.0, 960.0),
        ("CircleFilled", 960.0, 960.0),
        ("Square", 1200.0, 960.0),
        ("SquareFilled", 1440.0, 960.0),
        ("Stealth", 1680.0, 960.0),
    ]
    .into_iter()
    .map(|(text, x, y)| caption(ui, text, x, y))
    .collect();
    svgr!(
        <g>
            {head}{lines}{theta}{other}{right}{elbow}{circle}{tangent}{arrows}{captions}
        </g>
    )
}

fn polygrams<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Polygrams");
    let enter = Some(create());
    let c = ui.theme.colors;
    let polygram = ui.show(Polygram {
        vertex_groups: vec![
            vec![(160.0, 300.0), (420.0, 300.0), (420.0, 520.0)],
            vec![(160.0, 560.0), (420.0, 560.0), (160.0, 360.0)],
        ],
        enter,
        ..ui.defaults()
    });
    let star = ui.show(RegularPolygram {
        cx: 700.0,
        cy: 430.0,
        radius: 140.0,
        num_vertices: 7,
        density: 3,
        enter,
        ..ui.defaults()
    });
    let points = [
        (980.0, 340.0),
        (1120.0, 300.0),
        (1240.0, 380.0),
        (1200.0, 540.0),
        (1060.0, 560.0),
        (1100.0, 430.0),
        (1020.0, 470.0),
    ];
    let hull = ui.show(ConvexHull {
        points: points.to_vec(),
        enter,
        ..ui.defaults()
    });
    let dots: Vec<Svgr<'a>> = points
        .iter()
        .map(|&(cx, cy)| {
            ui.show(Dot {
                cx,
                cy,
                radius: 6.0,
                fill: Some(c.highlight),
                ..ui.defaults()
            })
        })
        .collect();
    let outer = kurbo::Shape::to_path(&kurbo::Rect::new(1400.0, 300.0, 1760.0, 560.0), 0.1);
    let hole = Circle {
        cx: 1580.0,
        cy: 430.0,
        radius: 80.0,
        ..ui.defaults()
    }
    .path();
    let cutout = ui.show(Cutout {
        outer,
        holes: vec![hole],
        enter: Some(GrowFromCenter::default().into()),
        ..ui.defaults()
    });
    let screen = ui.show(ScreenRectangle {
        cx: 420.0,
        cy: 840.0,
        height: 220.0,
        enter,
        ..ui.defaults()
    });
    let labeled = ui.show(LabeledPolygram {
        vertex_groups: vec![vec![
            (900.0, 690.0),
            (1000.0, 690.0),
            (1000.0, 760.0),
            (1560.0, 760.0),
            (1560.0, 980.0),
            (900.0, 980.0),
        ]],
        label: "widest place".into(),
        enter: Some(Write::default().into()),
        ..ui.defaults()
    });
    let captions: Vec<Svgr<'a>> = [
        ("Polygram", 290.0, 620.0),
        ("RegularPolygram {7/3}", 700.0, 620.0),
        ("ConvexHull", 1110.0, 620.0),
        ("Cutout", 1580.0, 620.0),
        ("ScreenRectangle", 420.0, 990.0),
        ("LabeledPolygram", 1230.0, 1030.0),
    ]
    .into_iter()
    .map(|(text, x, y)| caption(ui, text, x, y))
    .collect();
    svgr!(<g>{head}{polygram}{star}{hull}{dots}{cutout}{screen}{labeled}{captions}</g>)
}

fn braces<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Braces");
    let enter = Some(create());
    let muted = ui.theme.colors.muted;
    let line = ui.show(Line {
        x1: 160.0,
        y1: 400.0,
        x2: 760.0,
        y2: 400.0,
        stroke: Some(muted),
        ..ui.defaults()
    });
    let below = ui.show(BraceLabel {
        brace: Brace {
            x1: 160.0,
            y1: 430.0,
            x2: 760.0,
            y2: 430.0,
            depth: 30.0,
            enter,
            ..ui.defaults()
        },
        text: "BraceLabel, BraceText".into(),
        label_enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    let above = ui.show(BraceLabel {
        brace: Brace {
            x1: 160.0,
            y1: 370.0,
            x2: 460.0,
            y2: 370.0,
            depth: -30.0,
            enter,
            ..ui.defaults()
        },
        text: "depth < 0".into(),
        label_enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    // between two points at an angle
    let slanted = ui.show(Brace {
        x1: 240.0,
        y1: 900.0,
        x2: 640.0,
        y2: 640.0,
        depth: 30.0,
        enter,
        ..ui.defaults()
    });
    let (a, b) = (
        ui.show(Dot {
            cx: 240.0,
            cy: 900.0,
            ..ui.defaults()
        }),
        ui.show(Dot {
            cx: 640.0,
            cy: 640.0,
            ..ui.defaults()
        }),
    );
    let circle = ui.show(Circle {
        cx: 1300.0,
        cy: 560.0,
        radius: 200.0,
        stroke: Some(muted),
        ..ui.defaults()
    });
    let outer = ArcBrace {
        cx: 1300.0,
        cy: 560.0,
        radius: 200.0,
        start_angle: 0.3,
        angle: 1.6,
        depth: 30.0,
        enter,
        ..ui.defaults()
    };
    let inner = ArcBrace {
        start_angle: 3.6,
        angle: 1.4,
        inside: true,
        ..outer.clone()
    };
    let labels: Vec<Svgr<'a>> = [(&outer, "ArcBrace"), (&inner, "inside")]
        .into_iter()
        .map(|(brace, text)| {
            let (x, y) = brace.tip();
            // beyond the tip: outward, or toward the center for the inside brace
            let side = if brace.inside { -1.0 } else { 1.0 };
            ui.show(Label {
                text: text.into(),
                x,
                y,
                direction: Anchor::toward(
                    side * f64::from(x - 1300.0),
                    side * f64::from(y - 560.0),
                ),
                enter: Some(FadeIn::default()),
                ..ui.defaults()
            })
        })
        .collect();
    let (outer, inner) = (ui.show(outer), ui.show(inner));
    let captions: Vec<Svgr<'a>> = [("Brace between two points", 440.0, 960.0)]
        .into_iter()
        .map(|(text, x, y)| caption(ui, text, x, y))
        .collect();
    svgr!(<g>{head}{line}{below}{above}{slanted}{a}{b}{circle}{outer}{inner}{labels}{captions}</g>)
}

fn boolean<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Boolean operations");
    let muted = ui.theme.colors.muted;
    let enter = Some(GrowFromCenter::default().into());
    let inputs = |x: f32| {
        let circle = Circle {
            cx: x - 40.0,
            cy: 520.0,
            radius: 110.0,
            ..Circle::default()
        }
        .path();
        let square = kurbo::Rect::new(f64::from(x) - 30.0, 450.0, f64::from(x) + 150.0, 630.0);
        (circle, kurbo::Shape::to_path(&square, 0.1))
    };
    let mut drawn: Vec<Svgr<'a>> = Vec::new();
    for (i, name) in ["Union", "Intersection", "Difference", "Exclusion"]
        .into_iter()
        .enumerate()
    {
        let x = 260.0 + i as f32 * 440.0;
        let (a, b) = inputs(x);
        drawn.push(match name {
            "Union" => ui.show(Union {
                paths: vec![a.clone(), b.clone()],
                enter,
                ..ui.defaults()
            }),
            "Intersection" => ui.show(Intersection {
                paths: vec![a.clone(), b.clone()],
                enter,
                ..ui.defaults()
            }),
            "Difference" => ui.show(Difference {
                subject: a.clone(),
                clip: b.clone(),
                enter,
                ..ui.defaults()
            }),
            _ => ui.show(Exclusion {
                subject: a.clone(),
                clip: b.clone(),
                enter,
                ..ui.defaults()
            }),
        });
        for path in [a, b] {
            drawn.push(ui.show(DashedVMobject {
                path,
                num_dashes: 40,
                stroke: Some(muted),
                ..ui.defaults()
            }));
        }
        drawn.push(caption(ui, name, x + 20.0, 720.0));
    }
    svgr!(<g>{head}{drawn}</g>)
}

/// The sample drawing, parsed once.
fn lighthouse() -> &'static SvgShape {
    static LIGHTHOUSE: OnceLock<SvgShape> = OnceLock::new();
    LIGHTHOUSE.get_or_init(|| SvgShape::parse(include_str!("lighthouse.svg")).unwrap_or_default())
}

fn svg_shape<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "SVG shapes");
    let c = ui.theme.colors;
    let write = Some(Write::default().into());
    let file = ui.show(SvgShape {
        cx: 300.0,
        cy: 520.0,
        height: Some(400.0),
        enter: write,
        ..lighthouse().clone()
    });
    let painted = lighthouse()
        .clone()
        .fill_part("stripes", c.accent)
        .fill_part("roof", c.accent)
        .fill_part("light", c.highlight);
    let painted = ui.show(SvgShape {
        cx: 760.0,
        cy: 520.0,
        height: Some(400.0),
        enter: write,
        ..painted
    });
    let tower = lighthouse()
        .clone()
        .only(|_, p| !matches!(p.id.as_str(), "sea" | "beam"));
    let tower = ui.show(SvgShape {
        cx: 1180.0,
        cy: 520.0,
        height: Some(400.0),
        stroke: None,
        enter: write,
        ..tower.stroke_part("tower", c.muted)
    });
    let heart = paths::parse_path(
        "M1600 420 C1600 360 1680 340 1700 400 C1720 340 1800 360 1800 420 C1800 500 1700 560 1700 600 C1700 560 1600 500 1600 420 Z",
    )
    .unwrap_or_default();
    let heart = ui.show(SvgPath {
        path: heart,
        enter: Some(create()),
        ..ui.defaults()
    });
    let captions: Vec<Svgr<'a>> = [
        ("SvgShape", 300.0, 790.0),
        ("fill_part(\"stripes\", ..)", 760.0, 790.0),
        ("only(..), stroke_part", 1180.0, 790.0),
        ("SvgPath", 1700.0, 790.0),
    ]
    .into_iter()
    .map(|(text, x, y)| caption(ui, text, x, y))
    .collect();
    svgr!(<g>{head}{file}{painted}{tower}{heart}{captions}</g>)
}
