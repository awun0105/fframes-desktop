//! The physics kit's second set: fields of charges and currents, lenses, rigid bodies,
//! waves and circuit symbols.

use fframes_kit::prelude::*;

use super::core::title;
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

pub fn scenes() -> Vec<GalleryScene> {
    #[allow(unused_mut)]
    let mut scenes = vec![
        GalleryScene::new("ElectricField", 4.0, electric_field),
        GalleryScene::new("MagneticField", 2.0, magnetic_field),
        GalleryScene::new("Lens", 3.5, lens),
        GalleryScene::new("RadialWave", 4.0, radial_wave),
    ];
    #[cfg(feature = "three-d")]
    scenes.push(GalleryScene::new("RadialWave3D", 2.5, radial_wave_3d));
    scenes
}
