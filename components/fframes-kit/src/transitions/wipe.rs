use fframes::Svgr;

use crate::{
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Direction, Presentation, Side};

/// The entering item is uncovered by an edge that moves across from `direction` (Remotion's
/// `wipe()`); the exiting item stays in place under it. `size` is the area in view-box
/// units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wipe {
    pub direction: Direction,
    pub size: (f32, f32),
}

impl Default for Wipe {
    fn default() -> Self {
        Self {
            direction: Direction::FromLeft,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for Wipe {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl Wipe {
    /// The uncovered rectangle `(x, y, width, height)` at `progress`.
    pub fn visible(&self, progress: f32) -> (f32, f32, f32, f32) {
        let p = clamp01(progress);
        let (w, h) = self.size;
        match self.direction {
            Direction::FromLeft => (0.0, 0.0, w * p, h),
            Direction::FromRight => (w * (1.0 - p), 0.0, w * p, h),
            Direction::FromTop => (0.0, 0.0, w, h * p),
            Direction::FromBottom => (0.0, h * (1.0 - p), w, h * p),
        }
    }
}

impl Presentation for Wipe {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        if side == Side::Exiting {
            return content;
        }
        let (x, y, width, height) = self.visible(progress);
        let url = format!("url(#{id})");
        let id = id.to_owned();
        fframes::svgr!(
            <g>
                <defs>
                    <clipPath id={id}>
                        <rect x={x} y={y} width={width} height={height} />
                    </clipPath>
                </defs>
                <g clip-path={url}>{content}</g>
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edge_moves_in_from_its_side() {
        let w = Wipe {
            direction: Direction::FromRight,
            ..Wipe::default()
        };
        assert_eq!(w.visible(0.0), (1920.0, 0.0, 0.0, 1080.0));
        assert_eq!(w.visible(0.5), (960.0, 0.0, 960.0, 1080.0));
        let w = Wipe {
            direction: Direction::FromTop,
            ..Wipe::default()
        };
        assert_eq!(w.visible(1.0), (0.0, 0.0, 1920.0, 1080.0));
    }
}
