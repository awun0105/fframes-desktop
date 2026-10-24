//! Explainer kits, one Cargo feature each.

#[cfg(feature = "math")]
pub mod math;
#[cfg(feature = "physics")]
pub mod physics;

/// `v` with at most `max_decimals` decimals and no trailing zeros (`0.5`, `2`, `-1.25`).
#[cfg(any(feature = "math", feature = "physics"))]
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

#[cfg(all(test, any(feature = "math", feature = "physics")))]
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
