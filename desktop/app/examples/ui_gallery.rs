//! Every Studio UI component in every state, in one window, with a Light or
//! Dark switch. Used for screenshots and for comparing with the design spec.
//!
//! Run: `cargo run --manifest-path desktop/Cargo.toml -p fframes-studio --example ui_gallery`
//!
//! Hover and keyboard focus cannot be forced, so the gallery labels them: hover
//! a control to see its hover state and press Tab to move the focus ring through
//! them. A row at the end paints the focus ring statically.

use fframes_studio::design_system::{self, Appearance, TextStyle, text_style, tokens};
use fframes_studio::ui::{
    Badge, Button, ButtonVariant, Collapsible, FieldRow, Icon, IconButton, IconButtonSize, Kbd,
    Menu, MenuState, Notice, SectionTitle, SegmentOption, Segmented, Select, Slider, StatusDot,
    TabItem, TabRail, Toggle, Tone, card, field_value,
};
use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions,
    div, px, size,
};
use studio_ui::kit::{TextInput, TextInputState, bind_text_input_keys};

struct Gallery {
    toggle_on: bool,
    segment: usize,
    tab: usize,
    fraction: f32,
    zoom: f32,
    collapsible_open: bool,
    menu: MenuState,
    menu_open: bool,
    menu_focus: FocusHandle,
    notice_visible: bool,
    input: Entity<TextInputState>,
    compact_input: Entity<TextInputState>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            let mut input = TextInputState::single_line(window, cx);
            input.set_placeholder("Placeholder text");
            input
        });
        let compact_input = cx.new(|cx| {
            let mut input = TextInputState::single_line(window, cx);
            input.set_placeholder("Compact input");
            input.set_text("Typed value", cx);
            input
        });
        Self {
            toggle_on: true,
            segment: 1,
            tab: 0,
            fraction: 0.4,
            zoom: 0.6,
            collapsible_open: true,
            menu: MenuState::new(
                ["1080p", "1440p", "2160p", "Custom"]
                    .map(Into::into)
                    .to_vec(),
                Some(0),
            ),
            menu_open: false,
            menu_focus: cx.focus_handle(),
            notice_visible: true,
            input,
            compact_input,
        }
    }
}

/// A closure for gallery controls that updates the gallery.
fn update<A: 'static>(
    this: &Entity<Gallery>,
    change: impl Fn(&mut Gallery, A, &mut Window, &mut Context<Gallery>) + 'static,
) -> impl Fn(A, &mut Window, &mut App) + 'static {
    let this = this.clone();
    move |value, window, cx| {
        this.update(cx, |gallery, cx| {
            change(gallery, value, window, cx);
            cx.notify();
        })
    }
}

/// Like [`update`] for handlers that carry no value.
fn update0(
    this: &Entity<Gallery>,
    change: impl Fn(&mut Gallery, &mut Window, &mut Context<Gallery>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let this = this.clone();
    move |window, cx| {
        this.update(cx, |gallery, cx| {
            change(gallery, window, cx);
            cx.notify();
        })
    }
}

fn row(title: &str, children: impl IntoIterator<Item = gpui::AnyElement>) -> gpui::Div {
    let tokens = tokens();
    div()
        .flex()
        .flex_col()
        .gap(px(tokens.metrics.space_2))
        .child(SectionTitle::new(title.to_string()))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(tokens.metrics.space_3))
                .children(children),
        )
}

fn caption(text: &str) -> gpui::AnyElement {
    text_style(div(), TextStyle::Small)
        .text_color(tokens().palette.text_3)
        .child(text.to_string())
        .into_any_element()
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = studio_theme::Theme::for_window(window, cx);
        let this = cx.entity();
        let palette = tokens().palette;
        let metrics = tokens().metrics;
        let appearance = design_system::appearance();

        let appearance_switch = Segmented::new(
            "appearance",
            vec![SegmentOption::new("Light"), SegmentOption::new("Dark")],
            usize::from(appearance == Appearance::Dark),
        )
        .on_select(update(&this, |_, index, _, _| {
            design_system::set_appearance(if index == 0 {
                Appearance::Light
            } else {
                Appearance::Dark
            });
        }));

        let buttons = row(
            "Button",
            [
                Button::new("rest", "Rest").into_any_element(),
                Button::new("selected", "Selected")
                    .selected(true)
                    .into_any_element(),
                Button::new("disabled", "Disabled")
                    .disabled(true)
                    .into_any_element(),
                Button::new("primary", "Primary")
                    .variant(ButtonVariant::Primary)
                    .into_any_element(),
                Button::new("primary-off", "Primary disabled")
                    .variant(ButtonVariant::Primary)
                    .disabled(true)
                    .into_any_element(),
                Button::new("destructive", "Destructive")
                    .variant(ButtonVariant::Destructive)
                    .into_any_element(),
                Button::new("ghost", "Ghost")
                    .variant(ButtonVariant::Ghost)
                    .into_any_element(),
                Button::new("icons", "Build")
                    .icon(Icon::Hammer)
                    .trailing_icon(Icon::ChevronDown)
                    .tooltip("Build the preview")
                    .into_any_element(),
                caption("hover for the hover state, press Tab for focus"),
            ],
        );

        let icon_buttons = row(
            "IconButton",
            [
                IconButton::new("ib-small", Icon::Play)
                    .size(IconButtonSize::Small)
                    .label("Small")
                    .into_any_element(),
                IconButton::new("ib-medium", Icon::Pause)
                    .label("Medium")
                    .into_any_element(),
                IconButton::new("ib-large", Icon::Play)
                    .size(IconButtonSize::Large)
                    .round(true)
                    .variant(ButtonVariant::Primary)
                    .label("Large, round")
                    .into_any_element(),
                IconButton::new("ib-selected", Icon::Eye)
                    .selected(true)
                    .label("Selected")
                    .into_any_element(),
                IconButton::new("ib-disabled", Icon::Settings)
                    .disabled(true)
                    .label("Disabled")
                    .into_any_element(),
            ],
        );

        let toggles = row(
            "Toggle",
            [
                Toggle::new("toggle", self.toggle_on)
                    .label("Live toggle")
                    .on_change(update(&this, |gallery, value, _, _| {
                        gallery.toggle_on = value
                    }))
                    .into_any_element(),
                Toggle::new("toggle-off", false).into_any_element(),
                Toggle::new("toggle-disabled", true)
                    .disabled(true)
                    .into_any_element(),
            ],
        );

        let segmented = row(
            "Segmented",
            [
                Segmented::new(
                    "segmented",
                    vec![
                        SegmentOption::new("Fit"),
                        SegmentOption::new("50%"),
                        SegmentOption::new("100%").disabled(true),
                        SegmentOption::new("200%"),
                    ],
                    self.segment,
                )
                .on_select(update(&this, |gallery, index, _, _| {
                    gallery.segment = index
                }))
                .into_any_element(),
                caption("Left and Right move; the disabled option is skipped"),
            ],
        );

        let select = row(
            "Select and Menu",
            [
                div()
                    .w(px(200.))
                    .child(
                        Select::new(
                            "select",
                            self.menu.items()[self.menu.selected().unwrap_or(0)].clone(),
                        )
                        .menu(&self.menu)
                        .on_open(update0(&this, |gallery, window, cx| {
                            gallery.menu_open = true;
                            gallery.menu_focus.focus(window, cx);
                        }))
                        .into_any_element(),
                    )
                    .into_any_element(),
                caption("Up, Down, Home, End, Enter, Escape"),
            ],
        );

        let slider = row(
            "Slider",
            [
                div()
                    .w(px(220.))
                    .child(
                        Slider::new("slider", self.fraction)
                            .on_change(update(&this, |gallery, value, _, _| {
                                gallery.fraction = value
                            })),
                    )
                    .into_any_element(),
                div()
                    .w(px(72.))
                    .child(
                        Slider::new("zoom", self.zoom)
                            .track(design_system_zoom_track())
                            .on_change(update(&this, |gallery, value, _, _| gallery.zoom = value)),
                    )
                    .into_any_element(),
                div()
                    .w(px(120.))
                    .child(Slider::new("slider-off", 0.5).disabled(true))
                    .into_any_element(),
            ],
        );

        let tabs = row(
            "TabRail",
            [div()
                .w(px(240.))
                .child(
                    TabRail::new(
                        "tabs",
                        vec![
                            TabItem::new(Icon::Folder, "Project"),
                            TabItem::new(Icon::Image, "Assets"),
                            TabItem::new(Icon::Palette, "Style"),
                        ],
                        self.tab,
                    )
                    .on_select(update(&this, |gallery, index, _, _| gallery.tab = index)),
                )
                .into_any_element()],
        );

        let chips = row(
            "Kbd, Badge, StatusDot",
            [
                Kbd::new("Space").into_any_element(),
                Kbd::new("Ctrl K").into_any_element(),
                Badge::new("Draft").into_any_element(),
                Badge::new("Ready").tone(Tone::Success).into_any_element(),
                Badge::new("Stale").tone(Tone::Warning).into_any_element(),
                Badge::new("Failed").tone(Tone::Danger).into_any_element(),
                Badge::new("Info").tone(Tone::Info).into_any_element(),
                StatusDot::new(Tone::Neutral).into_any_element(),
                StatusDot::new(Tone::Success).into_any_element(),
                StatusDot::new(Tone::Warning).into_any_element(),
                StatusDot::new(Tone::Danger).into_any_element(),
            ],
        );

        let fields = div()
            .w(px(360.))
            .flex()
            .flex_col()
            .child(SectionTitle::new("Field rows").trailing(Badge::new("3")))
            .child(FieldRow::new("Width", field_value("1920 px")))
            .child(
                FieldRow::new("Hardware", Toggle::new("field-toggle", true))
                    .hint("Uses the GPU encoder when available"),
            )
            .child(FieldRow::new(
                "Quality",
                Slider::new("field-slider", 0.7).on_change(|_, _, _| {}),
            ));

        let collapsible = div().w(px(360.)).child(
            Collapsible::new("collapsible", self.collapsible_open)
                .header(text_style(div(), TextStyle::Label).child("Advanced"))
                .content(
                    div()
                        .pt(px(metrics.space_2))
                        .child(muted_text("Content that slides in over 200 ms.")),
                )
                .on_toggle(update(&this, |gallery, open, _, _| {
                    gallery.collapsible_open = open
                })),
        );

        let notices = div()
            .flex()
            .flex_col()
            .gap(px(metrics.space_2))
            .w(px(480.))
            .child(SectionTitle::new("Notice"))
            .child(
                Notice::new("n-info", Tone::Info, "Preview is building")
                    .body("This takes a few seconds."),
            )
            .child(Notice::new("n-success", Tone::Success, "Saved").body("Checkpoint written."))
            .child(
                Notice::new("n-warning", Tone::Warning, "Preview is out of date")
                    .body("The source changed after the last build.")
                    .action(
                        Button::new("n-build", "Build preview").variant(ButtonVariant::Primary),
                    ),
            )
            .children(self.notice_visible.then(|| {
                Notice::new("n-danger", Tone::Danger, "Build failed")
                    .body("The project did not compile.")
                    .details(
                        "error[E0425]: cannot find value `x` in this scope\n  --> src/lib.rs:12:5",
                    )
                    .action(Button::new("n-retry", "Retry"))
                    .on_dismiss(update0(&this, |gallery, _, _| {
                        gallery.notice_visible = false
                    }))
            }));

        let inputs = div()
            .w(px(360.))
            .flex()
            .flex_col()
            .gap(px(metrics.space_2))
            .child(SectionTitle::new("TextInput"))
            .child(TextInput::plain(&theme, "gallery-input", &self.input))
            .child(TextInput::plain(
                &theme,
                "gallery-compact-input",
                &self.compact_input,
            ));

        let focus_ring = row(
            "Focus ring (static sample)",
            [div()
                .rounded(px(metrics.radius_control))
                .shadow(fframes_studio::ui::interact::focus_ring())
                .child(Button::new("ring", "Focused"))
                .into_any_element()],
        );

        let content = div()
            .flex()
            .flex_col()
            .gap(px(metrics.space_6))
            .p(px(metrics.space_6))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        text_style(div(), TextStyle::Heading)
                            .text_color(palette.text_1)
                            .child("fframes Studio UI gallery"),
                    )
                    .child(appearance_switch),
            )
            .child(
                card()
                    .p(px(metrics.space_4))
                    .flex()
                    .flex_col()
                    .gap(px(metrics.space_4))
                    .child(buttons)
                    .child(icon_buttons)
                    .child(toggles)
                    .child(segmented)
                    .child(select)
                    .child(slider)
                    .child(tabs)
                    .child(chips)
                    .child(focus_ring),
            )
            .child(
                card()
                    .p(px(metrics.space_4))
                    .flex()
                    .flex_wrap()
                    .gap(px(metrics.space_6))
                    .child(fields)
                    .child(collapsible)
                    .child(inputs)
                    .child(notices),
            );

        let menu = self.menu_open.then(|| {
            Menu::new("menu", &self.menu, &self.menu_focus)
                .on_highlight(update(&this, |gallery, index, _, _| {
                    gallery.menu.set_highlighted(index)
                }))
                .on_select(update(&this, |gallery, index, _, _| {
                    gallery.menu.set_selected(Some(index));
                    gallery.menu_open = false;
                }))
                .on_dismiss(update0(&this, |gallery, _, _| gallery.menu_open = false))
        });

        div()
            .id("gallery")
            .size_full()
            .overflow_y_scroll()
            .bg(palette.window)
            .text_color(palette.text_1)
            .child(content)
            .children(menu)
    }
}

fn design_system_zoom_track() -> f32 {
    fframes_studio::ui::slider::ZOOM_TRACK_H
}

fn muted_text(text: &str) -> gpui::Div {
    text_style(div(), TextStyle::Body)
        .text_color(tokens().palette.text_2)
        .child(text.to_string())
}

fn main() {
    gpui_platform::application()
        .with_assets(fframes_studio::assets::Assets)
        .run(|cx: &mut App| {
            bind_text_input_keys(cx);
            design_system::set_appearance(design_system::appearance_from_window(
                cx.window_appearance(),
            ));
            let bounds = Bounds::centered(None, size(px(1120.), px(900.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| Gallery::new(window, cx)),
            )
            .expect("failed to open the gallery window");
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            cx.activate(true);
        });
}
