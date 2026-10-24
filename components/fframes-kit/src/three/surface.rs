use fframes::{
    Color,
    three_d::{Geometry, Material, Rgb, Scene, Vec3},
};

use crate::{
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Enter3D, Object3D, Point3, darker, place, rgb, world};

/// The most pieces a [`Surface`] is cut into along `u` or `v`.
pub const MAX_SURFACE_RESOLUTION: usize = 256;

/// The flat-shaded mesh of `f(u, v)` over the ranges, `res.0` by `res.1` pieces, in
/// `colors.0`, or a checkerboard of both colours. Both sides are drawn.
pub(crate) fn parametric(
    f: impl Fn(f32, f32) -> Point3,
    u: (f32, f32),
    v: (f32, f32),
    res: (usize, usize),
    colors: (Color, Option<Color>),
) -> (Geometry, Material) {
    let (nu, nv) = (
        res.0.clamp(1, MAX_SURFACE_RESOLUTION),
        res.1.clamp(1, MAX_SURFACE_RESOLUTION),
    );
    let at = |i: usize, j: usize| -> Vec3 {
        let a = u.0 + (u.1 - u.0) * i as f32 / nu as f32;
        let b = v.0 + (v.1 - v.0) * j as f32 / nv as f32;
        world(f(a, b))
    };
    let (mut even, mut odd) = (Vec::new(), Vec::new());
    for i in 0..nu {
        for j in 0..nv {
            let (p00, p10, p11, p01) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            let quad = if (i + j) % 2 == 0 || colors.1.is_none() {
                &mut even
            } else {
                &mut odd
            };
            quad.extend([p00, p10, p11, p00, p11, p01]);
        }
    }
    let first = Geometry::from_triangles(&even).with_color(rgb(colors.0));
    let geometry = match colors.1 {
        Some(second) if !odd.is_empty() => Geometry::merge([
            first,
            Geometry::from_triangles(&odd).with_color(rgb(second)),
        ]),
        _ => first,
    };
    let material = Material::standard(Rgb::WHITE).double_sided().flat();
    (geometry, material)
}

/// Manim's `Surface`: the points `func(u, v)` over `u_range` and `v_range`, cut into
/// `resolution` pieces (32 by 32) and coloured as a checkerboard of `checkerboard_colors`,
/// or all `fill_color` without them. `center` moves it, and `enter` animates it about
/// `center`: give ranges around 0 to grow it from its middle.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    pub func: fn(f32, f32) -> Point3,
    pub u_range: (f32, f32),
    pub v_range: (f32, f32),
    pub resolution: (usize, usize),
    pub fill_color: Color,
    pub checkerboard_colors: Option<(Color, Color)>,
    pub opacity: f32,
    pub center: Point3,
    pub enter: Option<Enter3D>,
}

fn flat(u: f32, v: f32) -> Point3 {
    (u, v, 0.0)
}

impl Themed for Surface {
    /// The unit square in the `xy` plane until `func` is set.
    fn themed(t: &Theme) -> Self {
        Self {
            func: flat,
            u_range: (0.0, 1.0),
            v_range: (0.0, 1.0),
            resolution: (32, 32),
            fill_color: t.colors.accent,
            checkerboard_colors: Some((t.colors.accent, darker(t.colors.accent))),
            opacity: 1.0,
            center: (0.0, 0.0, 0.0),
            enter: None,
        }
    }
}

impl Default for Surface {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Object3D for Surface {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let colors = match self.checkerboard_colors {
            Some((a, b)) => (a, Some(b)),
            None => (self.fill_color, None),
        };
        let (geometry, material) = parametric(
            self.func,
            self.u_range,
            self.v_range,
            self.resolution,
            colors,
        );
        let material = material.opacity(self.opacity);
        place(
            ui,
            scene,
            geometry,
            material,
            self.center,
            self.enter.as_ref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checkerboard_has_two_triangles_per_piece() {
        let (g, _) = parametric(
            flat,
            (0.0, 1.0),
            (0.0, 1.0),
            (4, 3),
            (Color::WHITE, Some(Color::BLACK)),
        );
        assert_eq!(g.triangle_count(), 24);
        let (one, _) = parametric(flat, (0.0, 1.0), (0.0, 1.0), (4, 3), (Color::WHITE, None));
        assert_eq!(one.triangle_count(), 24);
    }
}
