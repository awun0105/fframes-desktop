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

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("Projectile", 3.5, projectile),
        GalleryScene::new("Pendulum", 6.0, pendulum),
        // the motion starts at 0.5 s and the axes end at 5 s of it
        GalleryScene::new("SpringMass", 5.5, spring_mass),
    ]
}
