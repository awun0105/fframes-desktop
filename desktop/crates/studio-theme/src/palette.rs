//! The light and dark palettes, and the timeline's track colours.

use gpui::{Rgba, rgb, rgba};

use crate::{Appearance, EditorPalette, Theme, mix};

/// Opaque colour from `0xRRGGBB`.
pub const fn hex(value: u32) -> Rgba {
    Rgba {
        r: ((value >> 16) & 0xff) as f32 / 255.,
        g: ((value >> 8) & 0xff) as f32 / 255.,
        b: (value & 0xff) as f32 / 255.,
        a: 1.,
    }
}

/// Colour with alpha from `0xRRGGBBAA`.
pub const fn hexa(value: u32) -> Rgba {
    Rgba {
        r: ((value >> 24) & 0xff) as f32 / 255.,
        g: ((value >> 16) & 0xff) as f32 / 255.,
        b: ((value >> 8) & 0xff) as f32 / 255.,
        a: (value & 0xff) as f32 / 255.,
    }
}

impl EditorPalette {
    pub fn light() -> Self {
        Self {
            window: rgb(0xf1f1f3),
            card: rgb(0xffffff),
            card_2: rgb(0xf6f6f7),
            stage: rgb(0xe9e9ec),
            line: rgba(0x00000013),
            line_strong: rgba(0x0000001f),
            text_1: rgb(0x1c1c1e),
            text_2: rgb(0x6e6e75),
            text_3: rgb(0xa0a0a8),
            ctl: rgba(0x0000000b),
            ctl_hover: rgba(0x00000013),
            ctl_active: rgba(0x0000001c),
            accent: rgb(0x007aff),
            accent_2: rgb(0x2b8cff),
            playhead: rgb(0xff3b30),
            thumb: rgb(0xffffff),
            seg_tint: 0.13,
            seg_line: 0.34,
            seg_label_base: rgb(0x000000),
            seg_label_mix: 0.58,
            card_shadow_alpha: 0.04,
            pop_shadow_alpha: 0.25,
            pop_shadow_offset: 12.,
            pop_shadow_blur: 32.,
        }
    }

    pub fn dark() -> Self {
        Self {
            window: rgb(0x131315),
            card: rgb(0x1b1b1e),
            card_2: rgb(0x202024),
            stage: rgb(0x121214),
            line: rgba(0xffffff14),
            line_strong: rgba(0xffffff21),
            text_1: rgb(0xf4f4f5),
            text_2: rgb(0x9a9aa3),
            text_3: rgb(0x62626a),
            ctl: rgba(0xffffff0f),
            ctl_hover: rgba(0xffffff17),
            ctl_active: rgba(0xffffff21),
            accent: rgb(0x0a84ff),
            accent_2: rgb(0x3d9bff),
            playhead: rgb(0xff453a),
            thumb: rgb(0xe8e8ea),
            seg_tint: 0.20,
            seg_line: 0.42,
            seg_label_base: rgb(0xffffff),
            seg_label_mix: 0.55,
            card_shadow_alpha: 0.30,
            pop_shadow_alpha: 0.60,
            pop_shadow_offset: 16.,
            pop_shadow_blur: 40.,
        }
    }
}

impl Theme {
    pub fn light() -> Self {
        Self {
            appearance: Appearance::Light,
            editor: EditorPalette::light(),

            gray_1: rgb(0xfcfcfc),
            gray_2: rgb(0xf9f9f9),
            gray_3: rgb(0xf0f0f0),
            gray_4: rgb(0xe8e8e8),
            gray_5: rgb(0xe0e0e0),
            gray_6: rgb(0xd9d9d9),
            gray_7: rgb(0xcecece),
            gray_8: rgb(0xbbbbbb),
            gray_9: rgb(0x8d8d8d),
            gray_10: rgb(0x838383),
            gray_11: rgb(0x646464),
            gray_12: rgb(0x202020),

            blue_3: rgb(0xe6f4fe),
            blue_4: rgb(0xd5efff),
            blue_5: rgb(0xc2e5ff),
            blue_6: rgb(0xacd8fc),
            blue_8: rgb(0x5eb1ef),
            blue_9: rgb(0x0090ff),
            blue_10: rgb(0x0588f0),
            blue_11: rgb(0x0d74ce),
            blue_12: rgb(0x113264),

            red_2: rgb(0xfff7f7),
            red_3: rgb(0xfeebec),
            red_4: rgb(0xffdbdc),
            red_9: rgb(0xe5484d),
            red_10: rgb(0xdc3e42),
            red_11: rgb(0xce2c31),

            amber_3: rgb(0xfff7c2),
            amber_6: rgb(0xf3d673),
            amber_11: rgb(0xab6400),

            green_3: rgb(0xe6f6eb),
            green_11: rgb(0x218358),

            blue_100: rgb(0xadc9ff),
            blue_500: rgb(0x3666c5),
            red_300: rgb(0xff4766),

            // `theme.css:38-44`.
            gray_200_legacy: rgb(0xe4e6ed),
            gray_300_legacy: rgb(0xc7ccda),
            // `--gray-400: #868e9f` (`theme.css:42`).
            gray_400_legacy: rgb(0x868e9f),
            gray_500_legacy: rgb(0x161b26),

            text_primary: rgba(0x12161ff2),
        }
    }

    pub fn dark() -> Self {
        Self {
            appearance: Appearance::Dark,
            editor: EditorPalette::dark(),

            // gray 1-6 and 11 are the theme.css overrides, not stock Radix.
            gray_1: rgb(0x111111),
            gray_2: rgb(0x1c1c1c),
            gray_3: rgb(0x282828),
            gray_4: rgb(0x323232),
            gray_5: rgb(0x3a3a3a),
            gray_6: rgb(0x444444),
            gray_7: rgb(0x484848),
            gray_8: rgb(0x606060),
            gray_9: rgb(0x6e6e6e),
            gray_10: rgb(0x7b7b7b),
            gray_11: rgb(0xa1a1a1),
            gray_12: rgb(0xeeeeee),

            blue_3: rgb(0x0d2847),
            blue_4: rgb(0x003362),
            blue_5: rgb(0x004074),
            blue_6: rgb(0x104d87),
            blue_8: rgb(0x2870bd),
            blue_9: rgb(0x0090ff),
            blue_10: rgb(0x3b9eff),
            blue_11: rgb(0x70b8ff),
            blue_12: rgb(0xc2e6ff),

            red_2: rgb(0x201314),
            red_3: rgb(0x3b1219),
            red_4: rgb(0x500f1c),
            red_9: rgb(0xe5484d),
            red_10: rgb(0xec5d5e),
            red_11: rgb(0xff9592),

            amber_3: rgb(0x302008),
            amber_6: rgb(0x5c3d05),
            amber_11: rgb(0xffca16),

            green_3: rgb(0x132d21),
            green_11: rgb(0x3dd68c),

            blue_100: rgb(0x243352),
            blue_500: rgb(0x0a84ff),
            red_300: rgb(0xff4766),

            // `theme.css:101-107`.
            gray_200_legacy: rgb(0x282828),
            gray_300_legacy: rgb(0x323232),
            // `--gray-400: #a1a1a1` (`theme.css:105`).
            gray_400_legacy: rgb(0xa1a1a1),
            gray_500_legacy: rgb(0xffffff),

            text_primary: rgba(0xfffffff2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Clip,
    Overlay,
    Audio,
}

/// Base colour of a track kind (Tailwind 500: blue, violet, green).
pub const fn track_color(kind: TrackKind) -> Rgba {
    match kind {
        TrackKind::Clip => hex(0x3b82f6),
        TrackKind::Overlay => hex(0x8b5cf6),
        TrackKind::Audio => hex(0x22c55e),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentColors {
    pub fill: Rgba,
    pub border: Rgba,
    pub label: Rgba,
}

/// Segment colours for an appearance. The track colour is mixed into `card`:
/// fill 13 % light / 20 % dark (a selected segment adds 10 points), border
/// 34 % / 42 %; the label mixes the track colour toward black (light) or
/// white (dark) by 58 % / 55 %. Every result is opaque.
pub fn segment_colors_for(
    appearance: Appearance,
    kind: TrackKind,
    selected: bool,
) -> SegmentColors {
    let (palette, fill_mix, border_mix, label_target, label_mix) = match appearance {
        Appearance::Light => (EditorPalette::light(), 0.13, 0.34, hex(0x000000), 0.58),
        Appearance::Dark => (EditorPalette::dark(), 0.20, 0.42, hex(0xffffff), 0.55),
    };
    let track = track_color(kind);
    let opaque = |color: Rgba| Rgba { a: 1., ..color };
    SegmentColors {
        fill: opaque(mix(
            palette.card,
            track,
            fill_mix + if selected { 0.10 } else { 0. },
        )),
        border: opaque(mix(palette.card, track, border_mix)),
        label: opaque(mix(track, label_target, label_mix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `0xRRGGBBAA`, rounded from the stored floats.
    fn bytes(color: Rgba) -> u32 {
        let byte = |channel: f32| (channel * 255.).round() as u32;
        byte(color.r) << 24 | byte(color.g) << 16 | byte(color.b) << 8 | byte(color.a)
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
        for color in [light.fill, light.border, light.label, dark.fill] {
            assert_eq!(color.a, 1.);
        }
    }
}
