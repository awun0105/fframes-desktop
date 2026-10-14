//! Text styles: size, weight and line height.

use gpui::{FontWeight, Styled, px};

/// System UI font on every platform; no bundled font.
pub const UI_FONT: &str = ".SystemUIFont";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    /// 10, medium: badges and key chips.
    Caption,
    /// 11: inline values.
    Small,
    /// 12, medium: labels, segmented items, tooltips, menus.
    Label,
    /// 13: body, buttons, fields.
    Body,
    /// 13, semibold.
    BodyStrong,
    /// 14, medium: card titles.
    Title,
    /// 16, semibold: the one size fframes adds, for the empty-state title.
    Heading,
}

impl TextStyle {
    pub const fn size(self) -> f32 {
        match self {
            Self::Caption => 10.,
            Self::Small => 11.,
            Self::Label => 12.,
            Self::Body | Self::BodyStrong => 13.,
            Self::Title => 14.,
            Self::Heading => 16.,
        }
    }

    pub fn weight(self) -> FontWeight {
        match self {
            Self::Caption | Self::Label | Self::Title => FontWeight::MEDIUM,
            Self::Small | Self::Body => FontWeight::NORMAL,
            Self::BodyStrong | Self::Heading => FontWeight::SEMIBOLD,
        }
    }

    pub const fn line_height(self) -> f32 {
        match self {
            Self::Caption => 14.,
            Self::Small | Self::Label => 16.,
            Self::Body | Self::BodyStrong => 18.,
            Self::Title => 20.,
            Self::Heading => 22.,
        }
    }
}

/// Applies size, weight and line height of `style`.
pub fn text_style<E: Styled>(element: E, style: TextStyle) -> E {
    element
        .text_size(px(style.size()))
        .font_weight(style.weight())
        .line_height(px(style.line_height()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_styles_match_cap_sizes() {
        let sizes = [
            TextStyle::Caption,
            TextStyle::Small,
            TextStyle::Label,
            TextStyle::Body,
            TextStyle::BodyStrong,
            TextStyle::Title,
            TextStyle::Heading,
        ]
        .map(TextStyle::size);
        assert_eq!(sizes, [10., 11., 12., 13., 13., 14., 16.]);
    }
}
