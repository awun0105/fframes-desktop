//! Colour tokens, metrics and type for the fframes studio chrome.
//!
//! The palette is Cap's, transcribed from the web app's resolved values: the
//! Radix scales plus the editor's layered `--ed-*` surfaces. A few dark-mode
//! entries are Cap's `theme.css` overrides rather than stock Radix, so
//! regenerating this from a Radix crate would silently change the palette.
//!
//! Video style presets belong to project content; this crate owns only the
//! studio's own surfaces. A view resolves a [`Theme`] once per render with
//! [`Theme::for_window`] and passes it down.

use gpui::{BoxShadow, Hsla, Rgba, WindowAppearance, point, px};

mod appearance;
#[cfg(test)]
mod contrast;
mod elevation;
mod metrics;
mod motion;
mod palette;
mod typography;

pub use appearance::{bind_window, current_preference, init, resolve_appearance, set_preference};
pub use elevation::{Shadow, shadow_for};
pub use metrics::{METRICS, Metrics};
pub use motion::{FAST_MS, NORMAL_MS, reduced_motion};
pub use palette::{SegmentColors, TrackKind, hex, hexa, segment_colors_for, track_color};
pub use studio_proto::AppTheme;
pub use typography::{TextStyle, UI_FONT, text_style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub fn from_window(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
        }
    }
}

/// Every colour the studio draws with, already resolved for one appearance.
///
/// The scales are transcribed whole rather than trimmed to today's callers:
/// half a Radix scale sends the next caller back to the CSS to re-derive which
/// entries were overridden.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub appearance: Appearance,

    pub gray_1: Rgba,
    pub gray_2: Rgba,
    pub gray_3: Rgba,
    pub gray_4: Rgba,
    pub gray_5: Rgba,
    pub gray_6: Rgba,
    pub gray_7: Rgba,
    pub gray_8: Rgba,
    pub gray_9: Rgba,
    pub gray_10: Rgba,
    pub gray_11: Rgba,
    pub gray_12: Rgba,

    pub blue_3: Rgba,
    pub blue_4: Rgba,
    pub blue_5: Rgba,
    pub blue_6: Rgba,
    pub blue_8: Rgba,
    pub blue_9: Rgba,
    pub blue_10: Rgba,
    pub blue_11: Rgba,
    pub blue_12: Rgba,

    pub red_2: Rgba,
    /// Danger surfaces (error notices, destructive hover washes).
    pub red_3: Rgba,
    pub red_4: Rgba,
    pub red_9: Rgba,
    pub red_10: Rgba,
    /// Danger text and glyphs.
    pub red_11: Rgba,

    /// Warning surface.
    pub amber_3: Rgba,
    pub amber_6: Rgba,
    /// Warning text and glyphs.
    pub amber_11: Rgba,

    /// Success surface. Stock Radix green; Cap's palette carries no green.
    pub green_3: Rgba,
    /// Success text and glyphs.
    pub green_11: Rgba,

    /// `--blue-100`, a legacy step distinct from Radix blue.
    pub blue_100: Rgba,
    /// `--blue-500`: selection fills and rings. Not interchangeable with `blue_9`.
    pub blue_500: Rgba,
    /// `--red-300`. Same value in both themes.
    pub red_300: Rgba,

    /// The legacy gray scale (`--gray-200 … --gray-500`), which is *not*
    /// numerically aligned with Radix: legacy `gray-500` is the darkest text,
    /// Radix `gray-1` the lightest surface.
    pub gray_200_legacy: Rgba,
    pub gray_300_legacy: Rgba,
    pub gray_400_legacy: Rgba,
    pub gray_500_legacy: Rgba,

    /// `--text-primary`, the root text colour. Slightly translucent by design.
    pub text_primary: Rgba,

    /// The editor's layered surface system (`--ed-*`).
    pub editor: EditorPalette,
}

/// Editor surfaces: a layered, low-contrast system (window < card < control)
/// with one accent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorPalette {
    pub window: Rgba,
    pub card: Rgba,
    pub card_2: Rgba,
    pub stage: Rgba,
    pub line: Rgba,
    pub line_strong: Rgba,
    pub text_1: Rgba,
    pub text_2: Rgba,
    pub text_3: Rgba,
    pub ctl: Rgba,
    pub ctl_hover: Rgba,
    pub ctl_active: Rgba,
    pub accent: Rgba,
    pub accent_2: Rgba,
    pub playhead: Rgba,
    pub thumb: Rgba,
    /// `--ed-seg-tint`: how much of a track colour a segment fill carries.
    pub seg_tint: f32,
    /// `--ed-seg-line`: the segment border's alpha.
    pub seg_line: f32,
    /// Segment text is the track colour pulled this far toward black (light)
    /// or white (dark).
    pub seg_label_base: Rgba,
    pub seg_label_mix: f32,
    /// How black each surface's drop shadow is: a card lifts barely at all on
    /// white and much more on the dark window, where the hairline alone is
    /// invisible.
    pub card_shadow_alpha: f32,
    pub pop_shadow_alpha: f32,
    pub pop_shadow_offset: f32,
    pub pop_shadow_blur: f32,
}

impl EditorPalette {
    /// `color-mix(in srgb, color <tint>, card)`: the segment fill.
    pub fn seg_fill(&self, color: Rgba) -> Rgba {
        mix(self.card, color, self.seg_tint)
    }

    /// A selected segment: the tint plus ten points.
    pub fn seg_fill_selected(&self, color: Rgba) -> Rgba {
        mix(self.card, color, self.seg_tint + 0.10)
    }

    /// The segment's 1px inset border.
    pub fn seg_border(&self, color: Rgba) -> Rgba {
        rgba_alpha(color, self.seg_line)
    }

    /// Segment label text.
    pub fn seg_label(&self, color: Rgba) -> Rgba {
        mix(self.seg_label_base, color, self.seg_label_mix)
    }

    /// Segment secondary text: the label colour at 45% toward `text_3`.
    pub fn seg_muted(&self, color: Rgba) -> Rgba {
        mix(self.text_3, color, 0.45)
    }

    /// A track gutter's icon tile: `color 16%` over transparent.
    pub fn tile_bg(&self, color: Rgba) -> Rgba {
        rgba_alpha(color, 0.16)
    }

    /// The tile's glyph: the track colour pulled 62% toward the label base.
    pub fn tile_fg(&self, color: Rgba) -> Rgba {
        mix(self.seg_label_base, color, 0.62)
    }

    /// Trim handles: the colour at 70% toward the label base.
    pub fn seg_handle(&self, color: Rgba) -> Rgba {
        mix(self.seg_label_base, color, 0.70)
    }

    /// `--ed-card-shadow`: `0 1px 2px` under every editor card.
    pub fn card_shadow(&self) -> Vec<BoxShadow> {
        vec![BoxShadow {
            color: gpui::hsla(0., 0., 0., self.card_shadow_alpha),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: false,
        }]
    }

    /// `--ed-pop-shadow`: the lift under a menu, popover or dialog.
    pub fn pop_shadow(&self) -> Vec<BoxShadow> {
        vec![BoxShadow {
            color: gpui::hsla(0., 0., 0., self.pop_shadow_alpha),
            offset: point(px(0.), px(self.pop_shadow_offset)),
            blur_radius: px(self.pop_shadow_blur),
            spread_radius: px(-12.),
            inset: false,
        }]
    }
}

/// The drop shadow under the stage's letterboxed preview. The stage is nearly
/// black in both themes, so this one does not vary with the appearance.
pub fn preview_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: gpui::hsla(0., 0., 0., 0.35),
        offset: point(px(0.), px(12.)),
        blur_radius: px(32.),
        spread_radius: px(-8.),
        inset: false,
    }]
}

/// The lift under a selected segmented-control pill on the light editor
/// surface.
pub fn raised_pill_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.12),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.06),
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(0.5),
            inset: false,
        },
    ]
}

/// The drop shadow under a slider knob.
pub fn thumb_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: gpui::hsla(0., 0., 0., 0.25),
        offset: point(px(0.), px(1.)),
        blur_radius: px(3.),
        spread_radius: px(0.),
        inset: false,
    }]
}

/// `color-mix(in srgb, b <t>, a)`: straight sRGB interpolation, which is what
/// the CSS side does.
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

pub fn rgba_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba { a: alpha, ..color }
}

impl Theme {
    pub fn new(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Light => Self::light(),
            Appearance::Dark => Self::dark(),
        }
    }

    /// One Radix gray step by number.
    pub fn gray(&self, step: u8) -> Hsla {
        match step {
            1 => self.gray_1,
            2 => self.gray_2,
            3 => self.gray_3,
            4 => self.gray_4,
            5 => self.gray_5,
            6 => self.gray_6,
            7 => self.gray_7,
            8 => self.gray_8,
            9 => self.gray_9,
            10 => self.gray_10,
            11 => self.gray_11,
            _ => self.gray_12,
        }
        .into()
    }

    /// The window shell's background.
    pub fn shell_bg(&self) -> Hsla {
        self.gray_1.into()
    }

    /// A header's background.
    pub fn header_bg(&self) -> Hsla {
        self.gray_2.into()
    }

    /// The hairline under a header.
    pub fn header_border(&self) -> Hsla {
        self.gray_5.into()
    }

    /// The body's inherited text colour.
    pub fn body_text(&self) -> Hsla {
        self.text_primary.into()
    }

    /// A `bg-gray-N` fill in the window body.
    pub fn body_fill(&self, step: u8) -> Hsla {
        self.gray(step)
    }

    /// A `hover:bg-gray-N` fill in the window body.
    pub fn body_hover_fill(&self, step: u8) -> Hsla {
        self.gray(step)
    }

    /// A `border-gray-N` in the window body.
    pub fn body_border(&self, step: u8) -> Hsla {
        self.gray(step)
    }

    /// `ring-offset-gray-1`, the gap between a selected control and its ring.
    pub fn ring_offset(&self) -> Hsla {
        self.gray_1.into()
    }

    /// A settings window's corner radius.
    pub fn settings_window_radius(&self) -> f32 {
        16.
    }

    /// A settings sidebar's background.
    pub fn settings_sidebar_bg(&self) -> Hsla {
        self.gray_2.into()
    }

    /// A settings content pane's background.
    pub fn settings_content_bg(&self) -> Hsla {
        self.gray_1.into()
    }

    /// A settings card's background.
    pub fn settings_card_bg(&self) -> Hsla {
        self.gray_2.into()
    }

    /// Selects, inline code chips and summary boxes on a settings page.
    pub fn settings_fill(&self) -> Hsla {
        self.gray_3.into()
    }

    /// A hovered settings row.
    pub fn settings_hover(&self) -> Hsla {
        self.gray_3.into()
    }

    /// The selected settings sidebar row.
    pub fn settings_selection(&self) -> Hsla {
        self.gray_5.into()
    }

    /// Settings row dividers.
    pub fn settings_border(&self) -> Hsla {
        self.gray_3.into()
    }

    /// The divider between a settings sidebar and its content.
    pub fn settings_divider(&self) -> Hsla {
        self.gray_3.into()
    }

    /// Settings page text.
    pub fn settings_text(&self) -> Hsla {
        self.gray_12.into()
    }

    /// Settings description lines.
    pub fn settings_muted(&self) -> Hsla {
        self.gray_10.into()
    }

    pub fn is_dark(&self) -> bool {
        self.appearance == Appearance::Dark
    }

    /// The selected-tile fill. Dark mode uses `blue-3/30` rather than solid
    /// `blue-3`, so it reads as a tint over `gray-1` instead of a slab.
    pub fn tile_selected_bg(&self) -> Hsla {
        if self.is_dark() {
            let mut color: Hsla = self.blue_3.into();
            color.a = 0.3;
            color
        } else {
            self.blue_3.into()
        }
    }

    /// Text and glyphs on the accent fill.
    pub fn on_accent(&self) -> Rgba {
        hex(0xffffff)
    }

    /// Canvas and text selection: the accent at 25 %.
    pub fn selection(&self) -> Rgba {
        rgba_alpha(self.editor.accent, 0x40 as f32 / 255.)
    }

    /// Tooltip background: the inverse of the window, in both appearances.
    pub fn tooltip_bg(&self) -> Rgba {
        self.gray_12
    }

    /// Tooltip text.
    pub fn tooltip_text(&self) -> Rgba {
        self.gray_1
    }

    /// A colour at an alpha, for Tailwind's `/N` overlay fills.
    pub fn with_alpha(color: Rgba, alpha: f32) -> Hsla {
        let mut color: Hsla = color.into();
        color.a = alpha;
        color
    }

    /// The default macOS accent, which gpui offers no query for.
    pub const SETTINGS_ACCENT: u32 = 0x007aff;
}

#[cfg(test)]
mod tests {
    use gpui::Rgba;

    use super::contrast::ratio;
    use super::*;

    /// `0xRRGGBBAA`, rounded from the stored floats.
    fn bytes(color: Rgba) -> u32 {
        let byte = |channel: f32| (channel * 255.).round() as u32;
        byte(color.r) << 24 | byte(color.g) << 16 | byte(color.b) << 8 | byte(color.a)
    }

    /// The editor's colour roles, in the order of the spec table.
    fn roles(theme: &Theme) -> [(&'static str, Rgba); 26] {
        let e = theme.editor;
        [
            ("window", e.window),
            ("card", e.card),
            ("card_2", e.card_2),
            ("stage", e.stage),
            ("line", e.line),
            ("line_strong", e.line_strong),
            ("text_1", e.text_1),
            ("text_2", e.text_2),
            ("text_3", e.text_3),
            ("ctl", e.ctl),
            ("ctl_hover", e.ctl_hover),
            ("ctl_active", e.ctl_active),
            ("accent", e.accent),
            ("accent_2", e.accent_2),
            ("on_accent", theme.on_accent()),
            ("playhead", e.playhead),
            ("thumb", e.thumb),
            ("success", theme.green_11),
            ("success_surface", theme.green_3),
            ("warning", theme.amber_11),
            ("warning_surface", theme.amber_3),
            ("danger", theme.red_11),
            ("danger_surface", theme.red_3),
            ("selection", theme.selection()),
            ("tooltip_bg", theme.tooltip_bg()),
            ("tooltip_text", theme.tooltip_text()),
        ]
    }

    fn assert_roles(appearance: Appearance, expected: [u32; 26]) {
        for ((name, color), want) in roles(&Theme::new(appearance)).into_iter().zip(expected) {
            assert_eq!(bytes(color), want, "{appearance:?} {name}");
        }
    }

    #[test]
    fn light_palette_matches_cap_spec() {
        assert_roles(
            Appearance::Light,
            [
                0xf1f1f3ff, 0xffffffff, 0xf6f6f7ff, 0xe9e9ecff, 0x00000013, 0x0000001f, 0x1c1c1eff,
                0x6e6e75ff, 0xa0a0a8ff, 0x0000000b, 0x00000013, 0x0000001c, 0x007affff, 0x2b8cffff,
                0xffffffff, 0xff3b30ff, 0xffffffff, 0x218358ff, 0xe6f6ebff, 0xab6400ff, 0xfff7c2ff,
                0xce2c31ff, 0xfeebecff, 0x007aff40, 0x202020ff, 0xfcfcfcff,
            ],
        );
    }

    #[test]
    fn dark_palette_matches_cap_spec() {
        assert_roles(
            Appearance::Dark,
            [
                0x131315ff, 0x1b1b1eff, 0x202024ff, 0x121214ff, 0xffffff14, 0xffffff21, 0xf4f4f5ff,
                0x9a9aa3ff, 0x62626aff, 0xffffff0f, 0xffffff17, 0xffffff21, 0x0a84ffff, 0x3d9bffff,
                0xffffffff, 0xff453aff, 0xe8e8eaff, 0x3dd68cff, 0x132d21ff, 0xffca16ff, 0x302008ff,
                0xff9592ff, 0x3b1219ff, 0x0a84ff40, 0xeeeeeeff, 0x111111ff,
            ],
        );
    }

    #[test]
    fn theme_preference_resolves_like_tauri() {
        assert_eq!(
            resolve_appearance(AppTheme::System, Appearance::Dark),
            Appearance::Dark
        );
        assert_eq!(
            resolve_appearance(AppTheme::System, Appearance::Light),
            Appearance::Light
        );
        assert_eq!(
            resolve_appearance(AppTheme::Light, Appearance::Dark),
            Appearance::Light
        );
        assert_eq!(
            resolve_appearance(AppTheme::Dark, Appearance::Light),
            Appearance::Dark
        );
    }

    #[test]
    fn appearance_from_window_maps_all_variants() {
        assert_eq!(
            Appearance::from_window(WindowAppearance::Light),
            Appearance::Light
        );
        assert_eq!(
            Appearance::from_window(WindowAppearance::VibrantLight),
            Appearance::Light
        );
        assert_eq!(
            Appearance::from_window(WindowAppearance::Dark),
            Appearance::Dark
        );
        assert_eq!(
            Appearance::from_window(WindowAppearance::VibrantDark),
            Appearance::Dark
        );
    }

    #[test]
    fn body_surfaces_are_the_plain_radix_steps() {
        for theme in [Theme::light(), Theme::dark()] {
            assert_eq!(theme.shell_bg(), Hsla::from(theme.gray_1));
            assert_eq!(theme.header_bg(), Hsla::from(theme.gray_2));
            assert_eq!(theme.header_border(), Hsla::from(theme.gray_5));
            assert_eq!(theme.body_text(), Hsla::from(theme.text_primary));
            assert_eq!(theme.ring_offset(), Hsla::from(theme.gray_1));
            for step in 1..=12 {
                assert_eq!(theme.body_fill(step), theme.gray(step));
                assert_eq!(theme.body_hover_fill(step), theme.gray(step));
                assert_eq!(theme.body_border(step), theme.gray(step));
            }
        }
    }

    #[test]
    fn status_roles_match_radix() {
        let (light, dark) = (Theme::light(), Theme::dark());
        let roles = |t: &Theme| {
            [
                t.green_11, t.green_3, t.amber_11, t.amber_3, t.red_11, t.red_3,
            ]
            .map(bytes)
        };
        assert_eq!(
            roles(&light),
            [
                0x218358ff, 0xe6f6ebff, 0xab6400ff, 0xfff7c2ff, 0xce2c31ff, 0xfeebecff
            ]
        );
        assert_eq!(
            roles(&dark),
            [
                0x3dd68cff, 0x132d21ff, 0xffca16ff, 0x302008ff, 0xff9592ff, 0x3b1219ff
            ]
        );
        assert_eq!(bytes(light.selection()), 0x007aff40);
        assert_eq!(bytes(dark.selection()), 0x0a84ff40);
        assert_eq!(
            [light.tooltip_bg(), light.tooltip_text()].map(bytes),
            [0x202020ff, 0xfcfcfcff]
        );
        assert_eq!(
            [dark.tooltip_bg(), dark.tooltip_text()].map(bytes),
            [0xeeeeeeff, 0x111111ff]
        );
    }

    #[test]
    fn translucent_roles_keep_alpha() {
        for theme in [Theme::light(), Theme::dark()] {
            let p = theme.editor;
            for color in [p.line, p.line_strong, p.ctl, p.ctl_hover, p.ctl_active] {
                assert!(
                    color.a > 0. && color.a < 1.,
                    "{:?} {color:?}",
                    theme.appearance
                );
            }
        }
    }

    #[test]
    fn body_text_contrast() {
        for theme in [Theme::light(), Theme::dark()] {
            let p = theme.editor;
            for background in [p.window, p.card, p.card_2] {
                let value = ratio(p.text_1, background);
                assert!(value >= 4.5, "{:?} text_1 {value}", theme.appearance);
            }
            for background in [p.card, p.card_2] {
                let value = ratio(p.text_2, background);
                assert!(value >= 4.5, "{:?} text_2 {value}", theme.appearance);
            }
        }
    }

    #[test]
    fn accepted_contrast_shortfalls_are_recorded() {
        let (light, dark) = (Theme::light(), Theme::dark());
        let cases = [
            (
                "white on accent, light",
                ratio(light.on_accent(), light.editor.accent),
                4.02,
            ),
            (
                "white on accent, dark",
                ratio(dark.on_accent(), dark.editor.accent),
                3.65,
            ),
            (
                "text_2 on window, light",
                ratio(light.editor.text_2, light.editor.window),
                4.49,
            ),
            (
                "text_3 on card, light",
                ratio(light.editor.text_3, light.editor.card),
                2.60,
            ),
        ];
        for (name, measured, recorded) in cases {
            assert!(
                (measured - recorded).abs() <= 0.01,
                "{name}: measured {measured}, recorded {recorded}"
            );
        }
    }

    #[test]
    fn semantic_status_surfaces_are_distinct_from_the_neutral_surface() {
        for theme in [Theme::light(), Theme::dark()] {
            for surface in [theme.green_3, theme.amber_3, theme.red_3] {
                assert_ne!(surface, theme.editor.card);
            }
        }
    }
}
