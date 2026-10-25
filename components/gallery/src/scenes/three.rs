//! Manim's cameras and 3D mobjects.

use fframes_kit::explain::auto_zoom;
use fframes_kit::kurbo;
use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

/// The scene with `title`.
macro_rules! scenes {
    ($($name:literal, $seconds:expr => |$ui:ident| $body:expr,)*) => {
        vec![$(GalleryScene::new($name, $seconds, |$ui| {
            let head = title($ui, $name);
            let body: Svgr<'_> = $body;
            svgr!(<g>{head}{body}</g>)
        }),)*]
    };
}

/// A square, a circle and a small labelled dot to look at through the cameras.
fn still_life<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let square = ui.show(Square {
        cx: 560.0,
        cy: 580.0,
        side_length: 300.0,
        fill: Some(c.accent),
        ..ui.defaults()
    });
    let circle = ui.show(Circle {
        cx: 1340.0,
        cy: 580.0,
        radius: 160.0,
        stroke: Some(c.highlight),
        stroke_width: 6.0,
        ..ui.defaults()
    });
    let dot = ui.show(Dot {
        cx: 1340.0,
        cy: 580.0,
        radius: 6.0,
        fill: Some(c.title),
        ..ui.defaults()
    });
    let ring = ui.show(Circle {
        cx: 1340.0,
        cy: 580.0,
        radius: 24.0,
        stroke: Some(c.accent),
        stroke_width: 2.0,
        ..ui.defaults()
    });
    svgr!(<g>{square}{circle}{dot}{ring}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    #[allow(unused_mut)]
    let mut scenes = scenes![
        "MovingCamera", 2.0 => |ui| {
            // from the whole canvas onto the circle and back, like `self.camera.frame.animate`
            let space = ui.space;
            let full = fframes_kit::explain::full_frame_width(space);
            let circle = kurbo::Rect::new(1180.0, 420.0, 1500.0, 740.0);
            let (target, width) = auto_zoom(space, &[circle], 0.5).unwrap_or(((0.0, 0.0), full));
            let p = RateFunc::ThereAndBack.apply(motion::progress(ui.seconds(), 0.25, 1.75), 1.5);
            let camera = MovingCamera {
                center: (motion::lerp(0.0, target.0, p), motion::lerp(0.0, target.1, p)).into(),
                width: motion::lerp(full, width, p).into(),
                children: children(still_life),
            };
            ui.show(camera)
        },
        "ZoomedInset", 2.0 => |ui| {
            let inset = ZoomedInset {
                frame: kurbo::Rect::new(1290.0, 540.0, 1390.0, 600.0),
                display: kurbo::Rect::new(1400.0, 120.0, 1840.0, 384.0),
                delay: 0.25,
                run_time: 1.0,
                children: children(still_life),
                ..ui.defaults()
            };
            ui.show(inset)
        },
    ];
    #[cfg(feature = "three-d")]
    scenes.extend(three_d::scenes());
    scenes
}

#[cfg(feature = "three-d")]
mod three_d {
    use std::f32::consts::{PI, TAU};

    use fframes_kit::prelude::*;

    use super::title;
    use crate::GalleryScene;

    const DEG: f32 = PI / 180.0;

    /// Manim's favourite camera: `phi` 75°, `theta` 30°, turning slowly.
    fn view<'a>(objects: Vec<Box<dyn Object3D + 'a>>, rotation: f32) -> ThreeDView<'a> {
        ThreeDView {
            phi: 75.0 * DEG,
            theta: 30.0 * DEG,
            ambient_rotation: rotation,
            objects,
            ..ThreeDView::themed(&Theme::default())
        }
    }

    fn ripple(u: f32, v: f32) -> (f32, f32, f32) {
        (u, v, 0.6 * (u * 1.4).sin() * (v * 1.4).cos())
    }

    /// Axes drawn in, a dot, a line and an arrow on them, the camera turning.
    fn axes<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let head = title(ui, "ThreeDAxes");
        let c = ui.theme.colors;
        let axes = ThreeDAxes {
            enter: Some(
                Create {
                    run_time: 1.0,
                    ..ui.defaults()
                }
                .into(),
            ),
            ..ui.defaults()
        };
        let p = axes.c2p(3.0, 2.0, 2.0);
        let later = |delay: f32| -> Option<Enter3D> {
            Some(
                FadeIn {
                    delay,
                    run_time: 0.5,
                    ..FadeIn::themed(&Theme::default())
                }
                .into(),
            )
        };
        let objects: Vec<Box<dyn Object3D + 'a>> = vec![
            Box::new(axes),
            Box::new(Line3D {
                start: axes.c2p(0.0, 0.0, 0.0),
                end: p,
                color: c.highlight,
                enter: later(1.0),
                ..Line3D::default()
            }),
            Box::new(Arrow3D {
                start: axes.c2p(0.0, 0.0, 0.0),
                end: axes.c2p(-3.0, 2.0, 0.0),
                color: c.accent,
                thickness: 0.03,
                enter: later(1.3),
                ..Arrow3D::default()
            }),
            Box::new(Dot3D {
                point: p,
                radius: 0.12,
                color: c.highlight,
                enter: later(1.0),
                ..Dot3D::default()
            }),
        ];
        let figure = ui.show(view(objects, 0.15));
        svgr!(<g>{figure}{head}</g>)
    }

    /// A rippled checkerboard surface over axes, fading in while the camera turns.
    fn surface<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let head = title(ui, "Surface");
        let objects: Vec<Box<dyn Object3D + 'a>> = vec![
            Box::new(ThreeDAxes {
                show_labels: false,
                ..ui.defaults()
            }),
            Box::new(Surface {
                func: ripple,
                u_range: (-3.0, 3.0),
                v_range: (-3.0, 3.0),
                resolution: (24, 24),
                enter: Some(
                    FadeIn {
                        run_time: 1.0,
                        ..ui.defaults()
                    }
                    .into(),
                ),
                ..ui.defaults()
            }),
        ];
        let figure = ui.show(view(objects, 0.2));
        svgr!(<g>{figure}{head}</g>)
    }

    /// The six solids, each growing from its center in turn.
    fn solids<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let head = title(ui, "Solids");
        let grow = |i: usize| -> Option<Enter3D> {
            Some(
                Create {
                    delay: 0.2 * i as f32,
                    run_time: 0.8,
                    ..Create::themed(&Theme::default())
                }
                .into(),
            )
        };
        let x = |i: usize| -5.0 + 2.0 * i as f32;
        let objects: Vec<Box<dyn Object3D + 'a>> = vec![
            Box::new(Sphere {
                center: (x(0), 0.0, 0.0),
                radius: 0.8,
                enter: grow(0),
                ..ui.defaults()
            }),
            Box::new(Cube {
                center: (x(1), 0.0, 0.0),
                side_length: 1.3,
                enter: grow(1),
                ..ui.defaults()
            }),
            Box::new(Cylinder {
                center: (x(2), 0.0, 0.0),
                radius: 0.6,
                height: 1.6,
                enter: grow(2),
                ..ui.defaults()
            }),
            Box::new(Cone {
                apex: (x(3), 0.0, 0.8),
                base_radius: 0.7,
                height: 1.6,
                show_base: true,
                enter: grow(3),
                ..ui.defaults()
            }),
            Box::new(Torus {
                center: (x(4), 0.0, 0.0),
                major_radius: 0.6,
                minor_radius: 0.25,
                enter: grow(4),
                ..ui.defaults()
            }),
            Box::new(Prism {
                center: (x(5), 0.0, 0.0),
                dimensions: (1.0, 0.7, 1.5),
                enter: grow(5),
                ..ui.defaults()
            }),
        ];
        let figure = ui.show(ThreeDView {
            phi: 65.0 * DEG,
            theta: -80.0 * DEG,
            objects,
            ..ui.defaults()
        });
        svgr!(<g>{figure}{head}</g>)
    }

    /// The four Platonic solids and the hull of points on a twisted ring.
    fn polyhedra<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let head = title(ui, "Polyhedra");
        let fade = |i: usize| -> Option<Enter3D> {
            Some(
                FadeIn {
                    delay: 0.2 * i as f32,
                    run_time: 0.6,
                    ..FadeIn::themed(&Theme::default())
                }
                .into(),
            )
        };
        let ring: Vec<(f32, f32, f32)> = (0..10)
            .map(|i| {
                let a = TAU * i as f32 / 10.0;
                (
                    0.9 * a.cos(),
                    0.9 * a.sin(),
                    if i % 2 == 0 { 0.5 } else { -0.5 },
                )
            })
            .collect();
        let x = |i: usize| -4.8 + 2.4 * i as f32;
        let objects: Vec<Box<dyn Object3D + 'a>> = vec![
            Box::new(Tetrahedron {
                edge_length: 1.6,
                center: (x(0), 0.0, 0.0),
                enter: fade(0),
                ..ui.defaults()
            }),
            Box::new(Octahedron {
                edge_length: 1.3,
                center: (x(1), 0.0, 0.0),
                enter: fade(1),
                ..ui.defaults()
            }),
            Box::new(Icosahedron {
                edge_length: 1.0,
                center: (x(2), 0.0, 0.0),
                enter: fade(2),
                ..ui.defaults()
            }),
            Box::new(Dodecahedron {
                edge_length: 0.65,
                center: (x(3), 0.0, 0.0),
                enter: fade(3),
                ..ui.defaults()
            }),
            Box::new(ConvexHull3D {
                points: ring,
                center: (x(4), 0.0, 0.0),
                enter: fade(4),
                ..ui.defaults()
            }),
        ];
        let figure = ui.show(ThreeDView {
            phi: 70.0 * DEG,
            theta: -70.0 * DEG,
            ambient_rotation: 0.1,
            objects,
            ..ui.defaults()
        });
        svgr!(<g>{figure}{head}</g>)
    }

    /// The Lorenz attractor (sigma 10, rho 28, beta 8/3) once it has settled, in Manim units
    /// around the origin: the points of [`lorenz`]'s trace.
    fn lorenz_points() -> &'static [(f32, f32, f32)] {
        static POINTS: std::sync::OnceLock<Vec<(f32, f32, f32)>> = std::sync::OnceLock::new();
        POINTS.get_or_init(|| {
            let f = |(x, y, z): (f64, f64, f64)| {
                (10.0 * (y - x), x * (28.0 - z) - y, x * y - 8.0 / 3.0 * z)
            };
            let add = |a: (f64, f64, f64), b: (f64, f64, f64), k: f64| {
                (a.0 + b.0 * k, a.1 + b.1 * k, a.2 + b.2 * k)
            };
            let dt = 0.005;
            let mut p = (1.0, 1.0, 1.0);
            let mut points = Vec::new();
            for i in 0..7400 {
                // fourth order Runge-Kutta
                let k1 = f(p);
                let k2 = f(add(p, k1, dt / 2.0));
                let k3 = f(add(p, k2, dt / 2.0));
                let k4 = f(add(p, k3, dt));
                p = add(p, add(add(k1, k4, 1.0), add(k2, k3, 1.0), 2.0), dt / 6.0);
                // the first steps fall onto the attractor from (1, 1, 1)
                if i >= 1500 && i % 9 == 0 {
                    let s = 0.115;
                    points.push((
                        (p.0 * s) as f32,
                        (p.1 * s) as f32,
                        ((p.2 - 25.0) * s) as f32,
                    ));
                }
            }
            points
        })
    }

    /// The Lorenz attractor drawn out in 3D, its colour running round the wheel as it goes,
    /// a bright dot at its head, the camera turning round it.
    fn lorenz<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let head = title(ui, "LorenzAttractor");
        let points = lorenz_points();
        let shown = (motion::progress(ui.seconds(), 0.3, 5.2) * points.len() as f32) as usize;
        let n = points.len() as f32;
        let mut objects: Vec<Box<dyn Object3D + 'a>> = points
            .windows(2)
            .take(shown)
            .enumerate()
            .map(|(i, w)| -> Box<dyn Object3D + 'a> {
                Box::new(Line3D {
                    start: w[0],
                    end: w[1],
                    thickness: 0.025,
                    color: crate::scenes::core::hue(i as f32 / n),
                    ..ui.defaults()
                })
            })
            .collect();
        if let Some(&point) = points.get(shown.max(1) - 1) {
            objects.push(Box::new(Dot3D {
                point,
                radius: 0.09,
                ..ui.defaults()
            }));
        }
        let figure = ui.show(ThreeDView {
            phi: 78.0 * DEG,
            theta: -125.0 * DEG,
            ambient_rotation: 0.2,
            objects,
            ..ui.defaults()
        });
        let glow = crate::scenes::core::neon("gallery-lorenz-glow", 6.0);
        svgr!(
            <g>
                <defs>{glow}</defs>
                <g filter="url(#gallery-lorenz-glow)">{figure}</g>
                {head}
            </g>
        )
    }

    pub fn scenes() -> Vec<GalleryScene> {
        vec![
            GalleryScene::new("ThreeDAxes", 3.0, axes),
            GalleryScene::new("Surface", 3.0, surface),
            GalleryScene::new("Solids", 2.5, solids),
            GalleryScene::new("Polyhedra", 2.5, polyhedra),
            GalleryScene::new("LorenzAttractor", 6.0, lorenz),
        ]
    }
}
