//! The theme: typed design tokens every component reads its defaults from.
//!
//! Override a group with struct update syntax and keep the rest:
//!
//! ```
//! use fframes_kit::prelude::*;
//!
//! let theme = Theme {
//!     colors: Colors { accent: Color::hex("#ff5500"), ..Colors::default() },
//!     ..Theme::default()
//! };
//! assert_eq!(theme.colors.background, Theme::default().colors.background);
//! ```
//!
//! The defaults are a neutral dark look: near-black background, white text, one blue
//! accent, DM Sans (ship the font in the video's media folder).

use fframes::{Color, FontQuery};

use crate::anim::RateFunc;

/// All tokens. `Theme::default()` is the neutral dark theme described in the module docs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Theme {
    pub colors: Colors,
    pub typography: Typography,
    pub spacing: Spacing,
    pub radius: Radius,
    pub stroke: Stroke,
    pub shadow: Shadows,
    pub motion: Motion,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    /// Canvas behind everything.
    pub background: Color,
    /// Cards and panels on the background.
    pub surface: Color,
    /// Body text.
    pub text: Color,
    /// Titles and headings.
    pub title: Color,
    /// Secondary text, rules and inactive parts.
    pub muted: Color,
    /// The one brand colour: highlights, active parts, key data.
    pub accent: Color,
    /// A second emphasis colour for marks and callouts.
    pub highlight: Color,
    /// Caption and subtitle text.
    pub caption: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: Color::hex("#0b0d12"),
            surface: Color::hex("#171a22"),
            text: Color::hex("#e8eaf0"),
            title: Color::hex("#ffffff"),
            muted: Color::hex("#8a90a0"),
            accent: Color::hex("#4f8cff"),
            highlight: Color::hex("#ffc94d"),
            caption: Color::hex("#ffffff"),
        }
    }
}

/// A font and size, matching the `font-family`, `font-size` and `font-weight` attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub family: &'static str,
    pub size: usize,
    pub weight: u16,
    /// Line height as a multiple of `size`.
    pub line_height: f32,
}

impl TextStyle {
    /// The query for measuring text in this style.
    pub fn font(&self) -> FontQuery<'static> {
        FontQuery {
            family: self.family,
            size: self.size,
            weight: self.weight,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    pub title: TextStyle,
    pub heading: TextStyle,
    pub body: TextStyle,
    pub caption: TextStyle,
}

impl Default for Typography {
    fn default() -> Self {
        let style = |size, weight, line_height| TextStyle {
            family: "DM Sans",
            size,
            weight,
            line_height,
        };
        Self {
            title: style(96, 700, 1.1),
            heading: style(56, 600, 1.2),
            body: style(36, 400, 1.4),
            caption: style(28, 500, 1.3),
        }
    }
}

/// Distances in canvas units (pixels of a 1920x1080 viewBox).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    /// Between columns and cards.
    pub gutter: f32,
    /// From the canvas edge to content (title safe area).
    pub margin: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 32.0,
            xl: 64.0,
            gutter: 24.0,
            margin: 96.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub card: f32,
}

impl Default for Radius {
    fn default() -> Self {
        Self {
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            card: 24.0,
        }
    }
}

/// Stroke widths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub thin: f32,
    pub regular: f32,
    pub thick: f32,
    pub divider: f32,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            thin: 2.0,
            regular: 4.0,
            thick: 8.0,
            divider: 1.0,
        }
    }
}

/// A drop shadow: offset, blur radius (standard deviation) and colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub dx: f32,
    pub dy: f32,
    pub blur: f32,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadows {
    pub card: Shadow,
    pub glow: Shadow,
}

impl Default for Shadows {
    fn default() -> Self {
        Self {
            card: Shadow {
                dx: 0.0,
                dy: 12.0,
                blur: 24.0,
                color: Color::rgba(0, 0, 0, 140),
            },
            glow: Shadow {
                dx: 0.0,
                dy: 0.0,
                blur: 18.0,
                color: Color::rgba(79, 140, 255, 160),
            },
        }
    }
}

/// Durations in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Durations {
    pub fast: f32,
    pub base: f32,
    pub slow: f32,
    /// Default `run_time` of enter animations.
    pub enter: f32,
    /// Default `run_time` of exit animations.
    pub exit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Easings {
    pub standard: RateFunc,
    pub enter: RateFunc,
    pub exit: RateFunc,
    pub emphasis: RateFunc,
}

/// Seconds between the items of a staggered group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stagger {
    pub tight: f32,
    pub loose: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub duration: Durations,
    pub easing: Easings,
    pub stagger: Stagger,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            duration: Durations {
                fast: 0.2,
                base: 0.4,
                slow: 0.8,
                enter: 0.6,
                exit: 0.4,
            },
            easing: Easings {
                standard: RateFunc::Smooth,
                enter: RateFunc::Smooth,
                exit: RateFunc::Smooth,
                emphasis: RateFunc::ThereAndBack,
            },
            stagger: Stagger {
                tight: 0.05,
                loose: 0.12,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color) -> f32 {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn default_text_is_readable() {
        let c = Colors::default();
        assert!(contrast(c.text, c.background) >= 7.0);
        assert!(contrast(c.title, c.background) >= 7.0);
        assert!(contrast(c.text, c.surface) >= 4.5);
        assert!(contrast(c.muted, c.background) >= 4.5);
        assert!(contrast(c.accent, c.background) >= 3.0);
    }

    #[test]
    fn default_motion_and_sizes_are_positive() {
        let t = Theme::default();
        let d = t.motion.duration;
        for v in [d.fast, d.base, d.slow, d.enter, d.exit] {
            assert!(v > 0.0);
        }
        assert!(d.fast < d.base && d.base < d.slow);
        assert!(t.motion.stagger.tight < t.motion.stagger.loose);
        let ty = t.typography;
        assert!(ty.title.size > ty.heading.size && ty.heading.size > ty.body.size);
        assert!(t.spacing.xs < t.spacing.xl);
    }

    #[test]
    fn partial_override_keeps_other_defaults() {
        let theme = Theme {
            colors: Colors {
                accent: Color::hex("#ff0000"),
                ..Colors::default()
            },
            ..Theme::default()
        };
        assert_eq!(theme.colors.accent, Color::hex("#ff0000"));
        assert_eq!(theme.colors.text, Colors::default().text);
        assert_eq!(theme.motion, Motion::default());
        assert_eq!(theme.typography.body.font().family, "DM Sans");
    }
}
