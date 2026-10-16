//! The text element of rendered markdown and the root that makes a row's elements selectable.
//!
//! [`MdText`] lays out one flattened text (a paragraph, a heading, a table cell, a code block),
//! paints the selection wash and the streaming veil as run colours, and registers its layout with
//! the row's [`Selection`]. [`SelectRoot`] wraps the whole row: it owns the mouse listeners that
//! turn presses and drags into the selection, clicks on links into the row's link handler, and
//! the copy of the selected text.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ClipboardItem, CursorStyle, DispatchPhase, Element, ElementId,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString,
    StyledText, TextLayout, Window,
};

use crate::kit::CopySelection;

use super::inline::{LinkSpan, Seg, runs_for, wash};
use super::selection::{Entry, Selection};
use super::style::MdStyle;
use super::veil::{VeilSpan, apply_veil};

/// Shared selection state of one row.
pub type SharedSelection = Rc<RefCell<Selection>>;

/// What a click on a link does: destination, pointer position, window, app.
pub type LinkActivate = Rc<dyn Fn(&str, Point<Pixels>, &mut Window, &mut App)>;

/// One selectable text.
pub struct MdText {
    pub ix: usize,
    pub text: SharedString,
    pub segs: Vec<Seg>,
    pub veil: Vec<VeilSpan>,
    pub links: Rc<Vec<LinkSpan>>,
    pub sel: SharedSelection,
    pub style: Rc<MdStyle>,
    layout: TextLayout,
}

impl MdText {
    pub fn new(
        ix: usize,
        text: SharedString,
        segs: Vec<Seg>,
        veil: Vec<VeilSpan>,
        links: Rc<Vec<LinkSpan>>,
        sel: SharedSelection,
        style: Rc<MdStyle>,
    ) -> Self {
        Self {
            ix,
            text,
            segs,
            veil,
            links,
            sel,
            style,
            layout: TextLayout::default(),
        }
    }
}

impl IntoElement for MdText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for MdText {
    type RequestLayoutState = StyledText;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, StyledText) {
        let base = window.text_style();
        let mut runs = apply_veil(runs_for(&self.segs, &base, &self.style.mono), &self.veil);
        if let Some(range) = self.sel.borrow().range_for(self.ix) {
            runs = wash(runs, range, self.style.selection);
        }
        let mut styled = StyledText::new(self.text.clone());
        if !runs.is_empty() {
            styled = styled.with_runs(runs);
        }
        self.layout = styled.layout().clone();
        let (layout_id, ()) = styled.request_layout(None, inspector_id, window, cx);
        (layout_id, styled)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        styled: &mut StyledText,
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        styled.prepaint(None, inspector_id, bounds, &mut (), window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        styled: &mut StyledText,
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        styled.paint(None, inspector_id, bounds, &mut (), &mut (), window, cx);
        let pointer = window.mouse_position();
        let on_link = hitbox.is_hovered(window)
            && self
                .layout
                .index_for_position(pointer)
                .is_ok_and(|i| self.links.iter().any(|l| l.range.contains(&i)));
        window.set_cursor_style(
            if on_link {
                CursorStyle::PointingHand
            } else {
                CursorStyle::IBeam
            },
            hitbox,
        );
        self.sel.borrow_mut().register(
            self.ix,
            Entry {
                layout: self.layout.clone(),
                bounds,
                links: self.links.clone(),
            },
        );
    }
}

/// The text a [`SelectRoot`] last made copyable, read by the app-wide copy handler.
#[derive(Default)]
struct ActiveCopy(Option<String>);

impl gpui::Global for ActiveCopy {}

/// Installs the copy handler once. The kit's own handler runs first and passes the key on when
/// no kit text is selected; this one copies the markdown selection, and otherwise passes it on
/// so a focused text field still copies.
fn ensure_copy_handler(cx: &mut App) {
    if cx.has_global::<ActiveCopy>() {
        return;
    }
    cx.set_global(ActiveCopy::default());
    cx.on_action(
        |_: &CopySelection, cx| match cx.global::<ActiveCopy>().0.clone() {
            Some(text) => cx.write_to_clipboard(ClipboardItem::new_string(text)),
            None => cx.propagate(),
        },
    );
}

fn publish(sel: &Selection, cx: &mut App) {
    cx.global_mut::<ActiveCopy>().0 = sel.text();
}

/// Wraps a row's content and wires its mouse handling.
pub struct SelectRoot {
    child: AnyElement,
    sel: SharedSelection,
    activate: LinkActivate,
}

impl SelectRoot {
    pub fn new(child: AnyElement, sel: SharedSelection, activate: LinkActivate) -> Self {
        Self {
            child,
            sel,
            activate,
        }
    }
}

impl IntoElement for SelectRoot {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SelectRoot {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        // Inserted before the children: a modal layer inside the row (the link confirmation) then
        // sits above it, so presses on that layer never reach the row's selection.
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.child.prepaint(window, cx);
        hitbox
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        ensure_copy_handler(cx);
        self.sel.borrow_mut().begin_frame();
        self.child.paint(window, cx);
        let view = window.current_view();

        // A press outside the row ends its selection.
        window.on_mouse_event({
            let (sel, hitbox) = (self.sel.clone(), hitbox.clone());
            move |_: &MouseDownEvent, phase, window, cx| {
                if phase == DispatchPhase::Capture
                    && !hitbox.is_hovered(window)
                    && sel.borrow_mut().clear()
                {
                    publish(&sel.borrow(), cx);
                    cx.notify(view);
                }
            }
        });
        window.on_mouse_event({
            let (sel, hitbox) = (self.sel.clone(), hitbox.clone());
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !hitbox.is_hovered(window)
                {
                    return;
                }
                let changed = sel.borrow_mut().press(event.position, event.click_count);
                if changed {
                    publish(&sel.borrow(), cx);
                    cx.notify(view);
                }
            }
        });
        window.on_mouse_event({
            let sel = self.sel.clone();
            move |event: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                let mut state = sel.borrow_mut();
                if state.is_dragging() {
                    if state.drag_to(event.position) {
                        publish(&state, cx);
                        cx.notify(view);
                    }
                    return;
                }
                let over = state.link_at(event.position).is_some();
                if over != state.hover_link {
                    state.hover_link = over;
                    cx.notify(view);
                }
            }
        });
        window.on_mouse_event({
            let (sel, activate) = (self.sel.clone(), self.activate.clone());
            move |event: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                    return;
                }
                let link = sel.borrow_mut().release(event.position);
                if let Some(dest) = link {
                    activate(&dest, event.position, window, cx);
                }
            }
        });
    }
}
