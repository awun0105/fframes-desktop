//! The SVG-filter effects, one scene per group: every effect of the group on the same
//! picture, in a grid of labelled thumbnails.

use fframes_kit::prelude::*;

use crate::{GalleryScene, scenes::core::title};

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the hue turns and the tint fades in over the scene
        GalleryScene::new("ColorEffects", 2.0, color_effects),
    ]
}

/// A colourful picture: a sky, a sun, hills, a card and a word.
fn photo<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let s = ui.theme.typography.title;
    svgr!(
        <g>
            <defs>
                <linearGradient id="photo-sky" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0" stop-color="#1d4e89" />
                    <stop offset="0.6" stop-color="#f08a4b" />
                    <stop offset="1" stop-color="#f6d365" />
                </linearGradient>
            </defs>
            <rect x="0" y="0" width="1920" height="1080" fill="url(#photo-sky)" />
            <circle cx="1380" cy="420" r="170" fill="#ffe29a" />
            <path d="M0 760 C 320 600 620 640 900 760 S 1500 860 1920 700 L 1920 1080 L 0 1080 Z" fill="#2a9d8f" />
            <path d="M0 900 C 400 820 800 860 1100 940 S 1600 1000 1920 900 L 1920 1080 L 0 1080 Z" fill="#264653" />
            <rect x="260" y="380" width="360" height="240" rx="24" fill="#e63946" />
            <rect x="1560" y="760" width="200" height="200" rx="16" fill="#00c853" />
            <text x="440" y="540" text-anchor="middle" font-family={s.family}
                  font-size={s.size * 2} font-weight={s.weight} fill="#ffffff">
                "kit"
            </text>
        </g>
    )
}

/// `cells` as a grid of five columns of labelled thumbnails under the title.
fn grid<'a>(ui: &Ui<'a, '_, '_>, cells: Vec<(&'static str, Svgr<'a>)>) -> Svgr<'a> {
    const COLUMNS: usize = 5;
    const SCALE: f32 = 0.17;
    let c = ui.theme.colors;
    let s = ui.theme.typography.caption;
    let thumbs: Vec<Svgr<'a>> = cells
        .into_iter()
        .enumerate()
        .map(|(i, (label, cell))| {
            let x = 96.0 + (i % COLUMNS) as f32 * 352.0;
            let y = 190.0 + (i / COLUMNS) as f32 * 222.0;
            let place = Transform::translate(x, y);
            let shrink = Transform::scale(SCALE);
            svgr!(
                <g transform={place}>
                    <g transform={shrink}>{cell}</g>
                    <text x="0" y="208" font-family={s.family} font-size={s.size}
                          font-weight={s.weight} fill={c.text}>
                        {label}
                    </text>
                </g>
            )
        })
        .collect();
    svgr!(<g>{thumbs}</g>)
}

fn color_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let turn = 90.0 + 90.0 * t;
    let cells = vec![
        (
            "Brightness",
            ui.show(Brightness {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Contrast",
            ui.show(Contrast {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Saturation",
            ui.show(Saturation {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "HueRotate",
            ui.show(HueRotate {
                degrees: Anim::Fixed(turn),
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Grayscale",
            ui.show(Grayscale {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Invert",
            ui.show(Invert {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Tint",
            ui.show(Tint {
                amount: Anim::Fixed(0.4 + 0.3 * t),
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Duotone",
            ui.show(Duotone {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Levels",
            ui.show(Levels {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Curves",
            ui.show(Curves {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Exposure",
            ui.show(Exposure {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "ColorCorrection",
            ui.show(ColorCorrection {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "WhiteBalance",
            ui.show(WhiteBalance {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Vibrance",
            ui.show(Vibrance {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "ShadowsHighlights",
            ui.show(ShadowsHighlights {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        ("ColorKey", color_key(ui)),
        (
            "ThermalVision",
            ui.show(ThermalVision {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Lut",
            ui.show(Lut {
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "ColorStack",
            ui.show(ColorStack {
                matrices: vec![
                    ColorMatrix::exposure(0.3),
                    ColorMatrix::contrast(1.2),
                    ColorMatrix::white_balance(-0.6, 0.0),
                ],
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        ("original", photo(ui)),
    ];
    let figure = grid(ui, cells);
    let head = title(ui, "Colour effects");
    svgr!(<g>{figure}{head}</g>)
}

/// The green square keyed out over a striped backdrop, so the hole shows.
fn color_key<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let keyed = ui.show(ColorKey {
        children: children(photo),
        ..ui.defaults()
    });
    let stripes: Vec<Svgr<'a>> = (0..12)
        .map(|i| {
            let x = i as f32 * 160.0;
            svgr!(<rect x={x} y="0" width="80" height="1080" fill="#ffffff" />)
        })
        .collect();
    svgr!(
        <g>
            <rect x="0" y="0" width="1920" height="1080" fill="#8d99ae" />
            {stripes}
            {keyed}
        </g>
    )
}
