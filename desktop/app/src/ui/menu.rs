//! Popup menu. The caller owns a [`MenuState`] and renders a [`Menu`] while
//! it is open.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Anchor, AnchoredPositionMode, App, Bounds, ElementId, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement,
    Styled, Window, anchored, deferred, div, point, px,
};

use super::icon::{Icon, icon};
use super::interact::{Nav, nav_key};
use crate::design_system::{Shadow, TextStyle, shadow, text_style, tokens};

pub const RADIUS: f32 = 12.;
pub const PADDING: f32 = 4.;
pub const ROW_H: f32 = 24.;
pub const ROW_PAD_X: f32 = 6.;
pub const ROW_RADIUS: f32 = 4.;
pub const MIN_W: f32 = 180.;
pub const MAX_H: f32 = 320.;
/// Space between the anchor and the menu.
pub const ANCHOR_GAP: f32 = 4.;
/// Distance kept from the window edges.
pub const WINDOW_MARGIN: f32 = 12.;
/// Width reserved for the checkmark column.
const CHECK_SLOT: f32 = 16.;

/// Items, the committed choice and the keyboard highlight of one menu.
#[derive(Clone)]
pub struct MenuState {
    items: Vec<SharedString>,
    /// Rows that are shown but cannot be highlighted or chosen.
    disabled: Vec<bool>,
    selected: Option<usize>,
    highlighted: usize,
    /// Window bounds of the element that opened the menu, written by it.
    anchor: Rc<Cell<Bounds<Pixels>>>,
}

impl MenuState {
    pub fn new(items: Vec<SharedString>, selected: Option<usize>) -> Self {
        let highlighted = selected.filter(|i| *i < items.len()).unwrap_or(0);
        Self {
            disabled: vec![false; items.len()],
            items,
            selected,
            highlighted,
            anchor: Rc::new(Cell::new(Bounds::default())),
        }
    }

    /// Marks rows as disabled (`true`); missing entries stay enabled. The
    /// highlight moves to the first enabled row when it sits on a disabled one.
    pub fn with_disabled(mut self, disabled: Vec<bool>) -> Self {
        self.set_disabled(disabled);
        self
    }

    /// [`Self::with_disabled`] on an open menu, for availability that changes
    /// while the menu is showing.
    pub fn set_disabled(&mut self, disabled: Vec<bool>) {
        self.disabled = (0..self.items.len())
            .map(|i| disabled.get(i).copied().unwrap_or(false))
            .collect();
        if self.is_disabled(self.highlighted) {
            self.highlighted = step_enabled(&self.disabled, self.highlighted, 1).unwrap_or(0);
        }
    }

    pub fn is_disabled(&self, index: usize) -> bool {
        self.disabled.get(index).copied().unwrap_or(false)
    }

    pub fn items(&self) -> &[SharedString] {
        &self.items
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    pub fn set_highlighted(&mut self, index: usize) {
        if index < self.items.len() && !self.is_disabled(index) {
            self.highlighted = index;
        }
    }

    pub fn set_selected(&mut self, selected: Option<usize>) {
        self.selected = selected;
        self.highlighted = selected.filter(|i| *i < self.items.len()).unwrap_or(0);
    }

    /// The bounds cell a trigger fills so the menu can be placed.
    pub fn anchor_cell(&self) -> Rc<Cell<Bounds<Pixels>>> {
        self.anchor.clone()
    }
}

/// What a key press does to an open menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Highlight(usize),
    Commit(usize),
    Dismiss,
    Ignore,
}

/// The enabled row reached from `from` by `step` (1 or -1) in a wrapping walk
/// that skips disabled rows. `None` when every row is disabled.
fn step_enabled(disabled: &[bool], from: usize, step: isize) -> Option<usize> {
    let len = disabled.len();
    (1..=len)
        .map(|n| (from as isize + step * n as isize).rem_euclid(len as isize) as usize)
        .find(|i| !disabled[*i])
}

/// Key handling: Up and Down wrap and skip disabled rows, Home and End jump to
/// the first and last enabled row, Enter or Space commit the highlighted row
/// unless it is disabled, Escape dismisses.
pub fn menu_key(state: &MenuState, key: Nav) -> MenuKey {
    let len = state.items.len();
    if len == 0 {
        return if key == Nav::Escape {
            MenuKey::Dismiss
        } else {
            MenuKey::Ignore
        };
    }
    let current = state.highlighted.min(len - 1);
    let highlight = |index: Option<usize>| index.map_or(MenuKey::Ignore, MenuKey::Highlight);
    match key {
        Nav::Down => highlight(step_enabled(&state.disabled, current, 1)),
        Nav::Up => highlight(step_enabled(&state.disabled, current, -1)),
        Nav::Home => highlight(step_enabled(&state.disabled, len - 1, 1)),
        Nav::End => highlight(step_enabled(&state.disabled, 0, -1)),
        Nav::Activate if state.is_disabled(current) => MenuKey::Ignore,
        Nav::Activate => MenuKey::Commit(current),
        Nav::Escape => MenuKey::Dismiss,
        Nav::Left | Nav::Right => MenuKey::Ignore,
    }
}

/// Height of a menu with `rows` rows, capped at [`MAX_H`].
pub fn menu_height(rows: usize) -> f32 {
    (rows as f32 * ROW_H + 2. * PADDING).min(MAX_H)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Top edge of the menu in window coordinates.
    pub top: f32,
    pub above: bool,
}

/// Opens below the anchor with a 4 px gap; flips above when it would leave
/// less than the window margin below and the room above is larger.
pub fn menu_placement(
    anchor_top: f32,
    anchor_bottom: f32,
    height: f32,
    window_h: f32,
) -> Placement {
    let below_top = anchor_bottom + ANCHOR_GAP;
    let fits_below = below_top + height + WINDOW_MARGIN <= window_h;
    let above_top = anchor_top - ANCHOR_GAP - height;
    let room_above = anchor_top - ANCHOR_GAP - WINDOW_MARGIN;
    let room_below = window_h - WINDOW_MARGIN - below_top;
    if fits_below || room_above <= room_below {
        Placement {
            top: below_top,
            above: false,
        }
    } else {
        Placement {
            top: above_top.max(WINDOW_MARGIN),
            above: true,
        }
    }
}

type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Menu {
    id: ElementId,
    state: MenuState,
    focus: FocusHandle,
    row_ids: Vec<SharedString>,
    on_highlight: Option<Pick>,
    on_select: Option<Pick>,
    on_dismiss: Option<Dismiss>,
}

impl Menu {
    /// `focus` must be focused by the caller when the menu opens so the keys
    /// reach it.
    pub fn new(id: impl Into<ElementId>, state: &MenuState, focus: &FocusHandle) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            focus: focus.clone(),
            row_ids: Vec::new(),
            on_highlight: None,
            on_select: None,
            on_dismiss: None,
        }
    }

    /// Element ids for the rows, in order; rows without one get `(menu id, index)`.
    pub fn row_ids(mut self, ids: Vec<SharedString>) -> Self {
        self.row_ids = ids;
        self
    }

    pub fn on_highlight(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_highlight = Some(Rc::new(handler));
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let palette = tokens().palette;
        let anchor = self.state.anchor.get();
        let placement = menu_placement(
            f32::from(anchor.top()),
            f32::from(anchor.bottom()),
            menu_height(self.state.items.len()),
            f32::from(window.viewport_size().height),
        );
        let width = f32::from(anchor.size.width).max(MIN_W);

        let on_key = {
            let state = self.state.clone();
            let on_highlight = self.on_highlight.clone();
            let on_select = self.on_select.clone();
            let on_dismiss = self.on_dismiss.clone();
            move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                let Some(key) = nav_key(event.keystroke.key.as_str()) else {
                    return;
                };
                match menu_key(&state, key) {
                    MenuKey::Highlight(index) => {
                        if let Some(handler) = &on_highlight {
                            handler(index, window, cx);
                        }
                    }
                    MenuKey::Commit(index) => {
                        if let Some(handler) = &on_select {
                            handler(index, window, cx);
                        }
                    }
                    MenuKey::Dismiss => {
                        if let Some(handler) = &on_dismiss {
                            handler(window, cx);
                        }
                    }
                    MenuKey::Ignore => return,
                }
                cx.stop_propagation();
            }
        };

        let rows = self.state.items.iter().enumerate().map(|(index, label)| {
            let disabled = self.state.is_disabled(index);
            let highlighted = index == self.state.highlighted && !disabled;
            let chosen = self.state.selected == Some(index);
            let pick = self.on_select.clone().filter(|_| !disabled);
            let row = text_style(
                div()
                    .id(match self.row_ids.get(index) {
                        Some(id) => ElementId::from(id.clone()),
                        None => ElementId::from((self.id.clone(), index.to_string())),
                    })
                    .role(Role::MenuItem),
                TextStyle::Label,
            )
            .aria_label(label.clone())
            .flex()
            .items_center()
            .gap(px(ROW_PAD_X))
            .h(px(ROW_H))
            .px(px(ROW_PAD_X))
            .rounded(px(ROW_RADIUS))
            .text_color(if disabled {
                palette.text_3
            } else {
                palette.text_1
            })
            .when(!disabled, |row| {
                row.cursor_pointer()
                    .hover(|style| style.bg(palette.ctl_hover))
            })
            .on_click(move |_, window, cx| {
                if let Some(pick) = &pick {
                    pick(index, window, cx);
                }
            })
            .child(div().w(px(CHECK_SLOT)).flex_none().children(
                chosen.then(|| icon(Icon::Check, tokens().metrics.icon_sm, palette.text_1)),
            ))
            .child(label.clone());
            if highlighted {
                row.bg(palette.ctl_hover)
            } else {
                row
            }
        });

        let menu = div()
            .id(self.id.clone())
            .role(Role::Menu)
            .track_focus(&self.focus)
            .occlude()
            .on_key_down(on_key)
            .on_mouse_down_out({
                let on_dismiss = self.on_dismiss.clone();
                // A press on the anchor belongs to the trigger, which toggles the menu.
                move |event: &gpui::MouseDownEvent, window, cx| {
                    if anchor.contains(&event.position) {
                        return;
                    }
                    if let Some(handler) = &on_dismiss {
                        handler(window, cx);
                    }
                }
            })
            .flex()
            .flex_col()
            .w(px(width))
            .max_h(px(MAX_H))
            .overflow_y_scroll()
            .p(px(PADDING))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(palette.line)
            .bg(palette.card)
            .shadow(shadow(Shadow::Pop).into_vec())
            .children(rows);

        let left = f32::from(anchor.left());
        deferred(
            anchored()
                .position_mode(AnchoredPositionMode::Window)
                .position(point(px(left), px(placement.top)))
                .anchor(Anchor::TopLeft)
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(menu),
        )
        .with_priority(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(len: usize, highlighted: usize) -> MenuState {
        let items = (0..len)
            .map(|i| SharedString::from(format!("item {i}")))
            .collect();
        let mut state = MenuState::new(items, None);
        state.set_highlighted(highlighted);
        state
    }

    #[test]
    fn size_constants_match_spec() {
        assert_eq!(
            (RADIUS, PADDING, ROW_H, ROW_PAD_X, ROW_RADIUS),
            (12., 4., 24., 6., 4.)
        );
        assert_eq!(
            (MIN_W, MAX_H, ANCHOR_GAP, WINDOW_MARGIN),
            (180., 320., 4., 12.)
        );
        assert_eq!(menu_height(3), 80.);
        assert_eq!(menu_height(100), 320.);
    }

    #[test]
    fn down_and_up_wrap() {
        assert_eq!(menu_key(&state(3, 2), Nav::Down), MenuKey::Highlight(0));
        assert_eq!(menu_key(&state(3, 0), Nav::Up), MenuKey::Highlight(2));
        assert_eq!(menu_key(&state(3, 0), Nav::Down), MenuKey::Highlight(1));
    }

    #[test]
    fn home_end_commit_and_dismiss() {
        assert_eq!(menu_key(&state(4, 2), Nav::Home), MenuKey::Highlight(0));
        assert_eq!(menu_key(&state(4, 1), Nav::End), MenuKey::Highlight(3));
        assert_eq!(menu_key(&state(4, 2), Nav::Activate), MenuKey::Commit(2));
        assert_eq!(menu_key(&state(4, 2), Nav::Escape), MenuKey::Dismiss);
        assert_eq!(menu_key(&state(4, 2), Nav::Left), MenuKey::Ignore);
    }

    #[test]
    fn disabled_rows_are_skipped_by_every_key() {
        let at = |flags: &[bool], highlighted: usize| {
            let mut state = state(flags.len(), 0).with_disabled(flags.to_vec());
            state.highlighted = highlighted;
            state
        };
        let flags = [true, false, true, false, true];
        assert_eq!(menu_key(&at(&flags, 1), Nav::Down), MenuKey::Highlight(3));
        assert_eq!(menu_key(&at(&flags, 3), Nav::Down), MenuKey::Highlight(1));
        assert_eq!(menu_key(&at(&flags, 1), Nav::Up), MenuKey::Highlight(3));
        assert_eq!(menu_key(&at(&flags, 3), Nav::Home), MenuKey::Highlight(1));
        assert_eq!(menu_key(&at(&flags, 1), Nav::End), MenuKey::Highlight(3));
    }

    #[test]
    fn availability_can_change_while_the_menu_is_open() {
        let mut menu = state(3, 0);
        assert_eq!(menu_key(&menu, Nav::Activate), MenuKey::Commit(0));
        menu.set_disabled(vec![true, false, false]);
        assert!(menu.is_disabled(0));
        assert_eq!(menu.highlighted(), 1);
        menu.set_disabled(vec![]);
        assert!(!menu.is_disabled(0));
    }

    #[test]
    fn disabled_rows_cannot_be_highlighted_or_committed() {
        let mut menu = state(3, 0).with_disabled(vec![true, false, true]);
        assert_eq!(menu.highlighted(), 1);
        menu.set_highlighted(2);
        assert_eq!(menu.highlighted(), 1);
        menu.highlighted = 0;
        assert_eq!(menu_key(&menu, Nav::Activate), MenuKey::Ignore);
        menu.highlighted = 1;
        assert_eq!(menu_key(&menu, Nav::Activate), MenuKey::Commit(1));
    }

    #[test]
    fn a_menu_of_only_disabled_rows_ignores_navigation_and_commit() {
        let menu = state(2, 0).with_disabled(vec![true, true]);
        for key in [Nav::Down, Nav::Up, Nav::Home, Nav::End, Nav::Activate] {
            assert_eq!(menu_key(&menu, key), MenuKey::Ignore);
        }
        assert_eq!(menu_key(&menu, Nav::Escape), MenuKey::Dismiss);
    }

    #[test]
    fn empty_menu_only_dismisses() {
        assert_eq!(menu_key(&state(0, 0), Nav::Activate), MenuKey::Ignore);
        assert_eq!(menu_key(&state(0, 0), Nav::Escape), MenuKey::Dismiss);
    }

    #[test]
    fn highlight_starts_on_the_selected_row() {
        let items = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(MenuState::new(items, Some(2)).highlighted(), 2);
    }

    #[test]
    fn opens_below_when_there_is_room() {
        let placement = menu_placement(100., 130., 80., 800.);
        assert_eq!(
            placement,
            Placement {
                top: 134.,
                above: false
            }
        );
    }

    #[test]
    fn flips_above_near_the_bottom() {
        let placement = menu_placement(700., 730., 200., 760.);
        assert!(placement.above);
        assert_eq!(placement.top, 700. - 4. - 200.);
    }

    #[test]
    fn stays_below_when_neither_side_fits_better() {
        // Short window: more room below than above.
        let placement = menu_placement(20., 50., 300., 300.);
        assert!(!placement.above);
    }
}
