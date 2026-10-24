use fframes::Svgr;

use crate::{
    anim::Anim,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// Its children blurred by a Gaussian of `radius` (the standard deviation, in view-box
/// units), which can be animated: a focus pull, a background pushed back behind a title.
/// A radius of 0 draws the children unfiltered. `key` names the filter when two blurs share a
/// frame.
///
/// A wrapper, unlike [`Glow`](crate::decor::Glow) and
/// [`DropShadow`](crate::decor::DropShadow): those are theme tokens drawn once as a
/// definition and named by many elements, a blur belongs to what it wraps.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Blur<'a> {
    pub radius: Anim<'a, f32>,
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Blur<'_> {
    /// A radius of 8, with no children.
    fn themed(_: &Theme) -> Self {
        Self {
            radius: Anim::Fixed(8.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Blur<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let radius = ui.sample(&self.radius);
        let content = (self.children)(ui);
        if radius <= 0.0 {
            return content;
        }
        let id = ui.id(self.key, 0, "blur");
        let url = format!("url(#{id})");
        fframes::svgr!(
            <g>
                <defs>
                    <filter id={id} x="-50%" y="-50%" width="200%" height="200%">
                        <feGaussianBlur stdDeviation={radius} color-interpolation-filters="sRGB" />
                    </filter>
                </defs>
                <g filter={url}>{content}</g>
            </g>
        )
    }
}
