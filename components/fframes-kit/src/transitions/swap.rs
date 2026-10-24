use fframes::{Color, Svgr};
use kurbo::Affine;

use crate::{
    geometry::matrix,
    motion::{clamp01, cubic_in_out},
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side, presentation::bell};

/// The two items trade places like cards on a floor, each with its reflection (Remotion's
/// `swap()`). The mapping is `swap()` to `SwapCards`: the name `Swap` is Manim's animation.
///
/// A 2D approximation of the GL original, which moves the items through 3D space: here
/// the exiting card shrinks and slides out to the left while the entering card grows in
/// from the right, edge to edge, so neither covers the other. `depth` is how far they
/// recede (3 shrinks them to half at the far end), `perspective` skews them as they turn
/// (0.2 is about 9 degrees in the middle), and `reflection` is the opacity of their mirror
/// images below them on a `floor`, which the exiting side draws behind both. `size` is the
/// area in view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwapCards {
    pub reflection: f32,
    pub perspective: f32,
    pub depth: f32,
    pub floor: Color,
    pub size: (f32, f32),
}

impl Themed for SwapCards {
    /// Remotion's reflection, perspective and depth on the theme's background.
    fn themed(t: &Theme) -> Self {
        Self {
            reflection: 0.4,
            perspective: 0.2,
            depth: 3.0,
            floor: t.colors.background,
            size: (1920.0, 1080.0),
        }
    }
}

impl Default for SwapCards {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl SwapCards {
    /// The scale of a card at the far end of the swap.
    fn far(&self) -> f32 {
        let depth = self.depth.max(0.0);
        1.0 - depth / (depth + 3.0)
    }

    /// The scale and the horizontal offset of `side`'s card at `progress`. The cards stay
    /// edge to edge.
    pub fn card(&self, progress: f32, side: Side) -> (f32, f32) {
        let p = cubic_in_out(clamp01(progress));
        let shrink = 1.0 - self.far();
        let travel = self.size.0 * (1.0 - shrink / 2.0);
        match side {
            Side::Exiting => (1.0 - shrink * p, -travel * p),
            Side::Entering => (1.0 - shrink * (1.0 - p), travel * (1.0 - p)),
        }
    }
}

impl Presentation for SwapCards {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        let p = clamp01(progress);
        if (side == Side::Entering && p >= 1.0) || (side == Side::Exiting && p <= 0.0) {
            return content;
        }
        let (w, h) = self.size;
        let (scale, dx) = self.card(p, side);
        let (cx, cy) = (f64::from(w / 2.0 + dx), f64::from(h / 2.0));
        let lean = f64::from(self.perspective * bell(p)) * std::f64::consts::FRAC_PI_4;
        let lean = match side {
            Side::Exiting => lean.tan(),
            Side::Entering => -lean.tan(),
        };
        // a card: scaled about its centre, moved, leaning by a vertical skew
        let place = Affine::translate((cx, cy))
            * Affine::new([1.0, lean, 0.0, 1.0, 0.0, 0.0])
            * Affine::scale(f64::from(scale))
            * Affine::translate((-f64::from(w) / 2.0, -f64::from(h) / 2.0));
        // its mirror image below its bottom edge
        let mirror = place
            * Affine::translate((0.0, 2.0 * f64::from(h)))
            * Affine::scale_non_uniform(1.0, -1.0);
        let card = matrix(place);
        let image = matrix(mirror);
        let reflection = clamp01(self.reflection);
        let reflected = content.clone();
        let floor = (side == Side::Exiting)
            .then(|| fframes::svgr!(<rect x="0" y="0" width={w} height={h} fill={self.floor} />));
        fframes::svgr!(
            <g>
                {floor.unwrap_or_default()}
                <g transform={image} opacity={reflection}>{reflected}</g>
                <g transform={card}>{content}</g>
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_stay_edge_to_edge() {
        let swap = SwapCards::default();
        assert_eq!(swap.card(0.0, Side::Exiting), (1.0, 0.0));
        let (w, mid) = (swap.size.0, swap.size.0 / 2.0);
        for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let (se, de) = swap.card(p, Side::Exiting);
            let (sn, dn) = swap.card(p, Side::Entering);
            let right = mid + de + se * w / 2.0;
            let left = mid + dn - sn * w / 2.0;
            assert!((right - left).abs() < 1e-2, "{p}: {right} {left}");
        }
        // the entering card starts just off the canvas and ends in place
        let (s, d) = swap.card(0.0, Side::Entering);
        assert!((mid + d - s * w / 2.0 - w).abs() < 1e-3);
        assert_eq!(swap.card(1.0, Side::Entering), (1.0, 0.0));
    }
}
