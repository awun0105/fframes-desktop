//! The shared telemetry hook that reports an element's on-screen bounds.

use std::rc::Rc;

use gpui::{App, Bounds, IntoElement, ParentElement, Pixels, Styled, canvas};

type Record = Rc<dyn Fn([f32; 4], &mut App)>;

/// Receives `[left, top, width, height]` in window coordinates on every prepaint.
pub struct BoundsProbe(Record);

impl BoundsProbe {
    pub fn new(record: impl Fn([f32; 4], &mut App) + 'static) -> Self {
        Self(Rc::new(record))
    }
}

/// `[left, top, width, height]`, the format telemetry consumers read.
pub fn bounds_to_record(bounds: Bounds<Pixels>) -> [f32; 4] {
    [
        f32::from(bounds.left()),
        f32::from(bounds.top()),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    ]
}

/// Adds an absolute full-size canvas child that reports the element's bounds.
/// With `None` the element is returned unchanged. The element must be
/// positioned (`relative()`) so the canvas covers it.
pub fn with_probe<E: ParentElement>(element: E, probe: Option<BoundsProbe>) -> E {
    let mut element = element;
    if let Some(BoundsProbe(record)) = probe {
        element.extend(std::iter::once(
            canvas(
                move |bounds, _, cx| record(bounds_to_record(bounds), cx),
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .into_any_element(),
        ));
    }
    element
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size};

    use super::*;

    #[test]
    fn bounds_to_record_matches_existing_format() {
        let bounds = Bounds::new(point(px(12.5), px(30.)), size(px(96.), px(28.25)));
        assert_eq!(bounds_to_record(bounds), [12.5, 30., 96., 28.25]);
    }
}
