//! The physics kit's figures, in the order of the plan's table. The motions without a
//! closed form are stepped once into a `OnceLock`, as a video does in its constructor.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

fn create(delay: f32) -> EnterAnim {
    Create {
        delay,
        ..Create::default()
    }
    .into()
}

fn fade(delay: f32) -> FadeIn {
    FadeIn {
        delay,
        ..FadeIn::default()
    }
}

/// A ball thrown at 11 m/s and 54°, with its velocity, peak and range.
fn projectile<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Projectile");
    let figure = ui.show(Projectile {
        origin: (-6.0, -2.6),
        speed: 11.0,
        angle: 0.95,
        scale: 0.85,
        delay: 0.6,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A pendulum swinging without friction next to its energy bars.
fn pendulum<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTION: OnceLock<PendulumMotion> = OnceLock::new();
    let motion = MOTION.get_or_init(|| PendulumMotion::new(2.6, 0.9, 6.0));
    let delay = 0.6;
    let t = f64::from((ui.seconds() - delay).max(0.0));
    let (kinetic, potential) = motion.energy(t, 1.0);
    let head = title(ui, "Pendulum");
    let pendulum = ui.show(Pendulum {
        motion: Some(motion),
        pivot: (-3.0, 2.6),
        delay,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    let bars = ui.show(EnergyBars {
        kinetic,
        potential,
        max: motion.energy(0.0, 1.0).1 * 1.1,
        x: 1240.0,
        y: 860.0,
        height: 480.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{pendulum}{bars}</g>)
}

/// A damped block on a spring above the graph of its displacement.
fn spring_mass<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTION: OnceLock<SpringMotion> = OnceLock::new();
    let motion = MOTION.get_or_init(|| SpringMotion::damped(20.0, 1.0, 0.6, 1.0, 6.0));
    let head = title(ui, "SpringMass");
    let axes = Axes {
        x_range: Range::new(0.0, 5.0, 1.0),
        y_range: Range::new(-1.0, 1.0, 0.5),
        x_length: 10.0,
        y_length: 2.6,
        center: (0.0, -2.0),
        tips: false,
        x_numbers: (0..=5).map(f64::from).collect(),
        enter: Some(create(0.0)),
        ..ui.defaults()
    };
    let spring = ui.show(SpringMass {
        motion: Some(motion),
        wall: (-5.5, 1.4),
        rest_length: 3.5,
        delay: 0.5,
        plot: Some(axes.map(ui)),
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    svgr!(<g>{head}{axes}{spring}</g>)
}

/// The four kinds of wave, one per row, each with its name above its left end.
fn wave<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Wave");
    let rows = [
        (WaveKind::Transverse, "transverse", 1.75),
        (WaveKind::Longitudinal, "longitudinal", 0.25),
        (WaveKind::Standing, "standing", -1.25),
        (WaveKind::Superposition, "superposition", -2.85),
    ];
    let mut parts = Vec::new();
    for (i, (kind, name, y)) in rows.into_iter().enumerate() {
        let delay = 0.3 * i as f32;
        let origin = (-6.0, y);
        let at = ui.space.to_px(origin.0, origin.1 + 0.55);
        parts.push(ui.show(Label {
            text: name.into(),
            x: at.x as f32,
            y: at.y as f32,
            direction: Anchor::TopRight,
            style: ui.theme.typography.caption,
            fill: ui.theme.colors.muted,
            enter: Some(fade(delay)),
            ..ui.defaults()
        }));
        parts.push(ui.show(Wave {
            kind,
            origin,
            length: 12.0,
            amplitude: if kind == WaveKind::Standing {
                0.22
            } else {
                0.4
            },
            wavelength: 3.0,
            speed: 3.0,
            second_wavelength: 2.2,
            second_amplitude: 0.25,
            marker: (kind != WaveKind::Standing).then_some(6.0),
            particles: 56,
            band: 0.7,
            delay: delay + 0.3,
            enter: Some(create(delay)),
            parts_enter: Some(fade(delay + 0.2)),
            ..ui.defaults()
        }));
    }
    svgr!(<g>{head}{parts}</g>)
}

/// A block pushed to the right against friction, with its four forces and their sum.
fn free_body<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "FreeBody");
    let figure = ui.show(FreeBody {
        forces: vec![
            Force::new("W", 0.0, -5.0),
            Force::new("N", 0.0, 5.0),
            Force::new("F", 6.0, 0.0),
            Force::new("f", -2.0, 0.0),
        ],
        center: (0.0, -0.4),
        scale: 0.45,
        delay: 0.6,
        enter: Some(create(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A planet on an orbit of eccentricity 0.6 sweeping three equal areas.
fn orbit<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Orbit");
    let figure = ui.show(Orbit {
        focus: (2.0, -0.5),
        semi_major: 4.0,
        eccentricity: 0.6,
        period: 5.0,
        sweeps: 3,
        sweep: 0.12,
        delay: 0.6,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// The field of a dipole drawing on, then a test charge following one line.
fn field_lines<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static FIELD: OnceLock<Field> = OnceLock::new();
    let field = FIELD.get_or_init(|| {
        Field::new(
            vec![Charge::new(-2.5, -0.5, 1.0), Charge::new(2.5, -0.5, -1.0)],
            16,
            (-7.3, -4.1, 7.3, 2.5),
        )
    });
    let head = title(ui, "FieldLines");
    let figure = ui.show(FieldLines {
        field: Some(field),
        delay: 0.5,
        draw: 1.5,
        test_charge: Some(2),
        speed: 2.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A battery, switch, bulb and resistor in series: the switch closes, the current flows and
/// the bulb lights.
fn circuit<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Circuit");
    let figure = ui.show(Circuit {
        center: (0.0, -0.4),
        width: 8.0,
        height: 4.5,
        on_at: 1.4,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A converging lens forming a real, inverted image.
fn ray_diagram<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "RayDiagram");
    let figure = ui.show(RayDiagram {
        center: (0.5, -0.4),
        focal: 1.8,
        object_distance: 4.0,
        object_height: 1.2,
        reach: 5.5,
        delay: 0.8,
        enter: Some(create(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A concave mirror sending the rays back to a real image in front of it.
fn mirror<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Mirror");
    let figure = ui.show(RayDiagram {
        optic: Optic::Mirror,
        center: (4.0, -0.4),
        focal: 1.6,
        object_distance: 4.5,
        object_height: 1.2,
        reach: 5.5,
        delay: 0.8,
        enter: Some(create(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Light bending into glass, with its weak reflection.
fn refraction<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Refraction");
    let figure = ui.show(Refraction {
        point: (0.0, -0.6),
        n1: 1.0,
        n2: 1.5,
        angle: 0.75,
        length: 3.0,
        delay: 0.7,
        enter: Some(create(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A heavy cart running into a light one, elastically, above the momentum bars.
fn collision<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Collision1D");
    let figure = ui.show(Collision1D {
        track: (-6.0, 0.2),
        x1: 1.5,
        x2: 5.5,
        u1: 2.0,
        u2: -0.5,
        delay: 0.4,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A source moving right at a third of the wave speed: the fronts bunch up ahead of it, and
/// the listener ahead hears a higher pitch than the one behind.
fn doppler<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Doppler");
    let figure = ui.show(Doppler {
        start: (-3.5, -0.6),
        scale: 0.7,
        ahead: Some(8.0),
        behind: Some(-4.0),
        max_radius: 10.0,
        delay: 0.5,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Sixty particles of a gas, heated to three times the temperature after three seconds.
fn gas_box<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTION: OnceLock<GasMotion> = OnceLock::new();
    let motion = MOTION.get_or_init(|| GasMotion::new(gas_setup()));
    let head = title(ui, "GasBox");
    let figure = ui.show(GasBox {
        motion: Some(motion),
        center: (-0.6, -0.5),
        scale: 1.4,
        delay: 0.5,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// The gas of the `GasBox` scene.
fn gas_setup() -> GasSetup {
    GasSetup {
        seed: 7,
        count: 60,
        width: 6.0,
        height: 4.0,
        radius: 0.1,
        speed: 1.5,
        seconds: 7.0,
        heat: Some(Heat {
            at: 2.5,
            to: 3.0,
            over: 1.5,
        }),
    }
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("Projectile", 3.5, projectile),
        GalleryScene::new("Pendulum", 6.0, pendulum),
        // the motion starts at 0.5 s and the axes end at 5 s of it
        GalleryScene::new("SpringMass", 5.5, spring_mass),
        GalleryScene::new("Wave", 5.0, wave),
        GalleryScene::new("FreeBody", 4.5, free_body),
        GalleryScene::new("Orbit", 6.0, orbit),
        GalleryScene::new("FieldLines", 6.0, field_lines),
        GalleryScene::new("Circuit", 5.0, circuit),
        GalleryScene::new("RayDiagram", 4.5, ray_diagram),
        GalleryScene::new("Mirror", 4.5, mirror),
        GalleryScene::new("Refraction", 3.5, refraction),
        // B reaches the end of the track just after the scene
        GalleryScene::new("Collision1D", 3.8, collision),
        GalleryScene::new("Doppler", 5.5, doppler),
        GalleryScene::new("GasBox", 7.0, gas_box),
    ]
}
