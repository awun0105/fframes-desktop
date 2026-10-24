use fframes::{
    Color, Svgr,
    three_d::{Geometry, Material, Quat, Scene, Vec3},
};

use crate::{
    label::Label,
    layout::Anchor,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Enter3D, Object3D, Point3, Projector, place, rgb, world};

/// Manim's `ThreeDAxes`: three axes through `center` with a tick at every `step` of their
/// ranges and a tip at the far end, labelled `x`, `y` and `z` past the tips. Each range
/// `(min, max, step)` is stretched to its length, the middle of the ranges at `center`.
/// The defaults are Manim's: ranges (-6, 6), (-5, 5) and (-4, 4), lengths 10.5, 10.5 and
/// 6.5. Map numbers to points with [`ThreeDAxes::c2p`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreeDAxes {
    pub x_range: (f32, f32, f32),
    pub y_range: (f32, f32, f32),
    pub z_range: (f32, f32, f32),
    pub x_length: f32,
    pub y_length: f32,
    pub z_length: f32,
    pub center: Point3,
    pub color: Color,
    /// The radius of the axis rods.
    pub thickness: f32,
    /// Half the length of a tick.
    pub tick_size: f32,
    pub tip_length: f32,
    pub show_labels: bool,
    pub label_fill: Color,
    pub enter: Option<Enter3D>,
}

impl Themed for ThreeDAxes {
    fn themed(t: &Theme) -> Self {
        Self {
            x_range: (-6.0, 6.0, 1.0),
            y_range: (-5.0, 5.0, 1.0),
            z_range: (-4.0, 4.0, 1.0),
            x_length: 10.5,
            y_length: 10.5,
            z_length: 6.5,
            center: (0.0, 0.0, 0.0),
            color: t.colors.text,
            thickness: 0.012,
            tick_size: 0.1,
            tip_length: 0.35,
            show_labels: true,
            label_fill: t.colors.title,
            enter: None,
        }
    }
}

impl Default for ThreeDAxes {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `v` of `range` stretched to `length`, the middle of the range at 0.
fn along((min, max, _): (f32, f32, f32), length: f32, v: f32) -> f32 {
    let span = max - min;
    if span.abs() < f32::EPSILON {
        0.0
    } else {
        (v - min.midpoint(max)) * length / span
    }
}

/// The values of `range` a tick marks, 0 left out, at most 200.
fn ticks((min, max, step): (f32, f32, f32)) -> impl Iterator<Item = f32> {
    let count = if step > 0.0 {
        ((max - min) / step).floor() as usize
    } else {
        0
    };
    (0..=count.min(200))
        .map(move |i| min + i as f32 * step)
        .filter(|v| v.abs() > 1e-6)
}

impl ThreeDAxes {
    /// The point of the numbers `(x, y, z)`, Manim's `c2p`.
    pub fn c2p(&self, x: f32, y: f32, z: f32) -> Point3 {
        let c = self.center;
        (
            c.0 + along(self.x_range, self.x_length, x),
            c.1 + along(self.y_range, self.y_length, y),
            c.2 + along(self.z_range, self.z_length, z),
        )
    }

    /// The axes as one mesh around `center`, in Manim's directions: a rod, ticks across it
    /// along `across` and a tip, for each axis.
    fn geometry(&self) -> Geometry {
        let o = self.c2p(0.0, 0.0, 0.0);
        let local = |p: Point3| world((p.0 - o.0, p.1 - o.1, p.2 - o.2));
        let mut parts = Vec::new();
        let axes = [
            (
                self.c2p(self.x_range.0, 0.0, 0.0),
                self.c2p(self.x_range.1, 0.0, 0.0),
                (0.0, 1.0, 0.0),
                self.x_range,
            ),
            (
                self.c2p(0.0, self.y_range.0, 0.0),
                self.c2p(0.0, self.y_range.1, 0.0),
                (1.0, 0.0, 0.0),
                self.y_range,
            ),
            (
                self.c2p(0.0, 0.0, self.z_range.0),
                self.c2p(0.0, 0.0, self.z_range.1),
                (1.0, 0.0, 0.0),
                self.z_range,
            ),
        ];
        for (index, (start, end, across, range)) in axes.into_iter().enumerate() {
            let (a, b) = (local(start), local(end));
            let length = a.distance(b);
            if length <= 1e-4 {
                continue;
            }
            let dir = (b - a) / length;
            let turn = Quat::from_rotation_arc(Vec3::Y, dir);
            let tip = self.tip_length.min(length / 2.0);
            let shaft = length - tip;
            parts.push(shifted(
                Geometry::cylinder(self.thickness, self.thickness, shaft, 8).rotate(turn),
                a + dir * shaft / 2.0,
            ));
            parts.push(shifted(
                Geometry::cone(tip * 0.35, tip, 16).rotate(turn),
                b - dir * tip / 2.0,
            ));
            let across = world(across);
            let tick_turn = Quat::from_rotation_arc(Vec3::Y, across);
            for v in ticks(range) {
                let at = match index {
                    0 => self.c2p(v, 0.0, 0.0),
                    1 => self.c2p(0.0, v, 0.0),
                    _ => self.c2p(0.0, 0.0, v),
                };
                parts.push(shifted(
                    Geometry::cylinder(
                        self.thickness * 0.8,
                        self.thickness * 0.8,
                        2.0 * self.tick_size,
                        6,
                    )
                    .rotate(tick_turn),
                    local(at),
                ));
            }
        }
        Geometry::merge(parts)
    }
}

/// `g` moved by `v`.
fn shifted(g: Geometry, v: Vec3) -> Geometry {
    g.translate(v.x, v.y, v.z)
}

impl Object3D for ThreeDAxes {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let material = Material::standard(rgb(self.color)).flat();
        let origin = self.c2p(0.0, 0.0, 0.0);
        place(
            ui,
            scene,
            self.geometry(),
            material,
            origin,
            self.enter.as_ref(),
        );
    }

    fn overlay<'a>(&self, ui: &mut Ui<'a, '_, '_>, projector: &Projector) -> Svgr<'a> {
        if !self.show_labels {
            return Svgr::empty();
        }
        let fade = self.enter.map_or(1.0, |e| {
            let (scale, opacity) = e.look(ui);
            if scale < 1.0 { 0.0 } else { opacity }
        });
        if fade <= 0.0 {
            return Svgr::empty();
        }
        let pad = 0.45;
        let ends = [
            ("x", self.c2p(self.x_range.1, 0.0, 0.0), (pad, 0.0, 0.0)),
            ("y", self.c2p(0.0, self.y_range.1, 0.0), (0.0, pad, 0.0)),
            ("z", self.c2p(0.0, 0.0, self.z_range.1), (0.0, 0.0, pad)),
        ];
        let labels: Vec<Svgr<'a>> = ends
            .into_iter()
            .filter_map(|(name, end, (dx, dy, dz))| {
                let at = projector.project((end.0 + dx, end.1 + dy, end.2 + dz))?;
                let label = Label {
                    text: name.into(),
                    x: at.x as f32,
                    y: at.y as f32,
                    direction: Anchor::Center,
                    style: ui.theme.typography.body,
                    fill: self.label_fill,
                    ..ui.defaults()
                };
                Some(ui.show(label))
            })
            .collect();
        fframes::svgr!(<g opacity={fade}>{labels}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c2p_centers_the_ranges() {
        let axes = ThreeDAxes::default();
        assert_eq!(axes.c2p(0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
        let (x, y, z) = axes.c2p(6.0, -5.0, 4.0);
        assert!((x - 5.25).abs() < 1e-5 && (y + 5.25).abs() < 1e-5 && (z - 3.25).abs() < 1e-5);
    }

    #[test]
    fn ticks_skip_zero() {
        assert_eq!(
            ticks((-2.0, 2.0, 1.0)).collect::<Vec<_>>(),
            [-2.0, -1.0, 1.0, 2.0]
        );
        assert_eq!(ticks((0.0, 1.0, 0.0)).count(), 0);
    }
}
