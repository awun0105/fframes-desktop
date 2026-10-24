//! The DNA double helix in 3D.

use std::f32::consts::TAU;

use fframes::{
    Color, Svgr,
    three_d::{Geometry, Material, Node, Quat, Vec3, vec3},
};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    kits::stage::{fitting_camera, picture, rgb, stage},
    motion::{cubic_in_out, progress},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::dna::{BaseColors, complement};

/// B-DNA in nanometres: the rise per base pair, the radius of the backbone and ten pairs a
/// turn.
const RISE: f32 = 0.34;
const RADIUS: f32 = 1.0;
const PER_TURN: f32 = 10.0;
/// The angle between the two strands' backbones round the axis, which leaves a minor and a
/// major groove.
const GROOVE: f32 = 0.375 * TAU;

/// The two backbone points of base pair `index` of `count`, with the helix turned `turn`
/// radians: the axis runs along X through the origin, the first pair at the left.
fn backbone(index: usize, count: usize, turn: f32) -> (Vec3, Vec3) {
    let x = (index as f32 - (count as f32 - 1.0) / 2.0) * RISE;
    let angle = turn + index as f32 * TAU / PER_TURN;
    let at = |a: f32| vec3(x, RADIUS * a.cos(), RADIUS * a.sin());
    (at(angle), at(angle + GROOVE))
}

/// A cylinder from `a` to `b`.
fn rod(a: Vec3, b: Vec3, radius: f32, material: &Material) -> Option<Node> {
    let length = a.distance(b);
    if length <= 1e-4 {
        return None;
    }
    let middle = (a + b) / 2.0;
    let mut node = Node::mesh(
        Geometry::cylinder(radius, radius, length, 12),
        material.clone(),
    )
    .at(middle.x, middle.y, middle.z);
    node.rotation = Quat::from_rotation_arc(Vec3::Y, (b - a) / length);
    Some(node)
}

/// A DNA double helix in 3D, its axis across the image: `sequence` read from the left, each
/// pair a rung of its two bases in `bases` colours between the two backbones in
/// `strand_color`. It is rendered into a `width` x `height` pixel image around `center`
/// (units of `ui.space`), seen from `elevation` degrees above, and turns `spin` degrees a
/// second about its own axis. From `delay` the pairs build up from the left, one every
/// `pair_time` seconds. The image fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DnaHelix3D {
    pub sequence: String,
    pub delay: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub spin: f32,
    pub elevation: f32,
    pub pair_time: f32,
    pub bases: BaseColors,
    pub strand_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for DnaHelix3D {
    fn themed(t: &Theme) -> Self {
        Self {
            sequence: "ATGCGTACCTGAATCGGATCCATG".into(),
            delay: 0.0,
            center: (0.0, 0.0),
            width: 1500.0,
            height: 560.0,
            spin: 40.0,
            elevation: 12.0,
            pair_time: 0.12,
            bases: BaseColors::default(),
            strand_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for DnaHelix3D {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for DnaHelix3D {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let bases: Vec<char> = self.sequence.chars().collect();
        let count = bases.len();
        if count == 0 {
            return Svgr::empty();
        }
        let t = ui.seconds() - self.delay;
        let turn = (self.spin * ui.seconds()).to_radians();
        let mut scene = stage();
        let strand = Material::standard(rgb(self.strand_color))
            .roughness(0.3)
            .clearcoat(0.4);
        let mut previous: Option<(Vec3, Vec3)> = None;
        for (i, &base) in bases.iter().enumerate() {
            let start = i as f32 * self.pair_time;
            let grown = cubic_in_out(progress(t, start, start + 0.35));
            if grown <= 0.0 {
                break;
            }
            let (a, b) = backbone(i, count, turn);
            // the backbone: a bead at each nucleotide, joined to the one before
            for p in [a, b] {
                let bead = Geometry::sphere(0.17 * grown, 20, 10);
                scene.add(Node::mesh(bead, strand.clone()).at(p.x, p.y, p.z));
            }
            if let Some((pa, pb)) = previous {
                // the join grows from the previous bead
                let link = progress(t, start, start + 0.2);
                for (from, to) in [(pa, a), (pb, b)] {
                    if let Some(node) = rod(from, from.lerp(to, link), 0.08, &strand) {
                        scene.add(node);
                    }
                }
            }
            previous = Some((a, b));
            // the rung: each base reaches from its backbone to the middle
            let middle = a.lerp(b, 0.5);
            for (from, letter) in [(a, base), (b, complement(base))] {
                let material = Material::standard(rgb(self.bases.of(letter)))
                    .roughness(0.45)
                    .clearcoat(0.2);
                let reach = from.lerp(middle, grown * 0.97);
                if let Some(node) = rod(from, reach, 0.11, &material) {
                    scene.add(node);
                }
            }
        }
        // fit the whole length across the image and the diameter into its height
        let half = count as f32 * RISE / 2.0 + 0.3;
        // (with room for the perspective of the near end)
        let reach = (1.15 * half * self.height / self.width).max(RADIUS + 0.4);
        let camera = fitting_camera(
            Vec3::ZERO,
            reach,
            self.width,
            self.height,
            0.0,
            self.elevation,
        );
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let image = picture(&scene, &camera, middle, self.width, self.height);
        fade(ui, image, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_pairs_make_a_turn_on_the_backbone_radius() {
        let (a0, b0) = backbone(0, 24, 0.0);
        let (a10, _) = backbone(10, 24, 0.0);
        assert!((a0.y - a10.y).abs() < 1e-5 && (a0.z - a10.z).abs() < 1e-5);
        assert!((a10.x - a0.x - 10.0 * RISE).abs() < 1e-5);
        for p in [a0, b0] {
            assert!((p.y.hypot(p.z) - RADIUS).abs() < 1e-5);
        }
    }

    #[test]
    fn the_helix_is_centred_and_its_strands_leave_a_minor_groove() {
        let (first, _) = backbone(0, 24, 0.3);
        let (last, _) = backbone(23, 24, 0.3);
        assert!((first.x + last.x).abs() < 1e-5);
        // the strands are less than half a turn apart round the axis
        let (a, b) = backbone(3, 24, 0.0);
        let angle = (a.y * b.y + a.z * b.z).clamp(-1.0, 1.0).acos();
        assert!(angle < TAU / 2.0 && angle > TAU / 4.0);
    }
}
