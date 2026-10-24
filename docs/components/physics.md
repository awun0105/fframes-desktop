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
- **Simulations.** A motion with a closed form (`Projectile`, `Wave`, `Orbit`, `Collision1D`) is
  computed per frame. One without (`Pendulum`, `SpringMass`) is stepped once, at 240 steps per second with RK4, into
  a motion value built in the video's constructor (a `OnceLock` field, or a field set in
  `new`). The figure borrows it through `motion: Some(&..)` and draws nothing without it.
  Field lines are traced once the same way, into a `Field`.
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

## RigidWorld

Manim-physics' `Space` and `SpaceScene`. `RigidMotion::new(bodies, gravity, seconds)` runs
[rapier](https://rapier.rs) once at 60 steps a second, in the video's constructor. The same
bodies give the same motion every time (`enhanced-determinism`). A `Body` is
`Body::circle`, `Body::rect`, `Body::polygon` (the convex hull of its points) or
`Body::wall(a, b, thickness)`, which is fixed. It is set up with `velocity`, `spin`,
`angle`, `fixed`, `restitution`, `friction`, `density` and `color`. `poses(t)` gives every
body's `Pose` (`x`, `y`, and an `angle` that counts whole turns). `RigidWorld` draws the
bodies, filled and outlined, with a radius on circles that shows them turning. Its `draw`
closure gets a `BodyView` (index, center in pixels, rotation in degrees for SVG `rotate()`,
pixels per unit) and returns its own drawing, or `None` to keep the default. Rapier adds
about 2 MB to an editor bridge built with `physics`. Gallery scene: `RigidWorld`.

```rust
use fframes_kit::prelude::*;

struct Video {
    world: RigidMotion,
}

impl Video {
    fn new() -> Self {
        let bodies = vec![
            Body::wall((-6.0, -3.0), (6.0, -3.0), 0.3),
            Body::circle(-2.0, 2.0, 0.4).velocity(1.5, 0.0).restitution(0.8),
            Body::rect(1.0, 1.0, 1.0, 0.6).angle(0.4),
        ];
        Self {
            world: RigidMotion::new(bodies, (0.0, -9.81), 5.0),
        }
    }

    fn figure<'a>(&'a self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        ui.show(RigidWorld {
            motion: Some(&self.world),
            ..ui.defaults()
        })
    }
}
```

## MultiPendulum

`MultiPendulumMotion::new(lengths, masses, angles, seconds)` solves a chain of point masses
on massless rods released at rest, a double pendulum with two links, from the Lagrangian with
Runge-Kutta at 240 steps a second. Keep it in the video, built once. Over 10 s a chaotic
double pendulum keeps its energy within 1e-6 on the steps. Sampled between two steps, the
blended state is off by about 1e-3, which does not show. `state(t)`, `positions(t)` (meters
from the pivot, `y` up) and `energy(t)` give the numbers. `MultiPendulum` draws it from
`pivot`, `scale` units per meter. With `trail` the last bob leaves a trail, sampled like
`TracedPath`, over the last `trail_time` seconds. Gallery scene: `MultiPendulum`.

```rust
use fframes_kit::prelude::*;

struct Video {
    chain: MultiPendulumMotion,
}

impl Video {
    fn new() -> Self {
        Self {
            chain: MultiPendulumMotion::new(&[1.4, 1.1], &[1.0, 1.0], &[2.2, 2.6], 10.0),
        }
    }

    fn figure<'a>(&'a self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        ui.show(MultiPendulum {
            motion: Some(&self.chain),
            pivot: (0.0, 1.0),
            trail_time: Some(2.0),
            ..ui.defaults()
        })
    }
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

## Wave

One figure for four kinds of wave, chosen by `kind`:

- `WaveKind::Transverse`: the medium moves up and down;
- `WaveKind::Longitudinal`: a row of particles that bunch and spread;
- `WaveKind::Standing`: `2A sin(kx) cos(ωt)` with its nodes and dashed envelope;
- `WaveKind::Superposition`: the wave and a second one (`second_wavelength`,
  `second_amplitude`) drawn thin, with their sum drawn thick.

With `front` (the default) the medium starts at rest and the wave leaves the left end.
`marker` picks out one point of the medium, which only moves in place. `displacement(x, t)`,
`envelope(x)` and `nodes()` give the numbers. Gallery scene: `Wave` (all four).

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Wave {
        kind: WaveKind::Superposition,
        origin: (-6.0, 0.0),
        length: 12.0,
        amplitude: 0.5,
        wavelength: 3.0,
        second_wavelength: 2.2,
        speed: 3.0,
        marker: Some(6.0),
        ..ui.defaults()
    })
}
```

`harmonic(n)` turns a wave into the standing wave of a string's `n`th harmonic, fixed at
both ends (Manim-physics' `StandingWave`): `Wave { length: 9.0, ..ui.defaults() }.harmonic(3)`.
Manim-physics' `LinearWave` is `Wave { front: false, .. }` with `speed = wavelength / period`.

## RadialWave

Circular waves from every point of `sources`, `amplitude sin(k r - ω t)` each, summed where
they meet. In 2D the crests are rings and the troughs dashed rings, clipped to `x_range` by
`y_range`; where the rings of two sources cross, the waves add up. With `front` (the
default) the rings leave the sources on the figure's clock. `displacement(x, y, t)` gives the
height. With the `3d` feature the same wave goes into a `ThreeDView` as a surface
(`resolution` pieces a side, `amplitude` high); only `enter` applies there. Gallery scenes:
`RadialWave`, and `RadialWave3D` with the gallery's `three-d` feature (on by default).

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(RadialWave {
        sources: vec![(-1.6, 0.0), (1.6, 0.0)],
        wavelength: 1.1,
        speed: 1.6,
        x_range: (-7.0, 7.0),
        y_range: (-4.0, 4.0),
        ..ui.defaults()
    })
}
```

## FreeBody

A block with `forces` (`Force::new(name, x, y)`, in newtons, `y` up) drawn from its middle,
`scale` units per newton. The arrows grow one after another, each labelled at its tip with
its name and magnitude. Then the other forces dim and their sum grows as a dashed arrow
labelled `net = … N`, or `net = 0` under the block when they balance. `net_force()` gives
the sum. Gallery scene: `FreeBody`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(FreeBody {
        forces: vec![
            Force::new("W", 0.0, -5.0),
            Force::new("N", 0.0, 5.0),
            Force::new("F", 6.0, 0.0),
            Force::new("f", -2.0, 0.0),
        ],
        center: (0.0, -0.4),
        scale: 0.45,
        delay: 0.5,
        ..ui.defaults()
    })
}
```

## Orbit

A planet on a Kepler orbit around a star at `focus`: `semi_major` units, `eccentricity`, the
perihelion to the right of the star, once around every `period` seconds (Kepler's equation
is solved for each frame). With `sweeps`, the first orbit shades that many sectors from the
star, each swept in the same `sweep` fraction of the period. The sector near the star is
short and wide, the far one long and thin, and their areas are equal. `position(t)` and
`eccentric_anomaly(m, e)` give the numbers. Gallery scene: `Orbit`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Orbit {
        focus: (2.0, -0.5),
        semi_major: 4.0,
        eccentricity: 0.6,
        period: 5.0,
        sweeps: 3,
        sweep: 0.12,
        ..ui.defaults()
    })
}
```

## FieldLines

`Field::new(charges, lines_per_charge, bounds)` traces the field lines of point charges once
(`Charge::new(x, y, q)`). The lines leave the positive charges, evenly spaced, and end on a
negative charge or at `bounds`. `FieldLines` draws them on together over `draw` seconds, each
with an arrowhead at its middle. Then, with `test_charge: Some(i)`, a small charge follows
line `i`. `field.at(x, y)` gives the field and `field.lines()` the points. Gallery scene:
`FieldLines`.

```rust
use std::sync::OnceLock;

use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static FIELD: OnceLock<Field> = OnceLock::new();
    let field = FIELD.get_or_init(|| {
        Field::new(
            vec![Charge::new(-2.5, 0.0, 1.0), Charge::new(2.5, 0.0, -1.0)],
            16,
            (-7.3, -4.1, 7.3, 4.1),
        )
    });
    ui.show(FieldLines {
        field: Some(field),
        delay: 0.5,
        test_charge: Some(2),
        ..ui.defaults()
    })
}
```
## ElectricField and MagneticField

`ElectricField` is manim-physics' `ElectricField`: an arrow on every grid point of
`x_range` by `y_range`, pointing along the field of `charges` and coloured by its strength
(`colors` from `min_magnitude` to `max_magnitude`), with each charge as a glowing
`ChargeGlow`. `lines_per_charge` adds the field lines of `Field`. `charges` is a plain prop:
compute it from `ui.seconds()` and the arrows and lines follow, the lines traced again each
frame. `ChargeGlow { charge, .. }` draws one charge on its own. Gallery scene:
`ElectricField`.

`MagneticField` is the field of `wires` in the page, by the Biot–Savart law (`μ₀` left out).
A `Wire::Through { x, y, current }` crosses the page (current out of it when positive) and
circles its field around it; a `Wire::Segment { from, to, current }` lies in the page, and
its field there points out of or into the page. Where the field lies mostly in the page the
grid shows an arrow, otherwise a ⊙ (out) or ⊗ (in) sized by its strength.
`magnetic_field(&wires, x, y)` returns `(B_x, B_y, B_z)`. Gallery scene: `MagneticField`.

```rust
use fframes_kit::prelude::*;

fn fields<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    // a negative charge circling a positive one
    let angle = 0.6 * f64::from(ui.seconds());
    let electric = ui.show(ElectricField {
        charges: vec![
            Charge::new(-1.0, 0.0, 2.0),
            Charge::new(-1.0 + 3.0 * angle.cos(), 2.0 * angle.sin(), -1.0),
        ],
        lines_per_charge: 8,
        ..ui.defaults()
    });
    let magnetic = ui.show(MagneticField {
        wires: vec![
            Wire::Through { x: -2.0, y: 0.0, current: 1.0 },
            Wire::Segment { from: (1.0, -1.0), to: (4.0, -1.0), current: 2.0 },
        ],
        ..ui.defaults()
    });
    svgr!(<g>{electric}{magnetic}</g>)
}
```

## Circuit

A series circuit on a rectangular loop: a battery on the left side, a switch on the top, a
bulb on the right and a resistor on the bottom. Turn a part off (`switch: false`, ..) and
the wire runs on. The switch closes at `on_at` seconds. Then the current flows as dots out
of the battery's long plate, clockwise, and the bulb glows. A label left empty is not drawn.
Gallery scene: `Circuit`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Circuit {
        center: (0.0, -0.4),
        width: 8.0,
        height: 4.5,
        on_at: 1.4,
        battery_label: "9 V",
        enter: Some(Create::default().into()),
        ..ui.defaults()
    })
}
```

## Symbol and Schematic

`Symbol` draws one circuit part: `SymbolKind::Resistor`, `Capacitor`, `Inductor`,
`VoltageSource`, `CurrentSource`, `Ground`, `Opamp` (Manim-circuit's set), and `Battery`,
`Switch { closes_at }`, `Bulb` and `Diode`. It sits at `at`, turned `rotation` degrees
counterclockwise, `size` units from its middle to a terminal, with an optional `label`.
`terminal(name)` gives where a wire attaches: `a` and `b` for every two-terminal part (also
`-` and `+` on the sources and the battery, `anode` and `cathode` on the diode), `a` on top of
`Ground`, and `-`, `+`, `out` on the op-amp.

`Schematic` wires symbols together. A `Connection` runs from a pin to a pin, a terminal
`(symbol index, name)` or a point `(x, y)` to end on another wire, through the corners given
with `via`. Without corners a wire whose ends are not in line goes horizontally, then
vertically. There is no automatic routing. Junction dots mark where three or more conductors
meet. From `current_at` the current flows as dots along every wire (reversed with a negative
`current(..)`, none with 0) and bulbs light up. Each wire moves its own dots, so they do not
line up across a junction. A wire with a missing terminal is skipped, and `unresolved()` lists
it. Gallery scene: `Schematic`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let theme = ui.theme.clone();
    let part = |kind, at, rotation, label| Symbol {
        kind,
        at,
        rotation,
        label,
        ..Symbol::themed(&theme)
    };
    ui.show(Schematic {
        symbols: vec![
            part(SymbolKind::Battery, (-3.0, 0.0), 90.0, "9 V"),
            part(SymbolKind::Resistor, (0.0, 1.5), 0.0, "220R"),
            part(SymbolKind::Bulb, (3.0, 0.0), 90.0, ""),
        ],
        wires: vec![
            Connection::new((0, "+"), (1, "a")).via([(-3.0, 1.5)]),
            Connection::new((1, "b"), (2, "b")).via([(3.0, 1.5)]),
            Connection::new((2, "a"), (0, "-")).via([(3.0, -1.5), (-3.0, -1.5)]),
        ],
        current_at: Some(1.0),
        ..ui.defaults()
    })
}
```

## RayDiagram and Refraction

`RayDiagram` draws the three principal rays of a thin lens (`Optic::Lens`) or a mirror
(`Optic::Mirror`). A positive `focal` is a converging lens or a concave mirror, a negative
one a diverging lens or a convex mirror. The rays leave the tip of an object
`object_distance` units in front, one after another. Then the image forms where they meet,
dashed with the rays' backward extensions when it is virtual. `image_distance`,
`magnification` and `image` give the numbers from `1/d_o + 1/d_i = 1/f`. Gallery scenes:
`RayDiagram` (a lens) and `Mirror`.

`Refraction` bends a ray at the boundary between two media by Snell's law, with the weak
reflection and the angles. Past the critical angle (`critical_angle()`) the ray is
reflected whole. Gallery scene: `Refraction`.

```rust
use fframes_kit::prelude::*;

fn figures<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let lens = ui.show(RayDiagram {
        center: (0.5, 1.5),
        focal: 1.8,
        object_distance: 4.0,
        object_height: 1.0,
        optic_height: 2.5,
        delay: 0.5,
        ..ui.defaults()
    });
    let glass = ui.show(Refraction {
        point: (0.0, -2.0),
        n1: 1.0,
        n2: 1.5,
        angle: 0.75,
        length: 1.8,
        ..ui.defaults()
    });
    svgr!(<g>{lens}{glass}</g>)
}
```

## OpticalBench, Lens and Ray

`OpticalBench` is manim-physics' `Lens` and `Ray`: `lenses` on one horizontal axis, and
`rays` traced through them from left to right. `Lens::thin(x, focal)` is a thin lens,
`Lens::thick(x, r1, r2, index, thickness)` a real one from its radii, refractive index and
thickness (`focal_length()` is the lensmaker's equation). Radii are positive when the centre
of curvature is to the right, so a biconvex lens has `r1 > 0` and `r2 < 0`; a flat side is
`f64::INFINITY`. The rays are traced with paraxial optics, surface by surface, and a ray that
meets a lens outside its `height` stops there. Each lens shows its front and back focal
points, `F` and `F'`, found by tracing a parallel ray, so a thick lens's sit at its focal
distances from its surfaces. Gallery scene: `Lens`.

```rust
use fframes_kit::prelude::*;

fn bench<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let rays = (0..7)
        .map(|i| Ray::new((-6.8, -0.9 + 0.3 * f64::from(i)), (1.0, 0.0)))
        .collect();
    ui.show(OpticalBench {
        lenses: vec![Lens::thick(-3.0, 2.4, -2.4, 1.5, 0.5), Lens::thin(2.6, -2.0)],
        rays,
        delay: 0.5,
        ..ui.defaults()
    })
}
```

## Collision1D

Carts A and B on a track that collide head on: masses `m1`, `m2`, velocities `u1`, `u2`,
centers `x1`, `x2`, and a `restitution` from 1 (elastic) to 0 (they stick together). The
motion is a closed form, and heavier carts are drawn longer. Each cart carries its velocity.
Momentum bars under the track show `p A`, `p B` and their sum, which stays the same through
the collision. A cart stops at the end of the track, and the end's push changes the sum
then. `collision_time`, `final_velocities`, `state` and `momenta` give the numbers. Gallery
scene: `Collision1D`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Collision1D {
        m1: 2.0,
        m2: 1.0,
        u1: 2.0,
        u2: -0.5,
        x1: 1.5,
        x2: 5.5,
        restitution: 1.0,
        track: (-6.0, 0.2),
        ..ui.defaults()
    })
}
```
## Doppler

A source moving right from `start` at `source_speed` m/s sends out a front every `period`
seconds, and the fronts spread at `wave_speed` m/s. They bunch up ahead of the source and
spread out behind it. At or above the wave speed they pile into a cone. A front fades as it
grows and is gone at `max_radius` meters. Listeners `ahead` and `behind` (meters from
`start`) show the frequency they hear, `f c / (c ∓ v)`, once the first front has reached
them. The source's own frequency sits above it, on a chip of the background. `heard` and
`fronts` give the numbers. Gallery scene: `Doppler`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Doppler {
        start: (-3.5, 0.0),
        scale: 0.7,
        source_speed: 1.0,
        wave_speed: 3.0,
        period: 0.4,
        ahead: Some(8.0),
        behind: Some(-4.0),
        ..ui.defaults()
    })
}
```

## GasBox

`GasMotion::new(GasSetup { .. })` steps `count` hard balls in a box once, at 240 steps a
second: elastic collisions between equal masses and with the walls, with an optional
`Heat { at, to, over }` that brings the temperature to `to` times its start. The start
positions (a grid with jitter) and the directions come from `seed`. The steps use only
`+ - × ÷` and `√`, so the same setup gives the same frames on every run and machine.
`GasBox` draws the particles, slow ones in `slow_color` and those faster than the start's
root mean square speed in `fast_color`, and a thermometer with the factor `T × ..`. Build
the motion in the video's constructor. `particles`, `temperature` and `rms_speed` give the
numbers. Gallery scene: `GasBox`.

```rust
use fframes_kit::prelude::*;

struct Lesson {
    gas: GasMotion,
}

impl Lesson {
    fn new() -> Self {
        let setup = GasSetup {
            seed: 7,
            count: 60,
            seconds: 7.0,
            heat: Some(Heat { at: 2.5, to: 3.0, over: 1.5 }),
            ..GasSetup::default()
        };
        Self { gas: GasMotion::new(setup) }
    }

    fn figure<'a>(&'a self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        ui.show(GasBox {
            motion: Some(&self.gas),
            scale: 1.4,
            ..ui.defaults()
        })
    }
}
```