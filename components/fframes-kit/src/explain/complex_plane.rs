//! Manim's `ComplexPlane`.

use std::borrow::Cow;

use fframes::Svgr;
use kurbo::Point;

use crate::{
    label::Label,
    layout::Anchor,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{NumberPlane, Space, axes::number};

/// A [`NumberPlane`] of complex numbers (Manim `ComplexPlane`): the real part along x, the
/// imaginary part along y, whose numbers are written `i`, `2i`, `-i`. Complex numbers are
/// `(re, im)` pairs; [`ComplexValueTracker`](crate::anim::ComplexValueTracker) animates one.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ComplexPlane<'a> {
    pub plane: NumberPlane<'a>,
}

impl Themed for ComplexPlane<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            plane: NumberPlane::themed(t),
        }
    }
}

impl Default for ComplexPlane<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `z = (re, im)` as Manim writes it: `2+3i`, `-i`, `4`, `0.5-1.5i`.
pub fn format_complex(z: (f64, f64), decimals: usize) -> String {
    let (re, im) = (number(z.0, decimals), number(z.1, decimals));
    let im_zero = im
        .trim_start_matches('-')
        .chars()
        .all(|c| c == '0' || c == '.');
    let re_zero = re.chars().all(|c| c == '0' || c == '.');
    if im_zero {
        return re;
    }
    let im = match im.as_str() {
        "1" => "i".to_owned(),
        "-1" => "-i".to_owned(),
        _ => format!("{im}i"),
    };
    if re_zero {
        im
    } else if im.starts_with('-') {
        format!("{re}{im}")
    } else {
        format!("{re}+{im}")
    }
}

/// The imaginary axis' numbers: `i`, `2i`, `-i`.
fn imaginary(v: f64, decimals: usize) -> String {
    format_complex((0.0, v), decimals)
}

impl ComplexPlane<'_> {
    /// The pixel of `z` in `space`.
    pub fn n2p_in(&self, space: Space, z: (f64, f64)) -> Point {
        self.plane.axes.map_in(space).c2p(z.0, z.1)
    }

    /// The pixel of the complex number `z` (Manim `number_to_point`, `n2p`).
    pub fn n2p(&self, ui: &Ui<'_, '_, '_>, z: (f64, f64)) -> (f32, f32) {
        let p = self.n2p_in(ui.space, z);
        (p.x as f32, p.y as f32)
    }

    /// The complex number at a pixel in `space`.
    pub fn p2n_in(&self, space: Space, p: Point) -> (f64, f64) {
        self.plane.axes.map_in(space).p2c(p)
    }

    /// The complex number at a pixel (Manim `point_to_number`, `p2n`).
    pub fn p2n(&self, ui: &Ui<'_, '_, '_>, x: f32, y: f32) -> (f64, f64) {
        self.p2n_in(ui.space, Point::new(f64::from(x), f64::from(y)))
    }

    /// `z` written as `a+bi` up and right of its point, with `decimals` decimals.
    pub fn label(&self, ui: &Ui<'_, '_, '_>, z: (f64, f64), decimals: usize) -> Label<'static> {
        let (x, y) = self.n2p(ui, z);
        Label {
            text: Cow::Owned(format_complex(z, decimals)),
            x,
            y,
            direction: Anchor::TopRight,
            ..ui.defaults()
        }
    }
}

impl<'a> Component<'a> for ComplexPlane<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.plane.render_with(ui, imaginary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_numbers_are_written_like_manim() {
        assert_eq!(format_complex((2.0, 3.0), 0), "2+3i");
        assert_eq!(format_complex((2.0, -3.0), 0), "2-3i");
        assert_eq!(format_complex((0.0, 1.0), 0), "i");
        assert_eq!(format_complex((0.0, -1.0), 0), "-i");
        assert_eq!(format_complex((1.0, -1.0), 0), "1-i");
        assert_eq!(format_complex((4.0, 0.0), 0), "4");
        assert_eq!(format_complex((0.0, -0.0), 0), "0");
        assert_eq!(format_complex((0.5, -1.5), 1), "0.5-1.5i");
        assert_eq!(format_complex((0.0, 0.04), 1), "0.0");
        assert_eq!(imaginary(2.0, 0), "2i");
    }

    #[test]
    fn points_and_numbers_round_trip() {
        let plane = ComplexPlane::default();
        for z in [(0.0, 0.0), (2.0, 3.0), (-1.5, 0.25), (7.0, -4.0)] {
            let back = plane.p2n_in(Space::FULL_HD, plane.n2p_in(Space::FULL_HD, z));
            assert!(
                (back.0 - z.0).abs() < 1e-9 && (back.1 - z.1).abs() < 1e-9,
                "{z:?}"
            );
        }
        // one unit is 135 pixels on Full HD, imaginary up
        assert_eq!(
            plane.n2p_in(Space::FULL_HD, (1.0, 1.0)),
            Point::new(960.0 + 135.0, 540.0 - 135.0)
        );
    }
}
