use std::borrow::Cow;

use fframes::Svgr;

use crate::{
    theme::{Shadow, Theme},
    ui::{Component, Themed, Ui},
};

/// A `<filter>` that puts a blurred copy of the element's shape in `shadow.color` under it,
/// offset by `shadow.dx`/`dy`.
fn shadow_filter<'a>(id: &str, shadow: Shadow) -> Svgr<'a> {
    let id = id.to_owned();
    let color = shadow.color.with_alpha(255);
    let opacity = f32::from(shadow.color.a) / 255.0;
    fframes::svgr!(
        <filter id={id} x="-50%" y="-50%" width="200%" height="200%" color-interpolation-filters="sRGB">
            <feGaussianBlur in="SourceAlpha" stdDeviation={shadow.blur} />
            <feOffset dx={shadow.dx} dy={shadow.dy} result="offsetblur" />
            <feFlood flood-color={color} flood-opacity={opacity} />
            <feComposite in2="offsetblur" operator="in" />
            <feMerge>
                <feMergeNode />
                <feMergeNode in="SourceGraphic" />
            </feMerge>
        </filter>
    )
}

macro_rules! filter {
    ($(#[$meta:meta])* $name:ident, $token:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
        pub struct $name<'a> {
            /// The filter's id, unique in the frame.
            pub id: Cow<'a, str>,
            pub shadow: Shadow,
        }

        impl Themed for $name<'_> {
            fn themed(t: &Theme) -> Self {
                Self { id: Cow::Borrowed(concat!("kit-", stringify!($token))), shadow: t.shadow.$token }
            }
        }

        impl Default for $name<'_> {
            fn default() -> Self {
                Self::themed(&Theme::default())
            }
        }

        impl $name<'_> {
            /// `url(#id)`, the value of a `filter` attribute.
            pub fn url(&self) -> String {
                format!("url(#{})", self.id)
            }
        }

        impl<'a> Component<'a> for $name<'a> {
            fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
                shadow_filter(&self.id, self.shadow)
            }
        }
    };
}

filter!(
    /// The theme's `shadow.glow` as a filter definition: a soft halo in the accent colour.
    /// Draw it once, then use `filter={glow.url()}` on what should glow.
    Glow,
    glow
);

filter!(
    /// The theme's `shadow.card` as a filter definition: the drop shadow under a card.
    /// Draw it once, then use `filter={shadow.url()}` on the card.
    DropShadow,
    card
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_read_the_theme_tokens() {
        let t = Theme::default();
        assert_eq!(Glow::themed(&t).shadow, t.shadow.glow);
        assert_eq!(DropShadow::themed(&t).shadow, t.shadow.card);
        assert_eq!(Glow::default().url(), "url(#kit-glow)");
    }
}
