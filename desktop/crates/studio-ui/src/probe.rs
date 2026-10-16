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

/// Where a nested view reports the bounds of its named parts: `(name, [left, top, width,
/// height], app)`. The owner of the view decides where they are kept.
pub type BoundsSink = Rc<dyn Fn(&str, [f32; 4], &mut App)>;

/// A bounds handler that reports under `name` to `sink`, for the kit components'
/// `on_bounds`.
pub fn report_to(
    sink: &Option<BoundsSink>,
    name: impl Into<String>,
) -> Option<impl Fn(Bounds<Pixels>, &mut App) + 'static> {
    let sink = sink.clone()?;
    let name = name.into();
    Some(move |bounds, cx: &mut App| sink(&name, bounds_to_record(bounds), cx))
}

/// An invisible element that fills its parent and reports the parent's bounds under `name`.
pub fn probe_element(
    sink: &Option<BoundsSink>,
    name: impl Into<String>,
) -> Option<gpui::AnyElement> {
    let report = report_to(sink, name)?;
    Some(crate::kit::bounds_probe(std::rc::Rc::new(report)))
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
