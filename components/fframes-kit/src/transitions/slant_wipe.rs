use fframes::{Color, Svgr};

use crate::{
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side, presentation::clipped};

/// Width of the bright stripe on the leading edge and of the one on the trailing edge.
const EDGE: f32 = 26.0;
const BACK: f32 = 12.0;

/// A slanted panel of `color` sweeps across from the left with an `edge` stripe in front and
/// a `back` stripe behind. It covers the whole area at the middle of the transition, and the
/// entering item is behind it. `skew` is how far the top of the panel leans ahead of its
/// bottom; `size` is the area in view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlantWipe {
    pub color: Color,
    pub edge: Color,
    pub back: Color,
    pub skew: f32,
    pub size: (f32, f32),
}

impl Themed for SlantWipe {
    fn themed(t: &Theme) -> Self {
        Self {
            color: t.colors.background,
            edge: t.colors.highlight,
            back: t.colors.accent,
            skew: 320.0,
            size: (1920.0, 1080.0),
        }
    }
}

impl Default for SlantWipe {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl SlantWipe {
    /// The panel's width along its bottom: wide enough to cover the area at the middle.
    fn band(&self) -> f32 {
        self.size.0 + 2.5 * self.skew
    }

    /// The bottom corners of the panel's trailing and leading edges at `progress`. The
    /// panel starts left of the area and ends right of it.
    pub fn edges(&self, progress: f32) -> (f32, f32) {
        let band = self.band();
        // from the edge stripe left of the area to the back stripe right of it
        let travel = self.size.0 + band + self.skew + EDGE + BACK;
        let lead = -(self.skew + EDGE) + clamp01(progress) * travel;
        (lead - band, lead)
    }
}

/// A slanted band between bottom corners `from` and `to`, leaning `skew` to the right at the
/// top of an area `h` high.
fn band(from: f32, to: f32, skew: f32, h: f32) -> String {
    format!("M{from} {h}L{to} {h}L{} 0L{} 0Z", to + skew, from + skew)
}

impl Presentation for SlantWipe {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        if side == Side::Exiting || progress >= 1.0 {
            return content;
        }
        let (tail, lead) = self.edges(progress);
        let (skew, h) = (self.skew, self.size.1);
        // the entering item: everything left of the back stripe
        let left = (tail - BACK - 1.0).min(-1.0) - skew;
        let behind = band(left, tail - BACK, skew, h);
        let entering = clipped(content, fframes::svgr!(<path d={behind} />), id);
        let panel = band(tail, lead, skew, h);
        let edge = band(lead, lead + EDGE, skew, h);
        let back = band(tail - BACK, tail, skew, h);
        let (color, edge_color, back_color) = (self.color, self.edge, self.back);
        fframes::svgr!(
            <g>
                {entering}
                <path d={panel} fill={color} />
                <path d={edge} fill={edge_color} />
                <path d={back} fill={back_color} />
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_crosses_and_covers_the_middle() {
        let wipe = SlantWipe::default();
        let (w, skew) = (wipe.size.0, wipe.skew);
        // at the start the panel and its stripes are left of the area
        let (_, lead) = wipe.edges(0.0);
        assert!(lead + EDGE + skew <= 0.0);
        // in the middle it covers the area from corner to corner
        let (tail, lead) = wipe.edges(0.5);
        assert!(tail + skew < 0.0 && lead > w);
        // at the end it is right of the area
        let (tail, _) = wipe.edges(1.0);
        assert!(tail - BACK >= w);
    }
}
