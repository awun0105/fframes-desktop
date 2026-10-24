# fframes-kit: physics kit

The physics kit is the `physics` feature of `fframes-kit`
(`fframes-kit = { .., features = ["physics"] }`). Its figures come with the prelude and need
no other crates. Every entry names its scene in the components gallery
(`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

- **Placement.** A figure stands alone and is placed in units of `ui.space` (Manim's frame:
  8 units tall, `y` up, the origin in the middle). `scale` says how many units one meter
  is, so a 10 m throw fits on the canvas with `scale: 0.8`.
- **Units.** Quantities are SI: meters, seconds, kilograms, radians. `GRAVITY` is 9.81 m/s².
- **Time.** Each figure runs on its own clock: the local time minus `delay`, times
  `time_scale`. Before `delay` it shows its start.
- **Simulations.** A motion with a closed form (`Projectile`) is computed per frame. One
  without (`Pendulum`, `SpringMass`) is stepped once, at 240 steps per second with RK4, into
  a motion value built in the video's constructor (a `OnceLock` field, or a field set in
  `new`). The figure borrows it through `motion: Some(&..)` and draws nothing without it.
- **Animation props.** The fixed parts (ground, ceiling, wall) draw with `enter`; the moving
  parts fade with `parts_enter`.

## Projectile

A ball thrown at `speed` and `angle` without drag. Its path draws behind it, and it
carries its velocity and the velocity's two components. Once it passes them, the top of the
arc and the landing point are marked with the height and the range. `flight_time`,
`position`, `velocity`, `apex` and `range` give the numbers. Gallery scene: `Projectile`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Projectile {
        origin: (-6.0, -2.5),
        speed: 11.0,
        angle: 0.9,
        scale: 0.9,
        delay: 0.5,
        enter: Some(Create::default().into()),
        ..ui.defaults()
    })
}
```

## Pendulum and EnergyBars

`PendulumMotion::new(length, angle0, seconds)` swings a pendulum from rest without
friction; `PendulumMotion::damped(length, angle0, damping, seconds)` slows it. `Pendulum` draws it
hanging from `pivot`, with the reach of the swing as a dashed arc and the angle from the
vertical. `EnergyBars` shows kinetic energy, potential energy and their sum in pixels.
Feed it `motion.energy(t, mass)` at the pendulum's clock, and one bar empties while the
other fills, with the total level (or falling, with damping). Gallery scene: `Pendulum`.

```rust
use std::sync::OnceLock;

use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTION: OnceLock<PendulumMotion> = OnceLock::new();
    let motion = MOTION.get_or_init(|| PendulumMotion::new(2.5, 0.9, 6.0));
    let delay = 0.5;
    let t = f64::from((ui.seconds() - delay).max(0.0));
    let (kinetic, potential) = motion.energy(t, 1.0);
    let pendulum = ui.show(Pendulum {
        motion: Some(motion),
        pivot: (-2.5, 2.6),
        delay,
        ..ui.defaults()
    });
    let bars = ui.show(EnergyBars {
        kinetic,
        potential,
        max: motion.energy(0.0, 1.0).1,
        ..ui.defaults()
    });
    svgr!(<g>{pendulum}{bars}</g>)
}
```

## SpringMass

`SpringMotion::new(stiffness, mass, x0, seconds)` lets a block go from rest `x0` meters from
its rest position, and `SpringMotion::damped(stiffness, mass, damping, x0, seconds)` adds
friction. `SpringMass` draws the block on a spring from a wall. With `plot` set to the map of
axes you show (time along x, displacement up y), the displacement so far draws on them with
a dot at the current value, so the block and its graph move together. `trace(until,
per_second)` gives the same points for a plot of your own. Gallery scene: `SpringMass`.

```rust
use std::sync::OnceLock;

use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MOTION: OnceLock<SpringMotion> = OnceLock::new();
    let motion = MOTION.get_or_init(|| SpringMotion::damped(20.0, 1.0, 0.6, 1.0, 6.0));
    let axes = Axes {
        x_range: Range::new(0.0, 6.0, 1.0),
        y_range: Range::new(-1.0, 1.0, 0.5),
        x_length: 9.0,
        y_length: 2.6,
        center: (0.0, -2.0),
        tips: false,
        ..ui.defaults()
    };
    let spring = ui.show(SpringMass {
        motion: Some(motion),
        wall: (-5.5, 1.6),
        rest_length: 3.5,
        plot: Some(axes.map(ui)),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    svgr!(<g>{axes}{spring}</g>)
}
```
