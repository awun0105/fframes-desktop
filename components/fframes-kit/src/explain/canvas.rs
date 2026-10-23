use fframes::Svgr;

use crate::ui::{Children, Component, Ui};

use super::Space;

/// Content drawn in another coordinate system: its children see `space` as `ui.space`
/// (Manim's frame, moved and scaled), for a graph in one half of the screen or a figure in a
/// card.
pub struct Canvas<'a> {
    pub space: Space,
    pub children: Children<'a>,
}

impl<'a> Component<'a> for Canvas<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let mut inner = ui.with_space(self.space);
        (self.children)(&mut inner)
    }
}
