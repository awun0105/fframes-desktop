//! Draws fframes-components inside the 16:9 cut. A component is authored on its
//! own 30 fps clock and canvas; here it gets a box of the frame and the seconds
//! since it started, and measures text with the faces the video draws it in:
//! Geist at the display weight and Geist Mono.

use std::sync::LazyLock;

use fframes::{Frame, Svgr};
use fframes_components::{Component, ComponentFrame, Fonts, RenderCtx, TextMeter, TextMeters};

use super::look::{DISPLAY_WEIGHT, MONO, SANS};

/// The clock components are written against.
const COMPONENT_FPS: f32 = 30.0;

static METERS: LazyLock<TextMeters> = LazyLock::new(|| {
    let mut meters = fframes_components::embedded_meters();
    if let Some(display) = TextMeter::new(
        SANS,
        include_bytes!("../../../media/Geist-Book.ttf").to_vec(),
    ) {
        meters = meters.with_face(display);
    }
    meters.mono = TextMeter::new(
        MONO,
        include_bytes!("../../../media/GeistMono-Regular.ttf").to_vec(),
    );
    meters
});

/// The faces components draw in: Geist for headlines, Geist Mono for code.
pub fn fonts() -> Fonts {
    Fonts {
        sans: SANS.to_owned(),
        mono: MONO.to_owned(),
        weight_medium: DISPLAY_WEIGHT,
        weight_bold: DISPLAY_WEIGHT,
        weight_black: DISPLAY_WEIGHT,
    }
}

/// `component` at `seconds` since it started, laid out on a `w` x `h` canvas
/// whose top left corner sits at (`x`, `y`). Nothing before it starts.
pub fn show<'a, C: Component>(
    component: &'a C,
    frame: &Frame,
    seconds: f32,
    (x, y, w, h): (f32, f32, f32, f32),
) -> Svgr<'a> {
    if seconds < 0.0 {
        return Svgr::empty();
    }
    let mut frame = frame.clone();
    let info = ComponentFrame {
        local: seconds * COMPONENT_FPS,
        global: frame.global_index,
        fps: COMPONENT_FPS,
        duration: component.duration(),
        width: w,
        height: h,
    };
    let mut ctx = RenderCtx::new(&mut frame, fonts(), &METERS, info);
    let body = component.show(&mut ctx);
    fframes::svgr!(<g transform={format!("translate({x} {y})")}>{body}</g>)
}
