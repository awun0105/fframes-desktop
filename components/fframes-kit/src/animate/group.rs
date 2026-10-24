use fframes::Svgr;

use crate::{
    time::Sequence,
    ui::{Children, Component, Ui},
};

/// Starts each child `lag_ratio * run_time` seconds after the one before it (Manim's
/// `LaggedStart`). Each child's time starts at its own start, so its enter animations run
/// then; every child stays until the parent ends.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LaggedStart<'a> {
    /// Fraction of `run_time` between two starts.
    pub lag_ratio: f32,
    /// The run time of one child's animation.
    pub run_time: f32,
    pub children: Vec<Children<'a>>,
}

/// The start in seconds of child `i`.
fn lagged(i: usize, lag_ratio: f32, run_time: f32) -> f32 {
    i as f32 * lag_ratio * run_time
}

fn stagger<'a>(ui: &mut Ui<'a, '_, '_>, children: Vec<Children<'a>>, step: f32) -> Svgr<'a> {
    children
        .into_iter()
        .enumerate()
        .map(|(i, children)| {
            ui.show(Sequence {
                from: i as f32 * step,
                duration: None,
                children,
            })
        })
        .collect()
}

impl<'a> Component<'a> for LaggedStart<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let step = lagged(1, self.lag_ratio, self.run_time);
        stagger(ui, self.children, step)
    }
}

/// Every child at once (Manim's `AnimationGroup` with no lag).
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct AnimationGroup<'a> {
    pub children: Vec<Children<'a>>,
}

impl<'a> Component<'a> for AnimationGroup<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.children.iter().map(|c| c(ui)).collect()
    }
}

/// Each child starts when the one before it has run `run_time` seconds (Manim's
/// `Succession`, a `LaggedStart` with `lag_ratio` 1).
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Succession<'a> {
    pub run_time: f32,
    pub children: Vec<Children<'a>>,
}

impl<'a> Component<'a> for Succession<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        stagger(ui, self.children, self.run_time)
    }
}

#[cfg(test)]
mod tests {
    use super::lagged;

    #[test]
    fn lag_ratio_spaces_the_starts() {
        assert_eq!(lagged(0, 0.25, 2.0), 0.0);
        assert_eq!(lagged(3, 0.25, 2.0), 1.5);
    }
}
