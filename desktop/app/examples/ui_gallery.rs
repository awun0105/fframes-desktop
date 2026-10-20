//! Every Studio UI component in every state, in one window, with a System,
//! Light or Dark switch. Used for screenshots and for comparing with the
//! design spec.
//!
//! Run: `cargo run --manifest-path desktop/Cargo.toml -p fframes-studio --example ui_gallery`
//!
//! Hover and keyboard focus cannot be forced, so the gallery labels them: hover
//! a control to see its hover state and press Tab to move the focus ring through
//! them. A row at the end paints the focus ring statically.

use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, StatefulInteractiveElement, Styled, Window, WindowBounds,
    WindowOptions, div, px, size,
};
use studio_theme::{AppTheme, METRICS, Theme};
use studio_ui::kit::{
    Badge, Button, ButtonSize, ButtonVariant, Card, CollapsibleSection, EditorButton, Icon,
    IconButton, IconButtonSize, KbdChip, Menu, MenuItem, MenuState, NavRail, NavRailItem, Notice,
    Section, SegmentOption, SegmentedControl, Select, SettingRow, SettingsFrame, SettingsNavItem,
    Slider, SliderTrack, StatusDot, TabRail, TabRailItem, TextInput, TextInputState, Toggle, Tone,
    bind_text_input_keys, field_row, focus_ring, heading, label, muted, section_title,
};

const RESOLUTIONS: [&str; 4] = ["1080p", "1440p", "2160p", "Custom"];
const THEMES: [AppTheme; 3] = [AppTheme::System, AppTheme::Light, AppTheme::Dark];

struct Gallery {
    toggle_on: bool,
    segment: usize,
    tab: usize,
    drawer: Option<usize>,
    fraction: f32,
    zoom: f32,
    collapsible_open: bool,
    resolution: usize,
    menu: Option<MenuState>,
    menu_focus: FocusHandle,
    notice_visible: bool,
    input: Entity<TextInputState>,
    compact_input: Entity<TextInputState>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        studio_theme::bind_window(window, cx);
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
            drawer: Some(0),
            fraction: 0.4,
            zoom: 0.6,
            collapsible_open: true,
            resolution: 0,
            menu: None,
            menu_focus: cx.focus_handle(),
            notice_visible: true,
            input,
            compact_input,
        }
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        RESOLUTIONS
            .iter()
            .enumerate()
            .map(|(index, name)| MenuItem::new(*name, index == self.resolution))
            .collect()
    }
}

/// A closure for gallery controls that updates the gallery.
fn update<A: ?Sized + 'static>(
    this: &Entity<Gallery>,
    change: impl Fn(&mut Gallery, &A, &mut Window, &mut Context<Gallery>) + 'static,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
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

fn row(
    theme: &Theme,
    title: &str,
    children: impl IntoIterator<Item = gpui::AnyElement>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(METRICS.space_2))
        .child(section_title(theme, title.to_string()))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(METRICS.space_3))
                .children(children),
        )
}

fn caption(theme: &Theme, text: &str) -> gpui::AnyElement {
    muted(theme, text.to_string()).into_any_element()
}

fn icon_button(theme: &Theme, id: &'static str, glyph: Icon, name: &str) -> IconButton {
    IconButton::editor(theme, id, glyph.path())
        .label(name.to_string())
        .tooltip(theme, name.to_string())
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let colors = theme.editor;
        let this = cx.entity();
        let preference = studio_theme::current_preference(cx);

        let theme_switch = SegmentedControl::editor(
            &theme,
            "appearance",
            THEMES
                .iter()
                .map(|choice| {
                    let name = match choice {
                        AppTheme::System => "System",
                        AppTheme::Light => "Light",
                        AppTheme::Dark => "Dark",
                    };
                    SegmentOption::new(name, *choice == preference)
                })
                .collect(),
        )
        .on_select(|index, _, cx| {
            if let Some(choice) = THEMES.get(*index) {
                studio_theme::set_preference(*choice, cx);
            }
        });

        let buttons = row(
            &theme,
            "EditorButton and Button",
            [
                EditorButton::plain(&theme, "rest")
                    .label("Rest")
                    .into_any_element(),
                EditorButton::plain(&theme, "pressed")
                    .label("Pressed")
                    .pressed(true)
                    .into_any_element(),
                EditorButton::plain(&theme, "disabled")
                    .label("Disabled")
                    .disabled(true)
                    .into_any_element(),
                EditorButton::plain(&theme, "filled")
                    .label("Filled")
                    .filled(&theme)
                    .into_any_element(),
                EditorButton::plain(&theme, "accent")
                    .label("Accent")
                    .accent(&theme)
                    .into_any_element(),
                EditorButton::plain(&theme, "accent-off")
                    .label("Accent disabled")
                    .accent(&theme)
                    .disabled(true)
                    .into_any_element(),
                EditorButton::plain(&theme, "destructive")
                    .label("Destructive")
                    .destructive(&theme)
                    .into_any_element(),
                EditorButton::plain(&theme, "icons")
                    .label("Build")
                    .left_icon(Icon::Hammer.path())
                    .right_icon(Icon::ChevronDown.path())
                    .tooltip(&theme, "Build the preview")
                    .into_any_element(),
                Button::plain(
                    &theme,
                    "body-primary",
                    ButtonVariant::Primary,
                    ButtonSize::Sm,
                )
                .label("Body primary")
                .into_any_element(),
                Button::plain(&theme, "body-gray", ButtonVariant::Gray, ButtonSize::Sm)
                    .label("Body gray")
                    .into_any_element(),
                caption(&theme, "hover for the hover state, press Tab for focus"),
            ],
        );

        let icon_buttons = row(
            &theme,
            "IconButton",
            [
                icon_button(&theme, "ib-small", Icon::Play, "Small")
                    .editor_size(IconButtonSize::Small)
                    .into_any_element(),
                icon_button(&theme, "ib-medium", Icon::Pause, "Medium")
                    .editor_size(IconButtonSize::Medium)
                    .into_any_element(),
                icon_button(&theme, "ib-large", Icon::Play, "Large, round, accent")
                    .editor_size(IconButtonSize::Large)
                    .accent(&theme)
                    .rounded(px(999.))
                    .into_any_element(),
                icon_button(&theme, "ib-filled", Icon::Eye, "Filled")
                    .ctl_fill(&theme)
                    .into_any_element(),
                icon_button(&theme, "ib-active", Icon::Layers, "Active")
                    .active(true)
                    .into_any_element(),
                icon_button(&theme, "ib-destructive", Icon::Trash, "Destructive")
                    .destructive(&theme)
                    .into_any_element(),
                icon_button(&theme, "ib-disabled", Icon::Settings, "Disabled")
                    .disabled(true)
                    .into_any_element(),
            ],
        );

        let toggles = row(
            &theme,
            "Toggle",
            [
                Toggle::plain(&theme, "toggle", self.toggle_on)
                    .label("Live toggle")
                    .on_change({
                        let set = update(&this, |gallery, value: &bool, _, _| {
                            gallery.toggle_on = *value
                        });
                        move |value, window, cx| set(&value, window, cx)
                    })
                    .into_any_element(),
                Toggle::plain(&theme, "toggle-off", false).into_any_element(),
                Toggle::plain(&theme, "toggle-disabled", true)
                    .disabled(&theme, true)
                    .into_any_element(),
            ],
        );

        let segment = self.segment;
        let segmented = row(
            &theme,
            "SegmentedControl",
            [
                SegmentedControl::editor(
                    &theme,
                    "segmented",
                    ["Fit", "50%", "100%", "200%"]
                        .iter()
                        .enumerate()
                        .map(|(index, name)| {
                            SegmentOption::new(*name, index == segment).disabled(index == 2)
                        })
                        .collect(),
                )
                .on_select(update(&this, |gallery, index: &usize, _, _| {
                    gallery.segment = *index
                }))
                .into_any_element(),
                caption(
                    &theme,
                    "Left and Right move; the disabled option is skipped",
                ),
            ],
        );

        let select = row(
            &theme,
            "Select and Menu",
            [
                div()
                    .w(px(200.))
                    .child(
                        Select::plain(&theme, "select", RESOLUTIONS[self.resolution])
                            .stretch_label()
                            .on_open(update(
                                &this,
                                |gallery, bounds: &Bounds<Pixels>, window, cx| {
                                    gallery.menu =
                                        Some(MenuState::anchored(*bounds, &gallery.menu_items()));
                                    gallery.menu_focus.focus(window, cx);
                                },
                            )),
                    )
                    .into_any_element(),
                caption(&theme, "Up, Down, Home, End, Enter, Escape"),
            ],
        );

        let slider = row(
            &theme,
            "Slider",
            [
                div()
                    .w(px(220.))
                    .child(
                        Slider::new("slider", self.fraction, SliderTrack::default())
                            .flex()
                            .track(px(4.), colors.ctl_active.into())
                            .fill(colors.accent.into())
                            .thumb(px(14.), colors.thumb.into(), None)
                            .thumb_shadow()
                            .label("Fraction")
                            .on_change(
                                0.,
                                update(&this, |gallery, value: &f32, _, _| {
                                    gallery.fraction = *value
                                }),
                            ),
                    )
                    .into_any_element(),
                div()
                    .w(px(72.))
                    .px(px(7.))
                    .child(
                        Slider::new("zoom", self.zoom, SliderTrack::default())
                            .flex()
                            .track(px(3.), colors.ctl_active.into())
                            .fill(colors.accent.into())
                            .thumb(px(14.), colors.thumb.into(), None)
                            .thumb_shadow()
                            .label("Zoom")
                            .on_change(
                                0.,
                                update(&this, |gallery, value: &f32, _, _| gallery.zoom = *value),
                            ),
                    )
                    .into_any_element(),
                div()
                    .w(px(120.))
                    .child(
                        Slider::new("slider-off", 0.5, SliderTrack::default())
                            .flex()
                            .track(px(4.), colors.ctl_active.into())
                            .fill(colors.accent.into())
                            .thumb(px(14.), colors.thumb.into(), None)
                            .disabled(true),
                    )
                    .into_any_element(),
            ],
        );

        let tab = self.tab;
        let tabs = row(
            &theme,
            "TabRail",
            [div()
                .w(px(240.))
                .child(
                    TabRail::editor(
                        &theme,
                        "tabs",
                        colors.card.into(),
                        [
                            (Icon::Folder, "Project"),
                            (Icon::Image, "Assets"),
                            (Icon::Palette, "Style"),
                        ]
                        .iter()
                        .enumerate()
                        .map(|(index, (glyph, name))| {
                            TabRailItem::new(glyph.path(), *name, index == tab, false)
                        })
                        .collect(),
                    )
                    .on_select(update(&this, |gallery, index: &usize, _, _| {
                        gallery.tab = *index
                    })),
                )
                .into_any_element()],
        );

        let drawer = self.drawer;
        let rail = row(
            &theme,
            "NavRail",
            [div()
                .h(px(180.))
                .child(
                    NavRail::editor(
                        &theme,
                        "rail",
                        [
                            (Icon::Folder, "Project"),
                            (Icon::Image, "Assets"),
                            (Icon::Palette, "Style"),
                        ]
                        .iter()
                        .enumerate()
                        .map(|(index, (glyph, name))| {
                            NavRailItem::new(glyph.path(), *name, drawer == Some(index))
                        })
                        .collect(),
                    )
                    .on_select(update(
                        &this,
                        |gallery, index: &usize, _, _| {
                            gallery.drawer = (gallery.drawer != Some(*index)).then_some(*index)
                        },
                    )),
                )
                .into_any_element()],
        );

        let settings = row(
            &theme,
            "SettingsFrame",
            [div()
                .w(px(560.))
                .h(px(180.))
                .child(
                    SettingsFrame::settings(
                        &theme,
                        "settings-frame",
                        [(Icon::Palette, "Appearance"), (Icon::Package, "Setup")]
                            .iter()
                            .enumerate()
                            .map(|(index, (glyph, name))| {
                                SettingsNavItem::new(*name, *name, *glyph, index == tab)
                            })
                            .collect(),
                    )
                    .on_select(update(&this, |gallery, index: &usize, _, _| {
                        gallery.tab = *index
                    }))
                    .on_back(|_, _| {})
                    .children([Section::settings(
                        &theme,
                        "Appearance",
                        Some("A section of setting rows."),
                        None,
                        vec![
                            Card::settings_rows(
                                &theme,
                                vec![
                                    SettingRow::settings(
                                        &theme,
                                        "Theme",
                                        None,
                                        Toggle::settings(&theme, "settings-row-toggle", true)
                                            .into_any_element(),
                                    )
                                    .into_any_element(),
                                ],
                            )
                            .into_any_element(),
                        ],
                    )
                    .into_any_element()]),
                )
                .into_any_element()],
        );

        let chips = row(
            &theme,
            "KbdChip, Badge, StatusDot",
            [
                KbdChip::row(&theme, "Space").into_any_element(),
                KbdChip::row(&theme, "Ctrl K").into_any_element(),
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_title(&theme, "Field rows"))
                    .child(Badge::new("3")),
            )
            .child(field_row(&theme, "Width", label(&theme, "1920 px")))
            .child(field_row(
                &theme,
                "Hardware",
                Toggle::plain(&theme, "field-toggle", true),
            ))
            .child(muted(&theme, "Uses the GPU encoder when available"))
            .child(field_row(
                &theme,
                "Quality",
                div().w(px(160.)).child(
                    Slider::new("field-slider", 0.7, SliderTrack::default())
                        .flex()
                        .track(px(4.), colors.ctl_active.into())
                        .fill(colors.accent.into())
                        .thumb(px(14.), colors.thumb.into(), None),
                ),
            ));

        let collapsible = div().w(px(360.)).child(
            CollapsibleSection::new("collapsible", self.collapsible_open)
                .header(section_title(&theme, "Advanced"))
                .content(
                    div()
                        .pt(px(METRICS.space_2))
                        .child(muted(&theme, "Content that slides in over 200 ms.")),
                )
                .on_toggle(update(&this, |gallery, open: &bool, _, _| {
                    gallery.collapsible_open = *open
                })),
        );

        let notices = div()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .w(px(480.))
            .child(section_title(&theme, "Notice"))
            .child(
                Notice::new("n-info", Tone::Info, "Preview is building")
                    .body("This takes a few seconds."),
            )
            .child(Notice::new("n-success", Tone::Success, "Saved").body("Checkpoint written."))
            .child(
                Notice::new("n-warning", Tone::Warning, "Preview is out of date")
                    .body("The source changed after the last build.")
                    .action(
                        EditorButton::plain(&theme, "n-build")
                            .label("Build preview")
                            .accent(&theme),
                    ),
            )
            .children(self.notice_visible.then(|| {
                Notice::new("n-danger", Tone::Danger, "Build failed")
                    .body("The project did not compile.")
                    .details(
                        "error[E0425]: cannot find value `x` in this scope\n  --> src/lib.rs:12:5",
                    )
                    .action(EditorButton::plain(&theme, "n-retry").label("Retry"))
                    .on_dismiss(update0(&this, |gallery, _, _| {
                        gallery.notice_visible = false
                    }))
            }));

        let inputs = div()
            .w(px(360.))
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .child(section_title(&theme, "TextInput"))
            .child(TextInput::plain(&theme, "gallery-input", &self.input))
            .child(TextInput::plain(
                &theme,
                "gallery-compact-input",
                &self.compact_input,
            ));

        let focus_sample = row(
            &theme,
            "Focus ring (static sample)",
            [div()
                .rounded(px(METRICS.radius_control))
                .shadow(focus_ring(&theme))
                .child(EditorButton::plain(&theme, "ring").label("Focused"))
                .into_any_element()],
        );

        let content = div()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_6))
            .p(px(METRICS.space_6))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(heading(&theme, "fframes Studio UI gallery"))
                    .child(theme_switch),
            )
            .child(
                Card::editor(&theme)
                    .p(px(METRICS.space_4))
                    .flex()
                    .flex_col()
                    .gap(px(METRICS.space_4))
                    .child(buttons)
                    .child(icon_buttons)
                    .child(toggles)
                    .child(segmented)
                    .child(select)
                    .child(slider)
                    .child(tabs)
                    .child(rail)
                    .child(settings)
                    .child(chips)
                    .child(focus_sample),
            )
            .child(
                Card::editor(&theme)
                    .p(px(METRICS.space_4))
                    .flex()
                    .flex_wrap()
                    .gap(px(METRICS.space_6))
                    .child(fields)
                    .child(collapsible)
                    .child(inputs)
                    .child(notices),
            );

        let menu = self.menu.as_ref().map(|state| {
            Menu::editor(&theme, "menu", self.menu_items(), state)
                .focus(&self.menu_focus)
                .on_highlight(update(&this, |gallery, index: &usize, _, _| {
                    if let Some(menu) = gallery.menu.as_mut() {
                        menu.set_highlighted(*index)
                    }
                }))
                .on_select(update(&this, |gallery, index: &usize, _, _| {
                    gallery.resolution = *index;
                    gallery.menu = None;
                }))
                .on_dismiss(update0(&this, |gallery, _, _| gallery.menu = None))
        });

        div()
            .id("gallery")
            .size_full()
            .overflow_y_scroll()
            .bg(colors.window)
            .text_color(colors.text_1)
            .child(content)
            .children(menu)
    }
}

fn main() {
    gpui_platform::application()
        .with_assets(fframes_studio::assets::Assets)
        .run(|cx: &mut App| {
            bind_text_input_keys(cx);
            studio_theme::init(cx, AppTheme::System);
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
