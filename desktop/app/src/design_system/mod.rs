//! Studio's application-owned visual language.
//!
//! Video style presets belong to project content; this module owns the Studio
//! chrome. Colours, metrics, typography, shadows and motion live here, and the
//! colour set follows the OS light or dark appearance at runtime, so screens do
//! not create their own competing collections of values.

use std::sync::atomic::{AtomicU8, Ordering};

use gpui::{Div, Stateful, WindowAppearance};

#[cfg(test)]
mod contrast;
mod elevation;
mod metrics;
mod motion;
mod palette;
mod typography;

pub use elevation::{Shadow, shadow, shadow_for};
pub use metrics::{METRICS, Metrics};
pub use motion::{FAST_MS, NORMAL_MS, reduced_motion};
pub use palette::{
    DARK, LIGHT, Palette, SegmentColors, TrackKind, hex, hexa, segment_colors, segment_colors_for,
    track_color,
};
pub use typography::{TextStyle, UI_FONT, text_style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

static APPEARANCE: AtomicU8 = AtomicU8::new(DARK_VALUE);
const LIGHT_VALUE: u8 = 0;
const DARK_VALUE: u8 = 1;

/// The appearance every token lookup currently resolves against.
pub fn appearance() -> Appearance {
    match APPEARANCE.load(Ordering::Relaxed) {
        LIGHT_VALUE => Appearance::Light,
        _ => Appearance::Dark,
    }
}

/// Process-wide appearance switch; only `main.rs` calls this, from the OS
/// appearance.
pub fn set_appearance(appearance: Appearance) {
    let value = match appearance {
        Appearance::Light => LIGHT_VALUE,
        Appearance::Dark => DARK_VALUE,
    };
    APPEARANCE.store(value, Ordering::Relaxed);
}

pub fn appearance_from_window(appearance: WindowAppearance) -> Appearance {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignTokens {
    pub palette: Palette,
    pub metrics: Metrics,
}

static LIGHT_TOKENS: DesignTokens = DesignTokens {
    palette: LIGHT,
    metrics: METRICS,
};

static DARK_TOKENS: DesignTokens = DesignTokens {
    palette: DARK,
    metrics: METRICS,
};

/// Tokens of the current appearance.
pub fn tokens() -> &'static DesignTokens {
    tokens_for(appearance())
}

pub fn tokens_for(appearance: Appearance) -> &'static DesignTokens {
    match appearance {
        Appearance::Light => &LIGHT_TOKENS,
        Appearance::Dark => &DARK_TOKENS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Secondary,
    Primary,
    Selected,
    Destructive,
}

/// Shared baseline for native command buttons: applies the `ui::Button` look
/// to a caller-built element. Callers remain responsible for labels, actions,
/// keyboard activation and accessibility names; new code should use
/// `ui::Button` directly.
pub fn button(element: Stateful<Div>, style: ButtonStyle, enabled: bool) -> Stateful<Div> {
    use crate::ui::button::{ButtonVariant, style_button};
    let (variant, selected) = match style {
        ButtonStyle::Secondary => (ButtonVariant::Editor, false),
        ButtonStyle::Primary => (ButtonVariant::Primary, false),
        ButtonStyle::Selected => (ButtonVariant::Editor, true),
        ButtonStyle::Destructive => (ButtonVariant::Destructive, false),
    };
    style_button(element, variant, selected, !enabled)
}

#[cfg(test)]
mod tests {
    use gpui::{Rgba, WindowAppearance};

    use super::contrast::ratio;
    use super::*;

    /// `0xRRGGBBAA`, rounded from the stored floats.
    fn bytes(color: Rgba) -> u32 {
        let byte = |channel: f32| (channel * 255.).round() as u32;
        byte(color.r) << 24 | byte(color.g) << 16 | byte(color.b) << 8 | byte(color.a)
    }

    fn roles(p: &Palette) -> [(&'static str, Rgba); 26] {
        [
            ("window", p.window),
            ("card", p.card),
            ("card_2", p.card_2),
            ("stage", p.stage),
            ("line", p.line),
            ("line_strong", p.line_strong),
            ("text_1", p.text_1),
            ("text_2", p.text_2),
            ("text_3", p.text_3),
            ("ctl", p.ctl),
            ("ctl_hover", p.ctl_hover),
            ("ctl_active", p.ctl_active),
            ("accent", p.accent),
            ("accent_2", p.accent_2),
            ("on_accent", p.on_accent),
            ("playhead", p.playhead),
            ("thumb", p.thumb),
            ("success", p.success),
            ("success_surface", p.success_surface),
            ("warning", p.warning),
            ("warning_surface", p.warning_surface),
            ("danger", p.danger),
            ("danger_surface", p.danger_surface),
            ("selection", p.selection),
            ("tooltip_bg", p.tooltip_bg),
            ("tooltip_text", p.tooltip_text),
        ]
    }

    fn assert_roles(actual: &Palette, expected: [u32; 26]) {
        for ((name, color), want) in roles(actual).into_iter().zip(expected) {
            assert_eq!(bytes(color), want, "{name}");
        }
    }

    #[test]
    fn light_palette_matches_cap_spec() {
        assert_roles(
            &tokens_for(Appearance::Light).palette,
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
            &tokens_for(Appearance::Dark).palette,
            [
                0x131315ff, 0x1b1b1eff, 0x202024ff, 0x121214ff, 0xffffff14, 0xffffff21, 0xf4f4f5ff,
                0x9a9aa3ff, 0x62626aff, 0xffffff0f, 0xffffff17, 0xffffff21, 0x0a84ffff, 0x3d9bffff,
                0xffffffff, 0xff453aff, 0xe8e8eaff, 0x3dd68cff, 0x132d21ff, 0xffca16ff, 0x302008ff,
                0xff9592ff, 0x3b1219ff, 0x0a84ff40, 0xeeeeeeff, 0x111111ff,
            ],
        );
    }

    #[test]
    fn translucent_roles_keep_alpha() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let p = tokens_for(appearance).palette;
            for color in [p.line, p.line_strong, p.ctl, p.ctl_hover, p.ctl_active] {
                assert!(color.a > 0. && color.a < 1., "{appearance:?} {color:?}");
            }
        }
    }

    #[test]
    fn body_text_contrast() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let p = tokens_for(appearance).palette;
            for background in [p.window, p.card, p.card_2] {
                let value = ratio(p.text_1, background);
                assert!(value >= 4.5, "{appearance:?} text_1 {value}");
            }
            for background in [p.card, p.card_2] {
                let value = ratio(p.text_2, background);
                assert!(value >= 4.5, "{appearance:?} text_2 {value}");
            }
        }
    }

    #[test]
    fn accepted_contrast_shortfalls_are_recorded() {
        let light = tokens_for(Appearance::Light).palette;
        let dark = tokens_for(Appearance::Dark).palette;
        let cases = [
            (
                "white on accent, light",
                ratio(light.on_accent, light.accent),
                4.02,
            ),
            (
                "white on accent, dark",
                ratio(dark.on_accent, dark.accent),
                3.65,
            ),
            (
                "text_2 on window, light",
                ratio(light.text_2, light.window),
                4.49,
            ),
            (
                "text_3 on card, light",
                ratio(light.text_3, light.card),
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

    fn assert_close(actual: Rgba, expected: u32, name: &str) {
        let (r, g, b) = (
            (expected >> 16) & 0xff,
            (expected >> 8) & 0xff,
            expected & 0xff,
        );
        let got = bytes(actual) >> 8;
        let delta = |a: u32, b: u32| a.abs_diff(b);
        assert!(
            delta((got >> 16) & 0xff, r) <= 1
                && delta((got >> 8) & 0xff, g) <= 1
                && delta(got & 0xff, b) <= 1,
            "{name}: got {got:06x}, want {expected:06x}"
        );
    }

    #[test]
    fn segment_colors_follow_mix_rule() {
        // Clip blue 3b82f6 mixed into card; expected bytes computed from the rule
        // (fill 13 %/20 %, +10 points selected, border 34 %/42 %, label 58 %/55 %).
        let light = segment_colors_for(Appearance::Light, TrackKind::Clip, false);
        assert_close(light.fill, 0xe6effe, "light fill");
        assert_close(light.border, 0xbcd4fc, "light border");
        assert_close(light.label, 0x193767, "light label");
        let light_selected = segment_colors_for(Appearance::Light, TrackKind::Clip, true);
        assert_close(light_selected.fill, 0xd2e2fd, "light selected fill");
        assert_eq!(light_selected.border, light.border);

        let dark = segment_colors_for(Appearance::Dark, TrackKind::Audio, false);
        assert_close(dark.fill, 0x1c3d2b, "dark fill");
        assert_close(dark.border, 0x1e6239, "dark border");
        assert_close(dark.label, 0x9ce5b7, "dark label");
        let dark_selected = segment_colors_for(Appearance::Dark, TrackKind::Audio, true);
        assert_close(dark_selected.fill, 0x1d4e31, "dark selected fill");
    }

    #[test]
    fn metrics_match_cap_spec() {
        let m = tokens_for(Appearance::Dark).metrics;
        assert_eq!(
            [m.space_1, m.space_2, m.space_3, m.space_4, m.space_6],
            [4., 8., 12., 16., 24.]
        );
        assert_eq!(
            [
                m.radius_control,
                m.radius_item,
                m.radius_segment,
                m.radius_card,
                m.radius_popover,
                m.radius_tab,
                m.radius_frame
            ],
            [7., 6., 8., 12., 12., 9., 6.]
        );
        assert_eq!(
            [
                m.control_h,
                m.select_h,
                m.input_h,
                m.field_row_h,
                m.icon_sm,
                m.icon_md
            ],
            [28., 30., 32., 34., 14., 16.]
        );
        assert_eq!(
            [
                m.header_h,
                m.tab_bar_h,
                m.toolbar_h,
                m.transport_h,
                m.gutter,
                m.left_column_w,
                m.agent_panel_w,
                m.timeline_default_h,
                m.timeline_min_h,
                m.player_min_h
            ],
            [52., 46., 44., 48., 8., 248., 400., 260., 240., 320.]
        );
        assert_eq!(
            [m.ruler_h, m.track_h, m.track_gap, m.track_gutter_w],
            [26., 44., 6., 104.]
        );
        assert_eq!(tokens_for(Appearance::Light).metrics, m);
    }

    #[test]
    fn appearance_from_window_maps_all_variants() {
        assert_eq!(
            appearance_from_window(WindowAppearance::Light),
            Appearance::Light
        );
        assert_eq!(
            appearance_from_window(WindowAppearance::VibrantLight),
            Appearance::Light
        );
        assert_eq!(
            appearance_from_window(WindowAppearance::Dark),
            Appearance::Dark
        );
        assert_eq!(
            appearance_from_window(WindowAppearance::VibrantDark),
            Appearance::Dark
        );
    }

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

    #[test]
    fn shadows_follow_the_spec_table() {
        use gpui::Pixels;
        let px = |value: Pixels| f32::from(value);
        let card = shadow_for(Appearance::Dark, Shadow::Card);
        assert_eq!(card.len(), 1);
        assert_eq!(card[0].color.a, 0.30);
        assert_eq!(px(card[0].blur_radius), 2.);
        let pop_light = shadow_for(Appearance::Light, Shadow::Pop);
        let pop_dark = shadow_for(Appearance::Dark, Shadow::Pop);
        assert_eq!(
            (px(pop_light[0].offset.y), px(pop_light[0].blur_radius)),
            (12., 32.)
        );
        assert_eq!(
            (px(pop_dark[0].offset.y), px(pop_dark[0].blur_radius)),
            (16., 40.)
        );
        assert_eq!(px(pop_dark[0].spread_radius), -12.);
        assert_eq!(shadow_for(Appearance::Light, Shadow::RaisedPill).len(), 2);
    }

    /// Every role resolves to the same colour through `studio_theme`, so moving a call
    /// site from `tokens()` to the theme changes nothing on screen.
    #[test]
    fn roles_match_the_studio_theme() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let old = tokens_for(appearance).palette;
            let theme = studio_theme::Theme::new(match appearance {
                Appearance::Light => studio_theme::Appearance::Light,
                Appearance::Dark => studio_theme::Appearance::Dark,
            });
            let e = theme.editor;
            let new = Palette {
                window: e.window,
                card: e.card,
                card_2: e.card_2,
                stage: e.stage,
                line: e.line,
                line_strong: e.line_strong,
                text_1: e.text_1,
                text_2: e.text_2,
                text_3: e.text_3,
                ctl: e.ctl,
                ctl_hover: e.ctl_hover,
                ctl_active: e.ctl_active,
                accent: e.accent,
                accent_2: e.accent_2,
                on_accent: theme.on_accent(),
                playhead: e.playhead,
                thumb: e.thumb,
                success: theme.green_11,
                success_surface: theme.green_3,
                warning: theme.amber_11,
                warning_surface: theme.amber_3,
                danger: theme.red_11,
                danger_surface: theme.red_3,
                selection: theme.selection(),
                tooltip_bg: theme.tooltip_bg(),
                tooltip_text: theme.tooltip_text(),
            };
            for ((name, old), (_, new)) in roles(&old).into_iter().zip(roles(&new)) {
                assert_eq!(bytes(old), bytes(new), "{appearance:?} {name}");
            }
            for kind in [TrackKind::Clip, TrackKind::Overlay, TrackKind::Audio] {
                for selected in [false, true] {
                    let old = segment_colors_for(appearance, kind, selected);
                    let new = studio_theme::segment_colors_for(
                        theme.appearance,
                        match kind {
                            TrackKind::Clip => studio_theme::TrackKind::Clip,
                            TrackKind::Overlay => studio_theme::TrackKind::Overlay,
                            TrackKind::Audio => studio_theme::TrackKind::Audio,
                        },
                        selected,
                    );
                    for (old, new) in [
                        (old.fill, new.fill),
                        (old.border, new.border),
                        (old.label, new.label),
                    ] {
                        assert_eq!(bytes(old), bytes(new), "{appearance:?} {kind:?}");
                    }
                }
            }
            let shadows = [
                (Shadow::Card, studio_theme::Shadow::Card),
                (Shadow::Pop, studio_theme::Shadow::Pop),
                (Shadow::Preview, studio_theme::Shadow::Preview),
                (Shadow::RaisedPill, studio_theme::Shadow::RaisedPill),
                (Shadow::Thumb, studio_theme::Shadow::Thumb),
            ];
            for (old, new) in shadows {
                assert_eq!(
                    shadow_for(appearance, old).to_vec(),
                    studio_theme::shadow_for(theme.appearance, new).to_vec()
                );
            }
        }
        assert_eq!(
            format!("{METRICS:?}"),
            format!("{:?}", studio_theme::METRICS)
        );
        assert_eq!(UI_FONT, studio_theme::UI_FONT);
    }

    #[test]
    fn semantic_status_surfaces_are_distinct_from_the_neutral_surface() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let colors = tokens_for(appearance).palette;
            assert_ne!(colors.success_surface, colors.card);
            assert_ne!(colors.warning_surface, colors.card);
            assert_ne!(colors.danger_surface, colors.card);
        }
    }
}
