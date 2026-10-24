//! Explainer kits, one Cargo feature each.

#[cfg(feature = "biology")]
pub mod biology;
#[cfg(feature = "chemistry")]
pub mod chemistry;
#[cfg(feature = "cs")]
pub mod cs;
#[cfg(any(feature = "cs", feature = "biology"))]
// the links and arrows wait for the biology kit's food web
#[cfg_attr(not(feature = "cs"), allow(dead_code))]
mod draw;
#[cfg(feature = "math")]
pub mod math;
#[cfg(feature = "physics")]
pub mod physics;

#[cfg(any(feature = "physics", feature = "chemistry"))]
use crate::{layout::Anchor, ui::Ui};

/// A caption-sized label next to `at` on the side `direction`.
#[cfg(any(feature = "physics", feature = "chemistry"))]
pub(crate) fn label<'a>(
    ui: &mut Ui<'a, '_, '_>,
    text: String,
    at: kurbo::Point,
    direction: Anchor,
    fill: fframes::Color,
) -> fframes::Svgr<'a> {
    let label = crate::label::Label {
        text: text.into(),
        x: at.x as f32,
        y: at.y as f32,
        direction,
        style: ui.theme.typography.caption,
        fill,
        ..ui.defaults()
    };
    ui.show(label)
}

/// `v` with at most `max_decimals` decimals and no trailing zeros (`0.5`, `2`, `-1.25`).
#[cfg(any(feature = "math", feature = "physics", feature = "chemistry"))]
pub(crate) fn short(v: f64, max_decimals: usize) -> String {
    let s = format!("{v:.max_decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        &s
    };
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

#[cfg(all(
    test,
    any(feature = "math", feature = "physics", feature = "chemistry")
))]
mod tests {
    #[test]
    fn short_numbers_drop_trailing_zeros() {
        assert_eq!(super::short(0.5, 2), "0.5");
        assert_eq!(super::short(2.0, 2), "2");
        assert_eq!(super::short(-0.001, 2), "0");
        assert_eq!(super::short(60.0, 0), "60");
        assert_eq!(super::short(-1.257, 2), "-1.26");
    }
}
