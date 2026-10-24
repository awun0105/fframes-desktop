use fframes::Svgr;

use crate::{
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

use super::Space;

/// Content drawn in another coordinate system: its children see `space` as `ui.space`
/// (Manim's frame, moved and scaled), for a graph in one half of the screen or a figure in a
/// card.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Canvas<'a> {
    pub space: Space,
    pub children: Children<'a>,
}

impl Themed for Canvas<'_> {
    /// Manim's frame on a 1920x1080 canvas, with no children.
    fn themed(_: &Theme) -> Self {
        Self {
            space: Space::FULL_HD,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Canvas<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let mut inner = ui.with_space(self.space);
        (self.children)(&mut inner)
    }
}
