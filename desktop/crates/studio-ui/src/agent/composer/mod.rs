//! The agent prompt composer: a multiline input, slash command completion,
//! attachment chips and the send, steer and stop controls.
//!
//! The component owns what the user is typing and nothing else. It reads no
//! file, calls no agent and decides no policy. Every action leaves as a
//! [`ComposerEvent`] for its owner, and the owner pushes state back in:
//! the command list, whether a turn is running, how the agent takes a message
//! mid-turn ([`SteerMode`]), and the chips it accepted.
//!
//! ## Confinement is the owner's job
//!
//! Pressing attach, pasting an image or pasting copied files emits
//! [`ComposerEvent::AttachRequested`]. The component checks nothing on disk. The
//! owner decides whether a path is inside the project, whether the deny list
//! applies, and whether an image is within the size cap, then calls
//! [`Composer::add_attachment`] with what it accepted. A chip is a label and
//! an optional thumbnail; it carries no path.
//!
//! The owner calls [`keymap::init`] once at start so the input's key bindings
//! exist.

pub mod attachments;
mod input;
pub mod keymap;
pub mod metrics;
pub mod slash;
pub mod steer;
#[cfg(test)]
mod tests;

use std::{path::PathBuf, rc::Rc, sync::Arc};

use gpui::{
    App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, div, prelude::*, px,
};
use studio_theme::Theme;

pub use attachments::{Attachment, AttachmentDraft, AttachmentId, AttachmentKind, Attachments};
pub use input::{ComposerInput, ComposerInputEvent};
pub use slash::SlashCommand;
pub use steer::{SendAction, SteerMode};

use crate::kit::{Button, ButtonSize, ButtonVariant, Icon, IconButton};
use slash::SlashState;
use steer::steer_actions;

/// What the user wants to send: the text and the chips that went with it.
#[derive(Clone)]
pub struct Submission {
    pub text: String,
    pub attachments: Vec<Attachment>,
}

/// What the user asked to attach. Nothing has been read or checked.
#[derive(Clone)]
pub enum AttachRequest {
    /// The attach-file button: open a file picker.
    PickFiles,
    /// The attach-image button: open an image picker.
    PickImages,
    /// Files copied in a file manager and pasted.
    Paths(Vec<PathBuf>),
    /// Image data on the clipboard, such as a screenshot.
    Images(Vec<Arc<gpui::Image>>),
}

/// The kinds of attachment the owner can switch off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachKind {
    File,
    Image,
}

/// What the composer tells its owner.
#[derive(Clone)]
pub enum ComposerEvent {
    /// Send, with no turn running. The draft is already cleared; call
    /// [`Composer::restore`] if the send is refused.
    Submit(Submission),
    /// Send while a turn runs, to be delivered at the next boundary.
    SendAtBoundary(Submission),
    /// Send while a turn runs, to be handed to the running turn.
    Steer(Submission),
    /// Cancel the running turn, then send.
    SendNow(Submission),
    /// The stop button.
    Stop,
    AttachRequested(AttachRequest),
    /// A chip's remove button was pressed and the chip is gone.
    AttachmentRemoved(AttachmentId),
}

/// The composer view.
pub struct Composer {
    theme: Theme,
    input: Entity<ComposerInput>,
    commands: Vec<SlashCommand>,
    slash: SlashState,
    attachments: Attachments,
    steer_mode: SteerMode,
    running: bool,
    /// Why each kind of attachment is off, when it is.
    file_disabled: Option<SharedString>,
    image_disabled: Option<SharedString>,
    _input_events: Subscription,
}

impl EventEmitter<ComposerEvent> for Composer {}

impl Focusable for Composer {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

impl Composer {
    pub fn new(placeholder: impl Into<SharedString>, theme: Theme, cx: &mut Context<Self>) -> Self {
        let placeholder = placeholder.into();
        let input = cx.new(|cx| ComposerInput::new(placeholder, theme, cx));
        let subscription = cx.subscribe(&input, |this, _, event: &ComposerInputEvent, cx| {
            this.on_input_event(event, cx);
        });
        Self {
            theme,
            input,
            commands: Vec::new(),
            slash: SlashState::default(),
            attachments: Attachments::default(),
            steer_mode: SteerMode::None,
            running: false,
            file_disabled: None,
            image_disabled: None,
            _input_events: subscription,
        }
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        self.input
            .update(cx, |input, cx| input.set_theme(theme, cx));
        cx.notify();
    }

    /// The commands the agent advertises, shown when the user types `/`.
    pub fn set_commands(&mut self, commands: Vec<SlashCommand>, cx: &mut Context<Self>) {
        self.commands = commands;
        self.refresh_slash(cx);
    }

    /// Whether a turn is running. While it is, sending follows the steer mode
    /// and the stop button shows.
    pub fn set_running(&mut self, running: bool, cx: &mut Context<Self>) {
        if self.running != running {
            self.running = running;
            cx.notify();
        }
    }

    pub fn set_steer_mode(&mut self, mode: SteerMode, cx: &mut Context<Self>) {
        if self.steer_mode != mode {
            self.steer_mode = mode;
            cx.notify();
        }
    }

    /// Switches an attachment kind off with the reason to show, or back on
    /// with `None`. A disabled kind neither shows an enabled button nor emits
    /// [`ComposerEvent::AttachRequested`] from a paste.
    pub fn set_attach_disabled(
        &mut self,
        kind: AttachKind,
        reason: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let slot = match kind {
            AttachKind::File => &mut self.file_disabled,
            AttachKind::Image => &mut self.image_disabled,
        };
        if *slot != reason {
            *slot = reason;
            cx.notify();
        }
    }

    /// Shows a chip the owner accepted.
    pub fn add_attachment(
        &mut self,
        draft: AttachmentDraft,
        cx: &mut Context<Self>,
    ) -> AttachmentId {
        let id = self.attachments.add(draft);
        cx.notify();
        id
    }

    /// Removes a chip without telling the owner, as when the owner drops it.
    pub fn remove_attachment(&mut self, id: AttachmentId, cx: &mut Context<Self>) -> bool {
        let removed = self.attachments.remove(id);
        if removed {
            cx.notify();
        }
        removed
    }

    pub fn attachments(&self) -> &[Attachment] {
        self.attachments.items()
    }

    /// The draft text.
    pub fn text(&self, cx: &App) -> String {
        self.input.read(cx).text().to_string()
    }

    /// Replaces the draft text. Attachments stay.
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = text.into();
        self.input.update(cx, |input, cx| input.set_text(text, cx));
    }

    /// Puts a refused submission back: its text and chips under their old ids.
    pub fn restore(&mut self, submission: Submission, cx: &mut Context<Self>) {
        self.attachments.restore(submission.attachments);
        self.set_text(submission.text, cx);
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.input.focus_handle(cx), cx);
    }

    fn on_input_event(&mut self, event: &ComposerInputEvent, cx: &mut Context<Self>) {
        match event {
            ComposerInputEvent::Submitted => {
                let actions = steer_actions(self.steer_mode, self.running);
                if let Some(action) = actions.primary {
                    self.trigger(action, cx);
                }
            }
            ComposerInputEvent::SubmittedNow => {
                let action = steer_actions(self.steer_mode, self.running).now;
                self.trigger(action, cx);
            }
            ComposerInputEvent::Edited | ComposerInputEvent::CursorMoved => {
                self.refresh_slash(cx);
                cx.notify();
            }
            ComposerInputEvent::ViewportChanged => cx.notify(),
            ComposerInputEvent::CompletionNavigate(delta) => {
                self.slash.navigate(*delta);
                self.sync_completion_controls(cx);
                cx.notify();
            }
            ComposerInputEvent::CompletionAccept => self.accept_selected(cx),
            ComposerInputEvent::CompletionDismiss => {
                let text = self.text(cx);
                self.slash.dismiss(&text);
                self.sync_completion_controls(cx);
                cx.notify();
            }
            ComposerInputEvent::PastedImages(images) => {
                if self.image_disabled.is_none() {
                    let images = images.iter().cloned().map(Arc::new).collect();
                    cx.emit(ComposerEvent::AttachRequested(AttachRequest::Images(
                        images,
                    )));
                }
            }
            ComposerInputEvent::PastedPaths(paths) => {
                if self.file_disabled.is_none() {
                    cx.emit(ComposerEvent::AttachRequested(AttachRequest::Paths(
                        paths.clone(),
                    )));
                }
            }
        }
    }

    /// Whether there is anything to send.
    fn can_send(&self, cx: &App) -> bool {
        !self.input.read(cx).text().trim().is_empty() || !self.attachments.is_empty()
    }

    /// Sends the draft as `action`. Does nothing for an empty draft.
    fn trigger(&mut self, action: SendAction, cx: &mut Context<Self>) {
        if !self.can_send(cx) {
            return;
        }
        let text = self.text(cx).trim().to_string();
        self.input.update(cx, |input, cx| input.set_text("", cx));
        let submission = Submission {
            text,
            attachments: self.attachments.take(),
        };
        cx.emit(match action {
            SendAction::Submit => ComposerEvent::Submit(submission),
            SendAction::SendAtBoundary => ComposerEvent::SendAtBoundary(submission),
            SendAction::Steer => ComposerEvent::Steer(submission),
            SendAction::SendNow => ComposerEvent::SendNow(submission),
        });
        cx.notify();
    }

    fn refresh_slash(&mut self, cx: &mut Context<Self>) {
        let (text, cursor) = {
            let input = self.input.read(cx);
            (input.text().to_string(), input.cursor_offset())
        };
        self.slash.refresh(&text, cursor, &self.commands);
        self.sync_completion_controls(cx);
    }

    /// Tells the input whether the menu is open, so Enter, Tab and the arrows
    /// go to it.
    fn sync_completion_controls(&mut self, cx: &mut Context<Self>) {
        let open = self.slash.is_open();
        let selected = open && self.slash.selected_command().is_some();
        self.input.update(cx, |input, cx| {
            input.set_completion_controls(open, selected, cx)
        });
    }

    fn accept_selected(&mut self, cx: &mut Context<Self>) {
        let Some(command) = self.slash.selected_command() else {
            return;
        };
        let text = self.text(cx);
        let Some((range, replacement)) = self.slash.insertion(&text, &self.commands, command)
        else {
            return;
        };
        self.input.update(cx, |input, cx| {
            input.replace_range_as_step(range, &replacement, cx)
        });
    }

    fn remove_from_strip(&mut self, id: AttachmentId, cx: &mut Context<Self>) {
        if self.attachments.remove(id) {
            cx.emit(ComposerEvent::AttachmentRemoved(id));
            cx.notify();
        }
    }

    /// The stop button. Only meaningful while a turn runs.
    fn stop(&mut self, cx: &mut Context<Self>) {
        if self.running {
            cx.emit(ComposerEvent::Stop);
        }
    }

    fn request_attach(&mut self, request: AttachRequest, cx: &mut Context<Self>) {
        cx.emit(ComposerEvent::AttachRequested(request));
    }

    fn slash_menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.slash.is_open() {
            return None;
        }
        let theme = self.theme;
        let active = self.slash.active_row();
        let rows = self
            .slash
            .rows()
            .iter()
            .enumerate()
            .filter_map(|(row, index)| self.commands.get(*index).map(|command| (row, command)))
            .map(|(row, command)| {
                div()
                    .id(("slash-row", row))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(10.0))
                    .py(px(5.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .when(active == Some(row), |row| row.bg(theme.gray(4)))
                    .hover(|style| style.bg(theme.gray(4)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.slash.set_active_row(row);
                        this.accept_selected(cx);
                    }))
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(13.0))
                            .text_color(theme.gray(12))
                            .child(format!("/{}", command.name)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(theme.gray(10))
                            .child(command.description.clone()),
                    )
            })
            .collect::<Vec<_>>();
        Some(
            div()
                .id("composer-slash-menu")
                .absolute()
                .left_0()
                .right_0()
                .bottom_full()
                .mb(px(6.0))
                .max_h(px(240.0))
                .overflow_y_scroll()
                .p(px(4.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(theme.gray(6))
                .bg(theme.gray(2))
                .children(rows),
        )
    }

    fn controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let actions = steer_actions(self.steer_mode, self.running);
        let sendable = self.can_send(cx);
        let file_button = IconButton::body(&theme, "composer-attach-file", Icon::File.path())
            .size(px(24.0))
            .icon_size(px(14.0))
            .rounded(px(6.0))
            .label("Attach file")
            .tooltip(
                &theme,
                self.file_disabled
                    .clone()
                    .unwrap_or_else(|| "Attach file".into()),
            )
            .disabled(self.file_disabled.is_some())
            .on_click(
                cx.listener(|this, _, _, cx| this.request_attach(AttachRequest::PickFiles, cx)),
            );
        let image_button = IconButton::body(&theme, "composer-attach-image", Icon::Image.path())
            .size(px(24.0))
            .icon_size(px(14.0))
            .rounded(px(6.0))
            .label("Attach image")
            .tooltip(
                &theme,
                self.image_disabled
                    .clone()
                    .unwrap_or_else(|| "Attach image".into()),
            )
            .disabled(self.image_disabled.is_some())
            .on_click(
                cx.listener(|this, _, _, cx| this.request_attach(AttachRequest::PickImages, cx)),
            );

        let mut right = div().flex().flex_row().items_center().gap(px(6.0));
        if self.running {
            right = right.child(
                IconButton::body(&theme, "composer-stop", Icon::Square.path())
                    .size(px(24.0))
                    .icon_size(px(12.0))
                    .rounded(px(12.0))
                    .label("Stop")
                    .tooltip(&theme, "Stop")
                    .on_click(cx.listener(|this, _, _, cx| this.stop(cx))),
            );
        }
        if let Some(hint) = actions.hint {
            right = right.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.gray(10))
                    .child(hint),
            );
        }
        if self.running {
            let now = actions.now;
            right = right.child(
                Button::plain(
                    &theme,
                    "composer-send-now",
                    ButtonVariant::Outline,
                    ButtonSize::Sm,
                )
                .label(now.label())
                .disabled(!sendable)
                .on_click(cx.listener(move |this, _, _, cx| this.trigger(now, cx))),
            );
        }
        if let Some(primary) = actions.primary {
            right = right.child(
                Button::plain(
                    &theme,
                    "composer-send",
                    ButtonVariant::Primary,
                    ButtonSize::Sm,
                )
                .label(primary.label())
                .disabled(!sendable)
                .on_click(cx.listener(move |this, _, _, cx| this.trigger(primary, cx))),
            );
        }

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(8.0))
            .pb(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(2.0))
                    .child(file_button)
                    .child(image_button),
            )
            .child(right)
    }
}

impl Render for Composer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let weak = cx.entity().downgrade();
        let on_remove: attachments::RemoveHandler = Rc::new(move |id, _, cx| {
            weak.update(cx, |this, cx| this.remove_from_strip(id, cx))
                .ok();
        });
        div()
            .id("composer")
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .rounded(px(12.0))
            .border_1()
            .border_color(theme.gray(6))
            .bg(theme.gray(2))
            .children(self.slash_menu(cx))
            .children(attachments::strip(&self.attachments, &theme, on_remove))
            .child(div().px(px(14.0)).py(px(10.0)).child(self.input.clone()))
            .child(self.controls(cx))
    }
}
