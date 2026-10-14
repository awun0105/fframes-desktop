//! The shared telemetry hook that reports an element's on-screen bounds.

use std::rc::Rc;

use gpui::{App, Bounds, Pixels};

type Record = Rc<dyn Fn([f32; 4], &mut App)>;

/// Receives `[left, top, width, height]` in window coordinates on every prepaint.
pub struct BoundsProbe(Record);

impl BoundsProbe {
    pub fn new(record: impl Fn([f32; 4], &mut App) + 'static) -> Self {
        Self(Rc::new(record))
    }

    /// The same report as a kit component's `on_bounds` handler.
    pub fn into_bounds_handler(self) -> impl Fn(Bounds<Pixels>, &mut App) + 'static {
        let record = self.0;
        move |bounds, cx| record(bounds_to_record(bounds), cx)
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
