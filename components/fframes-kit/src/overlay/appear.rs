use fframes::Svgr;

use crate::{
    anim::RateFunc,
    motion::progress,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// Shows its children from `from` seconds on: they rise `rise` units into place, grow from
/// `scale_from` about `center` and fade in over `enter` seconds. They leave the same way up
/// over `exit` seconds, ending at `until` seconds, or at the end of the container
/// (`ui.duration`) when `until` is `None`. Inside, time starts at `from`, as in a `Sequence`.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Appear<'a> {
    pub from: f32,
    pub until: Option<f32>,
    /// The point the content scales about, in canvas units.
    pub center: (f32, f32),
    pub rise: f32,
    pub scale_from: f32,
    pub enter: f32,
    pub exit: f32,
    pub rate_func: RateFunc,
    pub children: Children<'a>,
}

impl Themed for Appear<'_> {
    /// From 0 seconds, scaling about the middle of a 1920x1080 canvas, with no children.
    fn themed(t: &Theme) -> Self {
        Self::with_theme(t, 0.0, (960.0, 540.0), no_children())
    }
}

impl<'a> Appear<'a> {
    /// Times and motion from the theme: `motion.duration.enter` and `exit`, a 60 unit rise,
    /// scaling from 0.92.
    pub fn new(ui: &Ui<'a, '_, '_>, from: f32, center: (f32, f32), children: Children<'a>) -> Self {
        Self::with_theme(ui.theme, from, center, children)
    }

    fn with_theme(t: &Theme, from: f32, center: (f32, f32), children: Children<'a>) -> Self {
        Self {
            from,
            until: None,
            center,
            rise: 60.0,
            scale_from: 0.92,
            enter: t.motion.duration.enter,
            exit: t.motion.duration.exit,
            rate_func: t.motion.easing.enter,
            children,
        }
    }

    /// `(entered, left)`, both from 0 to 1, at `t` seconds of a container that ends at `end`.
    pub fn state(&self, t: f32, end: Option<f32>) -> (f32, f32) {
        let entered = self
            .rate_func
            .apply(progress(t, self.from, self.from + self.enter), self.enter);
        let left = match self.until.or(end) {
            Some(stop) => progress(t, stop - self.exit, stop),
            None => 0.0,
        };
        (entered, left)
    }
}

impl<'a> Component<'a> for Appear<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let start = ui.frames(self.from);
        if ui.frame.index < start {
            return Svgr::empty();
        }
        let (entered, left) = self.state(ui.seconds(), ui.duration_seconds());
        let opacity = entered.min(1.0) * (1.0 - left);
        if opacity <= 0.0 {
            return Svgr::empty();
        }
        let length = match self.until {
            Some(until) => Some(ui.frames(until).saturating_sub(start)),
            None => ui.duration.map(|d| d.saturating_sub(start)),
        };
        let content = (self.children)(&mut ui.shifted(start, length));
        let (x, y) = self.center;
        let dy = (1.0 - entered) * self.rise - left * self.rise;
        let scale = self.scale_from + (1.0 - self.scale_from) * entered;
        let transform = format!(
            "translate({x} {}) scale({scale}) translate({} {})",
            y + dy,
            -x,
            -y
        );
        fframes::svgr!(<g transform={transform} opacity={opacity}>{content}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enters_then_leaves_before_the_end() {
        let a = Appear {
            rate_func: RateFunc::Linear,
            ..Appear::with_theme(
                &Theme::default(),
                1.0,
                (0.0, 0.0),
                crate::ui::children(|_| Svgr::empty()),
            )
        };
        let enter = a.enter;
        assert_eq!(a.state(1.0, Some(5.0)), (0.0, 0.0));
        assert_eq!(a.state(1.0 + enter, Some(5.0)).0, 1.0);
        assert_eq!(a.state(5.0, Some(5.0)).1, 1.0);
        assert_eq!(a.state(5.0 - a.exit, Some(5.0)).1, 0.0);
        let until = Appear {
            until: Some(3.0),
            ..a
        };
        assert_eq!(until.state(3.0, Some(5.0)).1, 1.0);
        assert_eq!(until.state(100.0, None).1, 1.0);
    }
}
