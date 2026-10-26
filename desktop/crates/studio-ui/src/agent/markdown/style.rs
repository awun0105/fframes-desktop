//! Colours and fonts of rendered markdown, derived from the resolved studio theme.

use gpui::{Hsla, Pixels, SharedString, px};
use studio_theme::Theme;

/// Size of code text, a notch under the body size.
pub const CODE_SIZE: f32 = 12.0;

/// The monospace family code is set in. The theme carries no code font.
pub const MONO_FAMILY: &str = if cfg!(target_os = "windows") {
    "Consolas"
} else if cfg!(target_os = "macos") {
    "Menlo"
} else {
    // DejaVu Sans Mono lacks several Vietnamese precomposed vowels and
    // mispositions their decomposed marks. Liberation Mono covers them.
    "Liberation Mono"
};

/// Size of a heading of `level` (1 to 6); the body is 13.
pub fn heading_size(level: u8) -> Pixels {
    match level {
        1 => px(18.),
        2 => px(16.),
        3 => px(14.5),
        _ => px(13.),
    }
}

/// The tokens markdown draws with.
#[derive(Clone, Debug)]
pub struct MdStyle {
    pub theme: Theme,
    pub text: Hsla,
    pub muted: Hsla,
    pub link: Hsla,
    pub code_bg: Hsla,
    pub code_border: Hsla,
    pub rule: Hsla,
    pub quote_bar: Hsla,
    pub selection: Hsla,
    pub mono: SharedString,
}

impl MdStyle {
    pub fn new(theme: &Theme) -> Self {
        let palette = theme.editor;
        Self {
            theme: *theme,
            text: palette.text_1.into(),
            muted: palette.text_2.into(),
            link: palette.accent.into(),
            code_bg: palette.card_2.into(),
            code_border: palette.line.into(),
            rule: palette.line.into(),
            quote_bar: palette.line_strong.into(),
            selection: gpui::hsla(211. / 360., 0.9, 0.55, 0.32),
            mono: SharedString::from(MONO_FAMILY),
        }
    }
}
