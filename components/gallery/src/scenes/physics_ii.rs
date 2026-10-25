//! The physics kit's second set: fields of charges and currents, lenses, rigid bodies,
//! waves and circuit symbols.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use super::core::{hue, neon, title};
use crate::GalleryScene;

fn fade(delay: f32) -> FadeIn {
    FadeIn {
        delay,
        ..FadeIn::default()
    }
}

/// A dipole whose negative charge circles the positive one, the arrows and lines following.
fn electric_field<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ElectricField");
    let angle = 0.6 * f64::from(ui.seconds());
    let figure = ui.show(ElectricField {
        charges: vec![
            Charge::new(-1.2, -0.3, 2.0),
            Charge::new(-1.2 + 3.0 * angle.cos(), -0.3 + 2.0 * angle.sin(), -1.0),
        ],
        x_range: Range::new(-6.5, 6.5, 0.65),
        y_range: Range::new(-3.5, 2.7, 0.62),
        lines_per_charge: 8,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// Two long wires through the page with opposite currents, and a square loop in the page.
fn magnetic_field<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "MagneticField");
    let (x0, x1, y0, y1) = (1.5, 5.0, -2.6, 1.6);
    let loop_current = 2.5;
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let mut wires: Vec<Wire> = vec![
        Wire::Through {
            x: -4.6,
            y: -0.5,
            current: 1.0,
        },
        Wire::Through {
            x: -1.6,
            y: -0.5,
            current: -1.0,
        },
    ];
    // counterclockwise: out of the page inside the loop
    for i in 0..4 {
        wires.push(Wire::Segment {
            from: corners[i],
            to: corners[(i + 1) % 4],
            current: loop_current,
        });
    }
    let figure = ui.show(MagneticField {
        wires,
        x_range: Range::new(-6.5, 6.5, 0.65),
        y_range: Range::new(-3.5, 2.7, 0.62),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// Parallel light through a thick biconvex lens, then a thin diverging one.
fn lens<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Lens");
    let rays = (0..7)
        .map(|i| Ray::new((-6.8, -1.3 + 0.3 * f64::from(i)), (1.0, 0.0)))
        .collect();
    let figure = ui.show(OpticalBench {
        lenses: vec![
            Lens::thick(-3.0, 2.4, -2.4, 1.5, 0.5),
            Lens::thin(2.6, -2.0),
        ],
        rays,
        axis_y: -0.4,
        x_range: (-7.0, 7.0),
        enter: Some(Create::default().into()),
        parts_enter: Some(fade(0.3)),
        delay: 0.6,
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// A double and a triple pendulum released high, the last bobs leaving trails.
fn multi_pendulum<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static DOUBLE: OnceLock<MultiPendulumMotion> = OnceLock::new();
    static TRIPLE: OnceLock<MultiPendulumMotion> = OnceLock::new();
    let double =
        DOUBLE.get_or_init(|| MultiPendulumMotion::new(&[1.4, 1.1], &[1.0, 1.0], &[2.2, 2.6], 4.0));
    let triple = TRIPLE.get_or_init(|| {
        MultiPendulumMotion::new(&[1.0, 0.9, 0.8], &[1.0, 1.0, 1.0], &[1.8, 2.4, 2.9], 4.0)
    });
    let head = title(ui, "MultiPendulum");
    let double = ui.show(MultiPendulum {
        motion: Some(double),
        pivot: (-3.2, 0.6),
        enter: Some(Create::default().into()),
        parts_enter: Some(fade(0.2)),
        delay: 0.5,
        ..ui.defaults()
    });
    let triple = ui.show(MultiPendulum {
        motion: Some(triple),
        pivot: (3.2, 0.6),
        enter: Some(Create::default().into()),
        parts_enter: Some(fade(0.2)),
        delay: 0.5,
        ..ui.defaults()
    });
    svgr!(<g>{double}{triple}{head}</g>)
}

/// How many pendulums [`chaos_fan`] swings, and how far apart their first links start.
const FAN: usize = 24;
const FAN_SPREAD: f64 = 0.002;

/// Twenty-four double pendulums released a thousandth of a radian apart: they swing as one,
/// then chaos fans them out into a rainbow.
fn chaos_fan<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTIONS: OnceLock<Vec<MultiPendulumMotion>> = OnceLock::new();
    let motions = MOTIONS.get_or_init(|| {
        (0..FAN)
            .map(|i| {
                let angle = 2.5 + FAN_SPREAD * i as f64;
                MultiPendulumMotion::new(&[1.25, 1.05], &[1.0, 1.0], &[angle, 2.9], 7.0)
            })
            .collect()
    });
    let head = title(ui, "ChaosFan");
    let pendulums: Vec<Svgr<'a>> = motions
        .iter()
        .enumerate()
        .map(|(i, motion)| {
            let color = hue(i as f32 / FAN as f32);
            ui.show(MultiPendulum {
                motion: Some(motion),
                pivot: (0.0, 0.3),
                delay: 0.5,
                bob_radius: 9.0,
                trail_time: Some(1.2),
                color: color.with_alpha(110),
                bob_color: color,
                trail_color: color,
                stroke_width: 2.0,
                parts_enter: Some(fade(0.0)),
                ..ui.defaults()
            })
        })
        .collect();
    let glow = neon("gallery-chaos-glow", 6.0);
    let c = ui.theme.colors;
    svgr!(
        <g>
            <defs>{glow}</defs>
            {head}
            <g filter="url(#gallery-chaos-glow)">{pendulums}</g>
            <circle cx="960" cy={540.0 - 0.3 * 135.0} r="10" fill={c.title} />
        </g>
    )
}

/// Balls, boxes and polygons dropped into a box with a ramp, bouncing and settling.
fn rigid_world<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTION: OnceLock<RigidMotion> = OnceLock::new();
    let (accent, highlight) = (ui.theme.colors.accent, ui.theme.colors.highlight);
    let motion = MOTION.get_or_init(|| {
        let mut bodies = vec![
            Body::wall((-6.6, -3.4), (6.6, -3.4), 0.3),
            Body::wall((-6.6, -3.4), (-6.6, 2.2), 0.3),
            Body::wall((6.6, -3.4), (6.6, 2.2), 0.3),
            Body::wall((-6.2, 0.4), (-2.2, -1.2), 0.25),
        ];
        for i in 0..6 {
            let x = -5.4 + 0.75 * f64::from(i);
            bodies.push(Body::circle(x, 1.6 + 0.4 * f64::from(i % 2), 0.32).color(highlight));
        }
        for i in 0..4 {
            let x = 0.6 + 1.3 * f64::from(i);
            bodies
                .push(Body::rect(x, 1.2 + 0.7 * f64::from(i), 0.9, 0.6).angle(0.25 * f64::from(i)));
        }
        let corners = |n: u32, r: f64| {
            (0..n).map(move |k| {
                let a = std::f64::consts::TAU * f64::from(k) / f64::from(n) + 0.3;
                (r * a.cos(), r * a.sin())
            })
        };
        bodies.push(
            Body::polygon(2.6, 3.0, corners(3, 0.55))
                .spin(2.0)
                .color(accent),
        );
        bodies.push(
            Body::polygon(4.8, 3.2, corners(5, 0.5))
                .restitution(0.8)
                .color(highlight),
        );
        bodies.push(
            Body::circle(-0.6, 3.0, 0.5)
                .velocity(2.5, 0.0)
                .restitution(0.9),
        );
        RigidMotion::new(bodies, (0.0, -9.81), 4.5)
    });
    let head = title(ui, "RigidWorld");
    let figure = ui.show(RigidWorld {
        motion: Some(motion),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// Two sources a few wavelengths apart: their rings cross where the waves add up.
fn radial_wave<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "RadialWave");
    let figure = ui.show(RadialWave {
        sources: vec![(-1.6, -0.4), (1.6, -0.4)],
        wavelength: 1.1,
        speed: 1.6,
        x_range: (-7.0, 7.0),
        y_range: (-3.8, 2.8),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// The same two sources as a surface, the camera turning slowly.
#[cfg(feature = "three-d")]
fn radial_wave_3d<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "RadialWave3D");
    let wave = RadialWave {
        sources: vec![(-1.2, 0.0), (1.2, 0.0)],
        wavelength: 1.0,
        speed: 1.2,
        amplitude: 0.18,
        front: false,
        x_range: (-4.0, 4.0),
        y_range: (-4.0, 4.0),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    };
    let objects: Vec<Box<dyn Object3D + 'a>> = vec![Box::new(wave)];
    let figure = ui.show(ThreeDView {
        phi: 60f32.to_radians(),
        theta: -60f32.to_radians(),
        ambient_rotation: 0.1,
        objects,
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// A lamp circuit whose switch closes, and an inverting amplifier beside it.
fn schematic<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Schematic");
    let theme = ui.theme.clone();
    let part = |kind, at: (f64, f64), rotation: f64, label: &'a str| Symbol {
        kind,
        at,
        rotation,
        label,
        ..Symbol::themed(&theme)
    };
    let symbols = vec![
        // the lamp circuit
        part(SymbolKind::Battery, (-6.0, -0.4), 90.0, "9 V"),
        part(
            SymbolKind::Switch {
                closes_at: Some(1.0),
            },
            (-4.6, 1.8),
            0.0,
            "",
        ),
        part(SymbolKind::Resistor, (-2.6, 1.8), 0.0, "220R"),
        part(SymbolKind::Bulb, (-0.8, -0.4), 90.0, ""),
        part(SymbolKind::Diode, (-3.4, -2.6), 180.0, ""),
        part(SymbolKind::Ground, (-6.0, -2.6), 0.0, ""),
        // the amplifier
        part(SymbolKind::Opamp, (3.8, -0.2), 0.0, ""),
        part(SymbolKind::Resistor, (1.6, 0.04), 0.0, "10k"),
        part(SymbolKind::Resistor, (3.8, 1.4), 0.0, "100k"),
        part(SymbolKind::VoltageSource, (0.4, -1.0), 90.0, ""),
        part(SymbolKind::Ground, (0.4, -2.6), 0.0, ""),
        part(SymbolKind::Ground, (3.2, -1.4), 0.0, ""),
        // the other parts
        part(SymbolKind::Capacitor, (2.4, -3.1), 0.0, ""),
        part(SymbolKind::Inductor, (4.2, -3.1), 0.0, ""),
        part(SymbolKind::CurrentSource, (6.0, -3.1), 0.0, ""),
    ];
    let still = |c: Connection<'a>| c.current(0.0);
    let wires = vec![
        Connection::new((0, "+"), (1, "a")).via([(-6.0, 1.8)]),
        Connection::new((1, "b"), (2, "a")),
        Connection::new((2, "b"), (3, "b")).via([(-0.8, 1.8)]),
        Connection::new((3, "a"), (4, "a")).via([(-0.8, -2.6)]),
        Connection::new((4, "b"), (5, "a")),
        Connection::new((5, "a"), (0, "-")),
        still(Connection::new((9, "+"), (7, "a")).via([(0.4, 0.04)])),
        still(Connection::new((9, "-"), (10, "a"))),
        still(Connection::new((7, "b"), (6, "-"))),
        still(Connection::new((6, "-"), (8, "a")).via([(3.2, 1.4)])),
        still(Connection::new((8, "b"), (5.0, -0.2)).via([(5.0, 1.4)])),
        still(Connection::new((6, "out"), (6.2, -0.2))),
        still(Connection::new((6, "+"), (11, "a"))),
    ];
    let figure = ui.show(Schematic {
        symbols,
        wires,
        current_at: Some(1.3),
        enter: Some(Create::default().into()),
        parts_enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    #[allow(unused_mut)]
    let mut scenes = vec![
        GalleryScene::new("ElectricField", 4.0, electric_field),
        GalleryScene::new("MagneticField", 2.0, magnetic_field),
        GalleryScene::new("Lens", 3.5, lens),
        GalleryScene::new("RigidWorld", 4.0, rigid_world),
        GalleryScene::new("MultiPendulum", 4.0, multi_pendulum),
        GalleryScene::new("ChaosFan", 6.5, chaos_fan),
        GalleryScene::new("RadialWave", 4.0, radial_wave),
        GalleryScene::new("Schematic", 3.5, schematic),
    ];
    #[cfg(feature = "three-d")]
    scenes.push(GalleryScene::new("RadialWave3D", 2.5, radial_wave_3d));
    scenes
}
