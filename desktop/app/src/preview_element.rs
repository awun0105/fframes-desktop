use crate::app::StudioSpikeApp;
use gpui::{
    App, Bounds, Corners, Element, ElementId, Entity, GlobalElementId, IntoElement, LayoutId,
    Pixels, RenderImage, Style, Window, point, px, relative, size,
};
use std::sync::Arc;
use studio_theme::{METRICS, Theme};

/// Paints the current image letterboxed into the element's bounds.
pub struct PreviewElement {
    pub app: Entity<StudioSpikeApp>,
    pub image: Option<Arc<RenderImage>>,
    pub dimensions: (u32, u32),
}

impl IntoElement for PreviewElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for PreviewElement {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.app
            .update(cx, |app, _| app.preview_bounds = Some(bounds));
        let Some(image) = &self.image else {
            return;
        };
        if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
            return;
        }
        let scale = (f32::from(bounds.size.width) / self.dimensions.0 as f32)
            .min(f32::from(bounds.size.height) / self.dimensions.1 as f32);
        let image_size = size(
            px(self.dimensions.0 as f32 * scale),
            px(self.dimensions.1 as f32 * scale),
        );
        let image_bounds = Bounds::new(
            point(
                bounds.left() + (bounds.size.width - image_size.width) / 2.,
                bounds.top() + (bounds.size.height - image_size.height) / 2.,
            ),
            image_size,
        );
        let radii = Corners::all(px(METRICS.radius_frame));
        // Corner radii round the clip rectangle, so the clip is the letterboxed frame.
        let painted = window.paint_image(image_bounds, radii, image.clone(), 0, false);
        if painted.is_ok() {
            let mut ring = gpui::outline(
                image_bounds,
                Theme::for_window(window, cx).editor.line,
                gpui::BorderStyle::Solid,
            );
            ring.corner_radii = radii;
            window.paint_quad(ring);
        }
        match painted {
            Ok(()) => self.app.update(cx, |app, _| {
                if let Some(source) = app.pending_source.take() {
                    app.displayed_source = Some(source);
                }
            }),
            Err(error) => self.app.update(cx, |app, cx| {
                app.status_message = format!("Image presentation failed: {error}");
                cx.notify();
            }),
        }
    }
}
