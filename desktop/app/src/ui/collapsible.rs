//! Header plus content whose height animates open and closed.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Role, StatefulInteractiveElement, Styled, Transformation, Window,
    canvas, div, px, radians,
};

use super::icon::{Icon, icon};
use super::interact::{Handler, focusable, on_activate};
use crate::design_system::{NORMAL_MS, reduced_motion, tokens};

/// Chevron turn when open: a right-pointing chevron pointing down.
const OPEN_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Cubic ease-out of a 0..=1 progress.
pub fn ease_out_cubic(t: f32) -> f32 {
    1. - (1. - t.clamp(0., 1.)).powi(3)
}

/// Visible fraction of the content `elapsed_ms` after a toggle.
pub fn collapse_fraction(opening: bool, elapsed_ms: u64) -> f32 {
    let eased = ease_out_cubic(elapsed_ms as f32 / NORMAL_MS as f32);
    if opening { eased } else { 1. - eased }
}

/// Height of the clipped content for a given visible fraction.
pub fn collapse_height(content_h: f32, fraction: f32) -> f32 {
    content_h * fraction.clamp(0., 1.)
}

type Toggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// What the collapsible remembers between frames.
#[derive(Default)]
struct CollapsibleState {
    content_h: Rc<Cell<f32>>,
    mounted: Cell<bool>,
}

#[derive(IntoElement)]
pub struct Collapsible {
    id: ElementId,
    open: bool,
    header: Option<AnyElement>,
    content: Option<AnyElement>,
    on_toggle: Option<Toggle>,
}

impl Collapsible {
    pub fn new(id: impl Into<ElementId>, open: bool) -> Self {
        Self {
            id: id.into(),
            open,
            header: None,
            content: None,
            on_toggle: None,
        }
    }

    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header = Some(element.into_any_element());
        self
    }

    pub fn content(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    /// Called with the requested open state.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = tokens().palette;
        let metrics = tokens().metrics;
        let state = window.use_keyed_state((self.id.clone(), "state".to_string()), cx, |_, _| {
            CollapsibleState::default()
        });
        let state = state.read(cx);
        let animate = state.mounted.replace(true) && !reduced_motion();
        let measured = state.content_h.clone();

        let open = self.open;
        let toggle: Option<Handler> = self.on_toggle.map(|handler| {
            Rc::new(move |window: &mut Window, cx: &mut App| handler(!open, window, cx)) as Handler
        });
        let header = focusable(
            div()
                .id((self.id.clone(), "header".to_string()))
                .role(Role::DisclosureTriangle)
                .flex()
                .items_center()
                .gap(px(metrics.space_2))
                .child(
                    icon(Icon::ChevronRight, metrics.icon_sm, palette.text_2).with_transformation(
                        Transformation::rotate(radians(if open { OPEN_TURN } else { 0. })),
                    ),
                )
                .children(self.header),
            false,
        );
        let header = on_activate(header, false, toggle);

        let content = self.content.map(|content| {
            let height = measured.clone();
            let inner = div()
                .flex_none()
                .w_full()
                .child(
                    canvas(
                        move |bounds, _, _| height.set(f32::from(bounds.size.height)),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
                .relative()
                .child(content);
            let clip = div().w_full().overflow_hidden().child(inner);
            if animate {
                let height = measured.clone();
                clip.with_animation(
                    (self.id.clone(), if open { "open" } else { "closed" }),
                    Animation::new(Duration::from_millis(NORMAL_MS)).with_easing(ease_out_cubic),
                    move |clip, t| {
                        let fraction = if open { t } else { 1. - t };
                        clip.h(px(collapse_height(height.get(), fraction)))
                    },
                )
                .into_any_element()
            } else if open {
                clip.into_any_element()
            } else {
                clip.h(px(0.)).into_any_element()
            }
        });

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .child(header)
            .children(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_is_the_motion_constant() {
        assert_eq!(NORMAL_MS, 200);
    }

    #[test]
    fn opening_height_interpolates_ease_out() {
        assert_eq!(collapse_fraction(true, 0), 0.);
        assert_eq!(collapse_fraction(true, 100), 0.875);
        assert_eq!(collapse_fraction(true, 200), 1.);
        assert_eq!(collapse_fraction(true, 400), 1.);
    }

    #[test]
    fn closing_mirrors_opening() {
        assert_eq!(collapse_fraction(false, 0), 1.);
        assert_eq!(collapse_fraction(false, 100), 0.125);
        assert_eq!(collapse_fraction(false, 200), 0.);
    }

    #[test]
    fn height_scales_with_content() {
        assert_eq!(collapse_height(80., 0.), 0.);
        assert_eq!(collapse_height(80., 0.5), 40.);
        assert_eq!(collapse_height(80., 2.), 80.);
    }
}
