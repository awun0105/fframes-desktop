use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::draw_animated,
    geometry::Shape,
    shapes::Dot,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::clock;

/// Manim-physics' `RadialWave`: circular waves from every point of `sources` (units of
/// `ui.space`), `amplitude sin(k r - ω t)` each, summed where they meet. `wavelength` and
/// `speed` are in units and units per second. In 2D the crests are rings in `color` and the
/// troughs dashed rings in `trough_color` (with `troughs`), drawn inside `x_range` by
/// `y_range`; where the rings of two sources cross the waves add up. With `front` (the
/// default) the water is at rest at the start and the rings leave the sources on the
/// figure's clock (`delay`, `time_scale`); without it they fill the area from the start.
///
/// With the `3d` feature it is an [`Object3D`](crate::three::Object3D) as well: put it into a
/// [`ThreeDView`](crate::three::ThreeDView) and the same sum is a surface of `resolution` by
/// `resolution` pieces over the two ranges, `amplitude` units high, like Manim-physics'
/// surface. There only `enter` applies, about the middle of the area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RadialWave {
    pub sources: Vec<(f64, f64)>,
    pub wavelength: f64,
    pub speed: f64,
    pub amplitude: f64,
    pub front: bool,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub troughs: bool,
    pub show_sources: bool,
    pub resolution: usize,
    pub delay: f32,
    pub time_scale: f32,
    pub color: Color,
    pub trough_color: Color,
    pub source_color: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for RadialWave {
    fn themed(t: &Theme) -> Self {
        Self {
            sources: vec![(0.0, 0.0)],
            wavelength: 1.0,
            speed: 1.0,
            amplitude: 0.3,
            front: true,
            x_range: (-7.0, 7.0),
            y_range: (-4.0, 4.0),
            troughs: true,
            show_sources: true,
            resolution: 96,
            delay: 0.0,
            time_scale: 1.0,
            color: t.colors.accent,
            trough_color: t.colors.muted,
            source_color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
        }
    }
}

impl Default for RadialWave {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl RadialWave {
    /// The height of the water at `(x, y)` after `t` seconds: the sum over the sources of
    /// `amplitude sin(k r - ω t)`, without the sources the front has not reached yet.
    pub fn displacement(&self, x: f64, y: f64, t: f64) -> f64 {
        if self.wavelength <= 0.0 {
            return 0.0;
        }
        let k = TAU / self.wavelength;
        let reach = self.speed * t;
        self.sources
            .iter()
            .map(|&(sx, sy)| {
                let r = (x - sx).hypot(y - sy);
                if self.front && r > reach {
                    0.0
                } else {
                    self.amplitude * (k * (r - reach)).sin()
                }
            })
            .sum()
    }

    /// The radii around a source where its own wave is at `phase` of a wavelength (a
    /// quarter for the crests, three quarters for the troughs) after `t` seconds, out to the
    /// front or to `limit`.
    fn rings(&self, t: f64, phase: f64, limit: f64) -> Vec<f64> {
        let l = self.wavelength;
        if l <= 0.0 {
            return Vec::new();
        }
        let reach = self.speed * t;
        let limit = if self.front { limit.min(reach) } else { limit };
        let first = (reach + phase * l).rem_euclid(l);
        if limit.is_nan() || limit < first {
            return Vec::new();
        }
        // a few thousand rings at most, more could not be told apart
        let count = ((limit - first) / l).floor().min(4096.0) as u32 + 1;
        (0..count)
            .map(|i| first + f64::from(i) * l)
            .filter(|&r| r > 0.0)
            .collect()
    }

    fn inside(&self, (x, y): (f64, f64)) -> bool {
        (self.x_range.0..=self.x_range.1).contains(&x)
            && (self.y_range.0..=self.y_range.1).contains(&y)
    }

    /// The farthest corner of the area from `(x, y)`.
    fn farthest(&self, (x, y): (f64, f64)) -> f64 {
        [self.x_range.0, self.x_range.1]
            .into_iter()
            .flat_map(|cx| [self.y_range.0, self.y_range.1].map(|cy| (cx - x).hypot(cy - y)))
            .fold(0.0, f64::max)
    }
}

/// The circle of `radius` units around `center`, only its parts inside `area`.
fn clipped_ring(
    ui: &Ui<'_, '_, '_>,
    wave: &RadialWave,
    center: (f64, f64),
    radius: f64,
) -> BezPath {
    let pixels = radius * f64::from(ui.space.unit);
    let n = (pixels * TAU / 6.0).clamp(24.0, 720.0) as u32;
    let mut path = BezPath::new();
    let mut drawing = false;
    for i in 0..=n {
        let a = TAU * f64::from(i) / f64::from(n);
        let p = (center.0 + radius * a.cos(), center.1 + radius * a.sin());
        if !wave.inside(p) {
            drawing = false;
            continue;
        }
        let px: Point = ui.space.to_px(p.0, p.1);
        if drawing {
            path.line_to(px);
        } else {
            path.move_to(px);
            drawing = true;
        }
    }
    path
}

impl<'a> Component<'a> for RadialWave {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = clock(ui, self.delay, self.time_scale);
        let thin = ui.theme.stroke.thin;
        let mut shapes = Vec::new();
        let mut kinds = vec![(0.25, self.color, self.stroke_width, None)];
        if self.troughs {
            kinds.push((0.75, self.trough_color, thin, Some((6.0, 8.0))));
        }
        for &source in &self.sources {
            let limit = self.farthest(source);
            for &(phase, color, width, dash) in &kinds {
                for r in self.rings(t, phase, limit) {
                    let path = clipped_ring(ui, &self, source, r);
                    if path.elements().is_empty() {
                        continue;
                    }
                    shapes.push(Shape {
                        path,
                        fill: None,
                        stroke: Some(color),
                        stroke_width: width,
                        dash,
                        opacity: 1.0,
                    });
                }
            }
        }
        if self.show_sources {
            for &(x, y) in &self.sources {
                let at = ui.space.to_px(x, y);
                shapes.push(
                    Dot {
                        cx: at.x as f32,
                        cy: at.y as f32,
                        radius: 8.0,
                        fill: Some(self.source_color),
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
        }
        let waves = draw_animated(ui, &shapes, None, None, None);
        fade(ui, waves, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(feature = "3d")]
impl crate::three::Object3D for RadialWave {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut fframes::three_d::Scene) {
        use crate::three::{Enter3D, darker, parametric, place};

        let t = clock(ui, self.delay, self.time_scale);
        let (cx, cy) = (
            self.x_range.0.midpoint(self.x_range.1),
            self.y_range.0.midpoint(self.y_range.1),
        );
        let height = |u: f32, v: f32| {
            let (x, y) = (f64::from(u) + cx, f64::from(v) + cy);
            (u, v, self.displacement(x, y, t) as f32)
        };
        let half = |(a, b): (f64, f64)| (a - b).abs() as f32 / 2.0;
        let (hx, hy) = (half(self.x_range), half(self.y_range));
        let (geometry, material) = parametric(
            height,
            (-hx, hx),
            (-hy, hy),
            (self.resolution, self.resolution),
            (self.color, Some(darker(self.color))),
        );
        let enter = self.enter.map(Enter3D::from);
        place(
            ui,
            scene,
            geometry,
            material,
            (cx as f32, cy as f32, 0.0),
            enter.as_ref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crest_rings_are_crests() {
        let wave = RadialWave {
            sources: vec![(1.0, -0.5)],
            wavelength: 1.5,
            speed: 2.0,
            front: false,
            ..RadialWave::default()
        };
        let t = 1.3;
        let crests = wave.rings(t, 0.25, 6.0);
        let troughs = wave.rings(t, 0.75, 6.0);
        assert_eq!(crests.len(), 4);
        for r in crests {
            assert!((wave.displacement(1.0 + r, -0.5, t) - wave.amplitude).abs() < 1e-9);
        }
        for r in troughs {
            assert!((wave.displacement(1.0, -0.5 + r, t) + wave.amplitude).abs() < 1e-9);
        }
    }

    #[test]
    fn two_sources_interfere() {
        let wave = RadialWave {
            sources: vec![(-1.0, 0.0), (1.0, 0.0)],
            wavelength: 1.0,
            front: false,
            ..RadialWave::default()
        };
        let one = RadialWave {
            sources: vec![(-1.0, 0.0)],
            ..wave.clone()
        };
        // equally far from both: twice one source
        for t in [0.0, 0.3, 0.71] {
            let both = wave.displacement(0.0, 2.0, t);
            assert!((both - 2.0 * one.displacement(0.0, 2.0, t)).abs() < 1e-9);
        }
        // half a wavelength farther from one than the other: they cancel
        for t in [0.0, 0.3, 0.71] {
            assert!(wave.displacement(0.25, 0.0, t).abs() < 1e-9);
        }
    }

    #[test]
    fn the_front_leaves_the_water_at_rest_ahead_of_it() {
        let wave = RadialWave::default();
        let t = 2.0;
        assert_eq!(wave.displacement(2.5, 0.0, t), 0.0);
        assert!(wave.displacement(2.0 - 1e-7, 0.0, t).abs() < 1e-5);
        assert!(wave.rings(t, 0.25, 10.0).iter().all(|&r| r <= 2.0));
        // nothing before it starts
        assert_eq!(wave.rings(0.0, 0.25, 10.0), Vec::<f64>::new());
    }
}
