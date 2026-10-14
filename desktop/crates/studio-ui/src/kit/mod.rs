//! The shared component kit, ported from Cap's desktop app.
//!
//! Every surface of the studio builds its controls from here: buttons,
//! toggles, selects, sliders, menus, fields, cards and the text input. A view
//! resolves a [`studio_theme::Theme`] once per render and passes it to each
//! component's constructor.
//!
//! ## How a component is themed
//!
//! Components carry named constructors, one per surface:
//!
//! | constructor | surface | token set |
//! |---|---|---|
//! | `::body(theme, ..)` | window body | Radix (`Theme::body_*`) |
//! | `::settings(theme, ..)` | settings page | `Theme::settings_*` |
//! | `::plain(theme, ..)` | editor | Radix |
//!
//! A call site that needs a colour no surface provides sets it explicitly --
//! the builders all expose the individual colours -- so consolidation never
//! costs fidelity.
//!
//! ## Event handlers
//!
//! Handlers are `impl Fn(&ClickEvent, &mut Window, &mut App)`, which is exactly
//! what `cx.listener(..)` produces for any view type. Components are
//! therefore view-agnostic without a generic parameter.

// The kit is ported whole rather than trimmed to today's callers: half a
// variant scale sends the next caller back to the source to re-derive it.
#![allow(dead_code)]

mod button;
mod collapsible;
mod color_picker;
mod editor_button;
mod field;
mod kbd;
mod menu;
mod number_field;
mod progress;
mod radio_cards;
mod segmented;
mod select;
mod selection_header;
mod slider;
mod surface;
mod tab_rail;
pub mod text_input;
mod toggle;
mod tooltip;

pub use button::{Button, ButtonSize, ButtonVariant, ClickHandler, IconButton};
pub use collapsible::{Collapsible, CollapsibleState};
pub use color_picker::{
    ColorPicker, ColorPickerSnapshot, hsv_to_rgb, hue_from_point, preview_ring, rgb_to_hsv,
    sv_from_point,
};
pub use editor_button::{EditorButton, EditorButtonVariant};
pub use field::{Field, Subfield};
pub use kbd::{KbdChip, KbdSize, kbd_symbol};
pub use menu::{Menu, MenuItem, MenuKey, MenuState};
pub use number_field::{
    NumberChange, NumberField, NumberFieldState, NumberLimits, format_number, parse_number,
};
pub use progress::CircularProgress;
pub use radio_cards::{RadioCard, RadioCards};
pub use segmented::{SegmentOption, SegmentedControl, option_at};
pub use select::Select;
pub use selection_header::{SelectionHeader, selection_label, zoom_selection_label};
pub use slider::{
    Slider, SliderDrag, SliderTrack, fraction_from_x, snap_to_step, value_at as slider_value_at,
    value_from_fraction,
};
pub use surface::{Card, Popover, Section, SettingRow};
pub use tab_rail::{TabRail, TabRailItem};
pub use text_input::{
    TextInput, TextInputEvent, TextInputState, bind_keys as bind_text_input_keys,
    text_input_has_focus,
};
pub use toggle::{Toggle, ToggleSize};
pub use tooltip::{TOOLTIP_SHOW_DELAY, Tooltip, TooltipStyle};
