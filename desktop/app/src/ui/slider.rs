//! Horizontal slider over a 0..=1 fraction.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, Bounds, DispatchPhase, ElementId, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, RenderOnce, Role,
    StatefulInteractiveElement, Styled, Window, canvas, div, px, relative,
};

use super::interact::{Nav, focusable, nav_key};
use crate::design_system::{Shadow, shadow, tokens};

pub const ROW_H: f32 = 16.;
pub const TRACK_H: f32 = 4.;
/// Track height of the timeline zoom slider.
pub const ZOOM_TRACK_H: f32 = 3.;
pub const THUMB: f32 = 14.;
/// Default key and snap step.
pub const DEFAULT_STEP: f32 = 0.05;

/// Maps a pointer x to a fraction of the travel `[left, left + width]`.
/// `None` when the travel has no width.
pub fn fraction_from_x(x: f32, left: f32, width: f32) -> Option<f32> {
    (width > 0.).then(|| ((x - left) / width).clamp(0., 1.))
}

/// Rounds `fraction` to the nearest multiple of `step`. A step of zero or less
/// leaves the value untouched.
pub fn snap_to_step(fraction: f32, step: f32) -> f32 {
    let fraction = fraction.clamp(0., 1.);
    if step <= 0. {
        return fraction;
    }
    ((fraction / step).round() * step).clamp(0., 1.)
}

/// New fraction for a key press, or `None` when the key does nothing here or
/// the value would not change.
pub fn slider_key(fraction: f32, key: Nav, step: f32) -> Option<f32> {
    let next = match key {
        Nav::Left | Nav::Down => fraction - step,
        Nav::Right | Nav::Up => fraction + step,
        Nav::Home => 0.,
        Nav::End => 1.,
        Nav::Activate | Nav::Escape => return None,
    }
    .clamp(0., 1.);
    (next != fraction).then_some(next)
}

/// Pointer-drag state of one slider; kept by gpui across frames.
#[derive(Default)]
pub struct SliderDrag {
    active: Cell<bool>,
    bounds: Cell<Bounds<Pixels>>,
}

type Change = Rc<dyn Fn(f32, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    fraction: f32,
    track_h: f32,
    step: f32,
    disabled: bool,
    on_change: Option<Change>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>, fraction: f32) -> Self {
        Self {
            id: id.into(),
            fraction: fraction.clamp(0., 1.),
            track_h: TRACK_H,
            step: DEFAULT_STEP,
            disabled: false,
            on_change: None,
        }
    }

    /// Track thickness in pixels (use [`ZOOM_TRACK_H`] for the zoom slider).
    pub fn track(mut self, height: f32) -> Self {
        self.track_h = height;
        self
    }

    /// Key step and snap increment; 0 disables snapping and uses the default key step.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = tokens().palette;
        let drag = window.use_keyed_state((self.id.clone(), "drag".to_string()), cx, |_, _| {
            SliderDrag::default()
        });
        let key_step = if self.step > 0. {
            self.step
        } else {
            DEFAULT_STEP
        };
        let snap = self.step;
        let fraction = self.fraction;
        let disabled = self.disabled;
        let half_thumb = THUMB / 2.;

        let listeners = {
            let drag = drag.clone();
            let on_change = self.on_change.clone();
            canvas(
                {
                    let drag = drag.clone();
                    move |bounds, _, cx| drag.read(cx).bounds.set(bounds)
                },
                move |_, _, window, _| {
                    let moved = {
                        let drag = drag.clone();
                        let on_change = on_change.clone();
                        move |event: &MouseMoveEvent,
                              phase: DispatchPhase,
                              window: &mut Window,
                              cx: &mut App| {
                            let state = drag.read(cx);
                            if phase != DispatchPhase::Bubble
                                || !state.active.get()
                                || event.pressed_button != Some(MouseButton::Left)
                            {
                                return;
                            }
                            let bounds = state.bounds.get();
                            if let (Some(value), Some(change)) = (
                                fraction_from_x(
                                    f32::from(event.position.x),
                                    f32::from(bounds.left()),
                                    f32::from(bounds.size.width),
                                ),
                                &on_change,
                            ) {
                                change(snap_to_step(value, snap), window, cx);
                            }
                        }
                    };
                    window.on_mouse_event(moved);
                    let drag = drag.clone();
                    window.on_mouse_event(
                        move |_: &MouseUpEvent, _: DispatchPhase, _: &mut Window, cx: &mut App| {
                            drag.read(cx).active.set(false);
                        },
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };

        let track = div()
            .relative()
            .flex_none()
            .w_full()
            .h(px(self.track_h))
            .rounded_full()
            .bg(palette.ctl_active)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(relative(fraction))
                    .rounded_full()
                    .bg(palette.accent),
            );
        let travel = div()
            .relative()
            .flex()
            .items_center()
            .size_full()
            .child(listeners)
            .child(track)
            .child(
                div()
                    .absolute()
                    .top(px((ROW_H - THUMB) / 2.))
                    .left(relative(fraction))
                    .ml(px(-half_thumb))
                    .size(px(THUMB))
                    .rounded_full()
                    .bg(palette.thumb)
                    .shadow(shadow(Shadow::Thumb).into_vec()),
            );

        let mut element = focusable(
            div()
                .id(self.id.clone())
                .role(Role::Slider)
                .w_full()
                .h(px(ROW_H))
                .flex_none()
                .px(px(half_thumb))
                .child(travel),
            disabled,
        );
        if !disabled && let Some(change) = self.on_change.clone() {
            let down = drag.clone();
            let keys = change.clone();
            element = element
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, window, cx| {
                        let state = down.read(cx);
                        let bounds = state.bounds.get();
                        if let Some(value) = fraction_from_x(
                            f32::from(event.position.x),
                            f32::from(bounds.left()),
                            f32::from(bounds.size.width),
                        ) {
                            state.active.set(true);
                            change(snap_to_step(value, snap), window, cx);
                        }
                    },
                )
                .on_key_down(move |event, window, cx| {
                    if let Some(next) = nav_key(event.keystroke.key.as_str())
                        .and_then(|key| slider_key(fraction, key, key_step))
                    {
                        keys(next, window, cx);
                        cx.stop_propagation();
                    }
                });
        }
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!((ROW_H, TRACK_H, ZOOM_TRACK_H, THUMB), (16., 4., 3., 14.));
    }

    #[test]
    fn fraction_from_x_clamps_and_rejects_empty_travel() {
        assert_eq!(fraction_from_x(50., 0., 0.), None);
        assert_eq!(fraction_from_x(50., 0., -5.), None);
        assert_eq!(fraction_from_x(50., 0., 200.), Some(0.25));
        assert_eq!(fraction_from_x(-20., 0., 200.), Some(0.));
        assert_eq!(fraction_from_x(500., 100., 200.), Some(1.));
    }

    #[test]
    fn snap_to_step_rounds_to_nearest() {
        assert_eq!(snap_to_step(0.26, 0.25), 0.25);
        assert_eq!(snap_to_step(0.4, 0.25), 0.5);
        assert_eq!(snap_to_step(1.4, 0.25), 1.);
        assert_eq!(snap_to_step(0.333, 0.), 0.333);
    }

    #[test]
    fn keys_step_and_jump() {
        assert_eq!(slider_key(0.5, Nav::Right, 0.1), Some(0.6));
        assert_eq!(slider_key(0.5, Nav::Left, 0.25), Some(0.25));
        assert_eq!(slider_key(0.5, Nav::Home, 0.1), Some(0.));
        assert_eq!(slider_key(0.5, Nav::End, 0.1), Some(1.));
        assert_eq!(slider_key(1., Nav::Right, 0.1), None);
        assert_eq!(slider_key(0., Nav::Home, 0.1), None);
        assert_eq!(slider_key(0.5, Nav::Escape, 0.1), None);
    }
}
