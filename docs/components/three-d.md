# fframes-kit: 3D

The `3d` feature of `fframes-kit` (`fframes-kit = { .., features = ["3d"] }`) adds the
`three` module and its names to the prelude. Gallery scenes: `ThreeDAxes`, `Surface`,
`Solids`, `Polyhedra`.

## ThreeDView

`three::ThreeDView` is Manim's `ThreeDScene`. Its `objects` are 3D mobjects, rendered by
`fframes::three_d` into one image over the canvas, and `fixed_in_frame` draws 2D on top.
The camera is Manim's: `phi` from the `z` axis, `theta` around it, `focal_distance` 20 and
`zoom` 1, so with the default `phi` 0 and `theta` -90° the `xy` plane lines up with
`ui.space`. `ambient_rotation` turns `theta` by that many radians per second, Manim's
`begin_ambient_camera_rotation`. `ThreeDView::projector(ui)` maps Manim's points to pixels
for labels on 3D points; `ThreeDAxes` draws its `x`, `y` and `z` that way.

The mobjects keep Manim's names, sizes and defaults: `Surface` (a checkerboard of
`func(u, v)`), `ThreeDAxes` (with `c2p`), `Sphere`, `Cube`, `Prism`, `Cylinder`, `Cone`,
`Torus`, `Dot3D`, `Line3D`, `Arrow3D`, `Polyhedron`, the four Platonic solids and
`ConvexHull3D`. Positions are Manim's `(x, y, z)` in units, `z` out of the screen. Every
object takes `enter: Some(Create { .. }.into())` to grow from its center or `FadeIn` to
fade in.

The scene is rebuilt for every frame. One checkerboard sphere at 1080p costs about 60 ms on
32 CPU threads; with `fframes/wgpu` the renderer uses the GPU.

```rust
use fframes_kit::prelude::*;

fn wave(u: f32, v: f32) -> (f32, f32, f32) {
    (u, v, 0.5 * u.sin() * v.cos())
}

fn surface<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let objects: Vec<Box<dyn Object3D + 'a>> = vec![
        Box::new(ThreeDAxes { ..ui.defaults() }),
        Box::new(Surface { func: wave, u_range: (-3.0, 3.0), v_range: (-3.0, 3.0), ..ui.defaults() }),
        Box::new(Sphere { center: (0.0, 0.0, 2.0), radius: 0.5, enter: Some(Create { ..ui.defaults() }.into()), ..ui.defaults() }),
    ];
    ui.show(ThreeDView {
        phi: 75f32.to_radians(),
        theta: 30f32.to_radians(),
        ambient_rotation: 0.2,
        objects,
        ..ui.defaults()
    })
}
```
