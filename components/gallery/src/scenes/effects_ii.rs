//! The SVG-filter effects, one scene per group: every effect of the group on the same
//! picture, in a grid of labelled thumbnails.

use fframes_kit::{motion, prelude::*};

use crate::{GalleryScene, scenes::core::title};

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the hue turns and the tint fades in over the scene
        GalleryScene::new("ColorEffects", 2.0, color_effects),
        // the outline grows and the long shadows stretch out over the scene
        GalleryScene::new("EdgeEffects", 2.0, edge_effects),
        // the static flickers, the clouds evolve and the dots ride the noise
        GalleryScene::new("NoiseEffects", 2.0, noise_effects),
        // the scanlines roll, the grid turns and the rings expand over the scene
        GalleryScene::new("OverlayEffects", 2.0, overlay_effects),
        // the staggered pattern slides over the scene
        GalleryScene::new("TextureEffects", 2.0, texture_effects),
        // paper on its own, large enough for its fibres
        GalleryScene::new("PaperEffects", 1.0, paper_effects),
        // the starburst turns over the scene
        GalleryScene::new("HalftoneEffects", 2.0, halftone_effects),
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

/// `cells` as a grid of `columns` labelled thumbnails under the title.
fn grid<'a>(ui: &Ui<'a, '_, '_>, columns: usize, cells: Vec<(&'static str, Svgr<'a>)>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let s = ui.theme.typography.caption;
    let pitch = 1760.0 / columns as f32;
    let scale = (pitch - 32.0) / 1920.0;
    let row = 1080.0 * scale + 52.0;
    let thumbs: Vec<Svgr<'a>> = cells
        .into_iter()
        .enumerate()
        .map(|(i, (label, cell))| {
            let x = 96.0 + (i % columns) as f32 * pitch;
            let y = 190.0 + (i / columns) as f32 * row;
            let place = Transform::translate(x, y);
            let shrink = Transform::scale(scale);
            let label_y = 1080.0 * scale + 38.0;
            svgr!(
                <g transform={place}>
                    <g transform={shrink}>{cell}</g>
                    <text x="0" y={label_y} font-family={s.family} font-size={s.size}
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
    let figure = grid(ui, 5, cells);
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

/// A logo and a word on a transparent background, for the effects that work on alpha edges.
fn logo<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let s = ui.theme.typography.title;
    svgr!(
        <g>
            <circle cx="520" cy="540" r="250" fill="#e63946" />
            <path d="M520 360 L575 480 L705 490 L605 575 L640 705 L520 635 L400 705 L435 575 L335 490 L465 480 Z"
                  fill="#ffe29a" />
            <text x="1300" y="660" text-anchor="middle" font-family={s.family} font-size="340"
                  font-weight="700" fill="#2a9d8f">
                "kit"
            </text>
        </g>
    )
}

/// `effect` over a light backdrop.
fn on_backdrop(effect: Svgr<'_>) -> Svgr<'_> {
    svgr!(
        <g>
            <rect x="0" y="0" width="1920" height="1080" fill="#e9e4da" />
            {effect}
        </g>
    )
}

fn edge_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let p = motion::clamp01(ui.seconds() / 2.0);
    let cells = vec![
        (
            "Outline",
            ui.show(Outline {
                width: Anim::Fixed(4.0 + 12.0 * p),
                color: Color::rgb(38, 70, 83),
                children: children(logo),
                ..ui.defaults()
            }),
        ),
        (
            "Emboss",
            ui.show(Emboss {
                children: children(logo),
                ..ui.defaults()
            }),
        ),
        (
            "BevelAlpha",
            ui.show(BevelAlpha {
                edge_thickness: Anim::Fixed(14.0),
                light_intensity: 0.7,
                children: children(logo),
                ..ui.defaults()
            }),
        ),
        (
            "LongShadows",
            ui.show(LongShadows {
                length: Anim::Fixed(60.0 + 180.0 * p),
                children: children(logo),
                ..ui.defaults()
            }),
        ),
        (
            "RoughenEdges",
            ui.show(RoughenEdges {
                children: children(logo),
                ..ui.defaults()
            }),
        ),
        (
            "Shrinkwrap",
            ui.show(Shrinkwrap {
                children: children(logo),
                ..ui.defaults()
            }),
        ),
        ("original", logo(ui)),
    ]
    .into_iter()
    .map(|(label, cell)| (label, on_backdrop(cell)))
    .collect();
    let figure = grid(ui, 4, cells);
    let head = title(ui, "Edges and shape");
    svgr!(<g>{figure}{head}</g>)
}

/// Dots riding 2D simplex noise over time, and a 3D noise field as a row of bars.
fn noise_motion<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let dots: Vec<Svgr<'a>> = (0..24)
        .map(|i| {
            let x = 120.0 + i as f32 * 73.0;
            let y = 380.0 + 220.0 * motion::noise_2d("dots", i as f32 * 0.21, t * 0.9);
            svgr!(<circle cx={x} cy={y} r="22" fill="#e63946" />)
        })
        .collect();
    let bars: Vec<Svgr<'a>> = (0..48)
        .map(|i| {
            let h = 160.0 + 140.0 * motion::noise_3d(7, i as f32 * 0.12, 0.0, t * 0.7);
            let x = 96.0 + i as f32 * 36.0;
            svgr!(<rect x={x} y={1000.0 - h} width="26" height={h} fill="#2a9d8f" />)
        })
        .collect();
    svgr!(
        <g>
            <rect x="0" y="0" width="1920" height="1080" fill="#e9e4da" />
            {dots}
            {bars}
        </g>
    )
}

fn noise_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let frame = ui.frame.index as u32;
    let cells = vec![
        (
            "WhiteNoise",
            ui.show(WhiteNoise {
                amount: Anim::Fixed(0.5),
                seed: frame,
                // specks of about five output pixels in a fifth-size thumbnail of a half-size render
                grain: 48.0,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "FractalNoise",
            ui.show(FractalNoise {
                amount: Anim::Fixed(0.7),
                evolution: Anim::Fixed(180.0 * t),
                key: Some("photo"),
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "FractalNoise turbulent",
            on_backdrop(ui.show(FractalNoise {
                fractal_type: FractalType::Turbulent,
                scale: 220.0,
                complexity: 3,
                offset_x: 120.0 * t,
                key: Some("logo"),
                children: children(logo),
                ..ui.defaults()
            })),
        ),
        ("Noise", {
            let picture = photo(ui);
            let grain = ui.show(Noise {
                opacity: 0.3,
                ..ui.defaults()
            });
            svgr!(<g>{picture}{grain}</g>)
        }),
        ("noise_2d, noise_3d", noise_motion(ui)),
    ];
    let figure = grid(ui, 4, cells);
    let head = title(ui, "Noise");
    svgr!(<g>{figure}{head}</g>)
}

fn overlay_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let stripes = vec![
        Color::rgba(255, 255, 255, 150),
        Color::rgba(29, 78, 137, 150),
    ];
    let cells = vec![
        (
            "Scanlines",
            ui.show(Scanlines {
                amount: Anim::Fixed(0.5),
                spacing: 36.0,
                thickness: 14.0,
                offset: 36.0 * t,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "DotGrid",
            ui.show(DotGrid {
                dot_size: 44.0,
                grid_size: 56.0,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "DotGrid invert",
            on_backdrop(ui.show(DotGrid {
                dot_size: 44.0,
                grid_size: 56.0,
                invert: true,
                key: Some("invert"),
                children: children(photo),
                ..ui.defaults()
            })),
        ),
        (
            "Checkerboard",
            on_backdrop(ui.show(Checkerboard {
                cell_size: 90.0,
                gap: 14.0,
                angle: 15.0,
                mask_to_source_alpha: true,
                children: children(logo),
                ..ui.defaults()
            })),
        ),
        (
            "Gridlines",
            ui.show(Gridlines {
                grid_size: 140.0,
                line_width: 10.0,
                rotation: 10.0 * t,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Lines",
            ui.show(Lines {
                colors: stripes,
                thickness: 50.0,
                gap: 40.0,
                angle: -30.0,
                offset: 45.0 * t,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Rings",
            on_backdrop(ui.show(Rings {
                gap: 24.0,
                offset: 64.0 * t,
                mask_to_source_alpha: true,
                children: children(logo),
                ..ui.defaults()
            })),
        ),
        (
            "LinearGradientTint",
            ui.show(LinearGradientTint {
                start_color: Color::rgb(230, 57, 70),
                end_color: Color::rgb(29, 78, 137),
                amount: Anim::Fixed(0.6),
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "LinearGradient",
            ui.show(LinearGradient {
                start: (0.0, 0.0),
                end: (1.0, 1.0),
                start_color: Color::rgb(42, 157, 143),
                end_color: Color::rgb(246, 211, 101),
                ..ui.defaults()
            }),
        ),
        (
            "Vignette",
            ui.show(Vignette {
                amount: Anim::Fixed(0.9),
                radius: 0.4,
                feather: 0.5,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Vignette square",
            ui.show(Vignette {
                amount: Anim::Fixed(1.0),
                radius: 0.6,
                feather: 0.3,
                roundness: 0.0,
                color: Color::rgb(230, 57, 70),
                key: Some("square"),
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Vignette alpha",
            on_backdrop(ui.show(Vignette {
                amount: Anim::Fixed(1.0),
                radius: 0.3,
                feather: 0.6,
                roundness: 0.5,
                mode: VignetteMode::Alpha,
                key: Some("alpha"),
                children: children(photo),
                ..ui.defaults()
            })),
        ),
    ];
    let figure = grid(ui, 4, cells);
    let head = title(ui, "Pattern overlays");
    svgr!(<g>{figure}{head}</g>)
}

fn texture_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let cells = vec![
        (
            "Pattern",
            ui.show(Pattern {
                scale: 0.25,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Pattern staggered",
            on_backdrop(ui.show(Pattern {
                scale: 0.2,
                gap_x: 40.0,
                gap_y: 40.0,
                row_offset: 200.0,
                row_offset_every: 2,
                offset_u: 0.05 * t,
                key: Some("staggered"),
                children: children(logo),
                ..ui.defaults()
            })),
        ),
        (
            "Speckle",
            on_backdrop(ui.show(Speckle {
                density: 0.25,
                size: 40.0,
                children: children(photo),
                ..ui.defaults()
            })),
        ),
        (
            "Flannel",
            ui.show(Flannel {
                size: 240.0,
                softness: 0.5,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Burlap",
            ui.show(Burlap {
                amount: Anim::Fixed(0.8),
                size: 40.0,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        ("original", photo(ui)),
    ];
    let figure = grid(ui, 3, cells);
    let head = title(ui, "Textures");
    svgr!(<g>{figure}{head}</g>)
}

/// Paper in two large cells: its fibres need more pixels than a small thumbnail has.
fn paper_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let cells = vec![
        (
            "Paper",
            ui.show(Paper {
                scale: 2.0,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Paper crumpled",
            ui.show(Paper {
                scale: 2.0,
                contrast: 0.5,
                crumples: 0.8,
                folds: 1.0,
                fold_count: 8,
                seed: 3,
                key: Some("crumpled"),
                children: children(photo),
                ..ui.defaults()
            }),
        ),
    ];
    let figure = grid(ui, 2, cells);
    let head = title(ui, "Paper");
    svgr!(<g>{figure}{head}</g>)
}

fn halftone_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let ink = Color::hex("#1d2b53");
    let cells = vec![
        (
            "Halftone",
            on_backdrop(ui.show(Halftone {
                dot_size: 64.0,
                dot_color: ink,
                children: children(photo),
                ..ui.defaults()
            })),
        ),
        (
            "Halftone squares",
            on_backdrop(ui.show(Halftone {
                shape: HalftoneShape::Square,
                dot_size: 72.0,
                rotation: 45.0,
                color: HalftoneColor::Source,
                key: Some("squares"),
                children: children(photo),
                ..ui.defaults()
            })),
        ),
        (
            "Halftone lines",
            on_backdrop(ui.show(Halftone {
                shape: HalftoneShape::Line,
                dot_size: 56.0,
                rotation: -20.0,
                invert: true,
                dot_color: ink,
                key: Some("lines"),
                children: children(photo),
                ..ui.defaults()
            })),
        ),
        (
            "Halftone gradient",
            ui.show(HalftoneLinearGradient {
                grid_size: 72.0,
                second_dot_size: 110.0,
                dot_color: ink,
                children: children(photo),
                ..ui.defaults()
            }),
        ),
        (
            "Starburst",
            ui.show(Starburst {
                rays: 14,
                colors: vec![
                    Color::hex("#ffcf5c"),
                    Color::hex("#ff7a59"),
                    Color::hex("#ffe9a8"),
                ],
                rotation: 12.0 * t,
                smoothness: 0.5,
                origin: (0.5, 0.65),
                ..ui.defaults()
            }),
        ),
        (
            "Light trail",
            svgr!(
                <g>
                    <rect x="0" y="0" width="1920" height="1080" fill="#14182b" />
                    {ui.show(LightTrail {
                        distance: Anim::Fixed(420.0),
                        intensity: 2.0,
                        color: Color::hex("#ffb347"),
                        children: children(logo),
                        ..ui.defaults()
                    })}
                </g>
            ),
        ),
    ];
    let figure = grid(ui, 3, cells);
    let head = title(ui, "Halftone and light");
    svgr!(<g>{figure}{head}</g>)
}
