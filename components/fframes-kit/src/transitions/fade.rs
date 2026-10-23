use fframes::Svgr;

use crate::{
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side};

/// Fades the entering item in over the exiting one (Remotion's `fade()`). The exiting item
/// stays opaque unless `should_fade_out_exiting_scene` is set, so the entering item should
/// cover what is behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fade {
    pub should_fade_out_exiting_scene: bool,
}

impl Themed for Fade {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl Presentation for Fade {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        let opacity = match side {
            Side::Entering => clamp01(progress),
            Side::Exiting if self.should_fade_out_exiting_scene => 1.0 - clamp01(progress),
            Side::Exiting => return content,
        };
        fframes::svgr!(<g opacity={opacity}>{content}</g>)
    }
}
