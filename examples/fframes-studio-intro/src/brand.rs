//! Palette, type, text measurement and the brand's slam-in title. The HUD pieces
//! (`label`, `corners`) draw the components of fframes-kit.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use fframes::{Color, FFramesContext, FontQuery, FontStyle, Frame, Svgr};
use fframes_kit::prelude::{CornerMarks, HudLabel, TextAnchor, TextStyle};

pub const BG: &str = "#0b0b0b";
pub const BONE: &str = "#ece8e1";
pub const ACCENT: &str = "#2f80ff";
pub const EMBER: &str = "#14357a";
pub const GREY: &str = "#76726c";
pub const DIM: &str = "#2b2926";
pub const PAPER: &str = "#e8e4db";
pub const INK: &str = "#121110";

pub const DISPLAY: &str = "Archivo Black";
pub const COND: &str = "Anton";
pub const MONO: &str = "IBM Plex Mono";
pub const SERIF: &str = "Instrument Serif";

/// (family, size, weight, italic, text)
type WidthKey = (String, usize, u16, bool, String);

static WIDTHS: LazyLock<Mutex<HashMap<WidthKey, f32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Width of a string in pixels, measured once per (font, text) and cached.
/// Falls back to an estimate while fonts are not available.
pub fn measure(
    frame: &mut Frame,
    ctx: &FFramesContext,
    family: &str,
    size: usize,
    weight: u16,
    italic: bool,
    text: &str,
) -> f32 {
    let key = (family.to_owned(), size, weight, italic, text.to_owned());
    if let Some(w) = WIDTHS.lock().unwrap().get(&key) {
        return *w;
    }
    let query = FontQuery {
        family,
        size,
        weight,
        style: if italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        ..Default::default()
    };
    // `text_width` wants the text to live as long as the context; results are
    // cached, so every distinct string is leaked at most once.
    let text: &'static str = Box::leak(text.to_owned().into_boxed_str());
    match frame.text_width(ctx, query, text) {
        Some(w) => {
            let w = w as f32;
            WIDTHS.lock().unwrap().insert(key, w);
            w
        }
        None => text.chars().count() as f32 * size as f32 * 0.6,
    }
}

/// Crop-mark corners around a rectangle.
pub fn corners(x: f32, y: f32, w: f32, h: f32, arm: f32, color: &str, width: f32) -> Svgr<'static> {
    CornerMarks {
        x,
        y,
        width: w,
        height: h,
        arm,
        color: Color::hex(color),
        stroke_width: width,
        enter: None,
    }
    .draw()
}

/// A small mono label, uppercase tracking like engineering drawings.
pub fn label(
    x: f32,
    y: f32,
    text: String,
    color: &str,
    size: f32,
    anchor: &'static str,
) -> Svgr<'static> {
    HudLabel {
        text: text.into(),
        x,
        y,
        style: TextStyle {
            family: MONO,
            size: size as usize,
            weight: 500,
            line_height: 1.2,
        },
        fill: Color::hex(color),
        anchor: match anchor {
            "middle" => TextAnchor::Middle,
            "end" => TextAnchor::End,
            _ => TextAnchor::Start,
        },
        letter_spacing: 2.5,
        uppercase: false,
        opacity: 1.0,
    }
    .draw()
}

/// A word that slams in with a spring from `(dx, dy)` and leaves a motion
/// trail of ember copies behind it while it moves (the fframes brand motif).
pub struct Slam {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub family: &'static str,
    pub size: f32,
    pub fill: &'static str,
    pub spacing: f32,
    pub anchor: &'static str,
    pub italic: bool,
}

impl Slam {
    pub fn new(
        x: f32,
        y: f32,
        text: impl Into<String>,
        family: &'static str,
        size: f32,
        fill: &'static str,
    ) -> Self {
        Slam {
            x,
            y,
            text: text.into(),
            family,
            size,
            fill,
            spacing: -size * 0.035,
            anchor: "start",
            italic: false,
        }
    }
    pub fn anchor(mut self, anchor: &'static str) -> Self {
        self.anchor = anchor;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self.spacing = 0.0;
        self
    }

    /// `since` is beats since the entrance started.
    pub fn draw(&self, since: f32, dx: f32, dy: f32) -> Svgr<'static> {
        if since < 0.0 {
            return Svgr::empty();
        }
        let off = |t: f32| {
            let s = crate::beat::snap(t.max(0.0));
            ((1.0 - s) * dx, (1.0 - s) * dy)
        };
        let text = |ox: f32, oy: f32, fill: &str, opacity: f32| {
            let t = self.text.clone();
            let style = if self.italic { "italic" } else { "normal" };
            fframes::svgr!(
                <text x={self.x + ox} y={self.y + oy} text-anchor={self.anchor} font-family={self.family} font-style={style}
                      font-size={self.size} letter-spacing={self.spacing} fill={fill.to_owned()} opacity={opacity}>{t}</text>
            )
        };
        let mut layers = Vec::new();
        let trail = [
            (0.16, EMBER, 0.35),
            (0.10, "#2563c9", 0.5),
            (0.05, ACCENT, 0.65),
        ];
        for (lag, color, o) in trail {
            let (ox, oy) = off(since - lag);
            let (cx, cy) = off(since);
            let moving = ((ox - cx).powi(2) + (oy - cy).powi(2)).sqrt();
            if moving > 2.0 && since > lag * 0.5 {
                layers.push(text(ox, oy, color, o * (moving / 40.0).min(1.0)));
            }
        }
        let (ox, oy) = off(since);
        layers.push(text(ox, oy, self.fill, crate::beat::prog(since, 0.0, 0.06)));
        fframes::svgr!(<g>{layers}</g>)
    }
}
