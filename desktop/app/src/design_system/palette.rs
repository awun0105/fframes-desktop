//! Colour roles for the light and dark appearance. Every role is an `Rgba` so
//! translucent borders and control fills keep their alpha.

use gpui::Rgba;

use super::Appearance;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
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
    pub on_accent: Rgba,
    pub playhead: Rgba,
    pub thumb: Rgba,
    pub success: Rgba,
    pub success_surface: Rgba,
    pub warning: Rgba,
    pub warning_surface: Rgba,
    pub danger: Rgba,
    pub danger_surface: Rgba,
    /// Accent at 25 %, for canvas selection and text selection.
    pub selection: Rgba,
    /// Radix grey 12.
    pub tooltip_bg: Rgba,
    /// Radix grey 1.
    pub tooltip_text: Rgba,
}

pub const LIGHT: Palette = Palette {
    window: hex(0xf1f1f3),
    card: hex(0xffffff),
    card_2: hex(0xf6f6f7),
    stage: hex(0xe9e9ec),
    line: hexa(0x00000013),
    line_strong: hexa(0x0000001f),
    text_1: hex(0x1c1c1e),
    text_2: hex(0x6e6e75),
    text_3: hex(0xa0a0a8),
    ctl: hexa(0x0000000b),
    ctl_hover: hexa(0x00000013),
    ctl_active: hexa(0x0000001c),
    accent: hex(0x007aff),
    accent_2: hex(0x2b8cff),
    on_accent: hex(0xffffff),
    playhead: hex(0xff3b30),
    thumb: hex(0xffffff),
    // Radix green 11 / 3, amber 11 / 3, red 11 / 3.
    success: hex(0x218358),
    success_surface: hex(0xe6f6eb),
    warning: hex(0xab6400),
    warning_surface: hex(0xfff7c2),
    danger: hex(0xce2c31),
    danger_surface: hex(0xfeebec),
    selection: hexa(0x007aff40),
    tooltip_bg: hex(0x202020),
    tooltip_text: hex(0xfcfcfc),
};

pub const DARK: Palette = Palette {
    window: hex(0x131315),
    card: hex(0x1b1b1e),
    card_2: hex(0x202024),
    stage: hex(0x121214),
    line: hexa(0xffffff14),
    line_strong: hexa(0xffffff21),
    text_1: hex(0xf4f4f5),
    text_2: hex(0x9a9aa3),
    text_3: hex(0x62626a),
    ctl: hexa(0xffffff0f),
    ctl_hover: hexa(0xffffff17),
    ctl_active: hexa(0xffffff21),
    accent: hex(0x0a84ff),
    accent_2: hex(0x3d9bff),
    on_accent: hex(0xffffff),
    playhead: hex(0xff453a),
    thumb: hex(0xe8e8ea),
    success: hex(0x3dd68c),
    success_surface: hex(0x132d21),
    warning: hex(0xffca16),
    warning_surface: hex(0x302008),
    danger: hex(0xff9592),
    danger_surface: hex(0x3b1219),
    selection: hexa(0x0a84ff40),
    tooltip_bg: hex(0xeeeeee),
    tooltip_text: hex(0x111111),
};

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

fn mix(from: Rgba, to: Rgba, amount: f32) -> Rgba {
    Rgba {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: 1.,
    }
}

/// Segment colours for an appearance. The track colour is mixed into `card`:
/// fill 13 % light / 20 % dark (a selected segment adds 10 points), border
/// 34 % / 42 %; the label mixes the track colour toward black (light) or
/// white (dark) by 58 % / 55 %.
pub fn segment_colors_for(
    appearance: Appearance,
    kind: TrackKind,
    selected: bool,
) -> SegmentColors {
    let (palette, fill_mix, border_mix, label_target, label_mix) = match appearance {
        Appearance::Light => (&LIGHT, 0.13, 0.34, hex(0x000000), 0.58),
        Appearance::Dark => (&DARK, 0.20, 0.42, hex(0xffffff), 0.55),
    };
    let track = track_color(kind);
    SegmentColors {
        fill: mix(
            palette.card,
            track,
            fill_mix + if selected { 0.10 } else { 0. },
        ),
        border: mix(palette.card, track, border_mix),
        label: mix(track, label_target, label_mix),
    }
}

/// Segment colours for the current appearance.
pub fn segment_colors(kind: TrackKind, selected: bool) -> SegmentColors {
    segment_colors_for(super::appearance(), kind, selected)
}
