//! Timeline edits on their way to the engine. The engine runs one command at a time and
//! the shell drops what arrives while one runs, so edits wait here in order: a drag sends
//! a trim per pointer move, and a trim replaces a queued trim of the same clip edge.

use gpui::{
    Context, DispatchPhase, IntoElement, MouseButton, MouseMoveEvent, MouseUpEvent, Styled, canvas,
};
use studio_engine::engine::backend::Command;
use studio_engine::timing::TimelineCommand;

use super::StudioShell;

impl StudioShell {
    /// Queues `command` and sends it when the engine is free.
    pub(super) fn timeline_edit(&mut self, command: TimelineCommand, cx: &mut Context<Self>) {
        let replaces = match (&command, self.timeline_queue.back()) {
            (
                TimelineCommand::Trim { clip, edge, .. },
                Some(TimelineCommand::Trim {
                    clip: queued_clip,
                    edge: queued_edge,
                    ..
                }),
            ) => clip == queued_clip && edge == queued_edge,
            _ => false,
        };
        if replaces {
            self.timeline_queue.pop_back();
        }
        self.timeline_queue.push_back(command);
        self.flush_timeline_edits(cx);
    }

    /// Sends the oldest queued edit when no command is running.
    pub(super) fn flush_timeline_edits(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(command) = self.timeline_queue.pop_front() else {
            return;
        };
        self.dispatch(Command::Timeline(command.clone()), cx);
        self.timeline_in_flight = self.in_flight.map(|(id, _)| (id, command));
    }

    /// The engine answered the edit it was sent. A refused `BeginGesture` never opened its
    /// gesture, so the gesture's trims are dropped (outside a gesture each would be
    /// written and undone on its own) and a drag still under way is let go.
    pub(super) fn timeline_edit_finished(
        &mut self,
        command: TimelineCommand,
        refused: bool,
        cx: &mut Context<Self>,
    ) {
        if command != TimelineCommand::BeginGesture || !refused {
            return;
        }
        match self
            .timeline_queue
            .iter()
            .position(|queued| *queued == TimelineCommand::EndGesture)
        {
            Some(end) => {
                self.timeline_queue.drain(..=end);
            }
            None => {
                self.timeline_queue
                    .retain(|queued| !matches!(queued, TimelineCommand::Trim { .. }));
                self.timeline
                    .update(cx, |timeline, cx| timeline.cancel_edit_drag(cx));
            }
        }
    }

    /// Undo or redo from the keyboard, sent only when the engine has a step to take or
    /// edits are still on their way to it, so a stray Ctrl+Z says nothing.
    pub(super) fn timeline_history(&mut self, command: TimelineCommand, cx: &mut Context<Self>) {
        let step = self.presentation.timeline.as_ref().is_some_and(|timeline| {
            if command == TimelineCommand::Undo {
                timeline.can_undo
            } else {
                timeline.can_redo
            }
        });
        let pending = !self.timeline_queue.is_empty() || self.timeline_in_flight.is_some();
        if !self.timeline.read(cx).edit_dragging() && (step || pending) {
            self.timeline_edit(command, cx);
        }
    }

    /// While a clip is dragged, follows the pointer through the whole window, including
    /// past its edges, where element listeners stop firing.
    pub(super) fn render_edit_tracking(&self) -> gpui::AnyElement {
        let timeline = self.timeline.clone();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moved = timeline.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    moved.update(cx, |timeline, cx| {
                        // A release outside the window never arrives; a move without the
                        // button ends the drag instead.
                        if event.pressed_button == Some(MouseButton::Left) {
                            let x = f32::from(event.position.x);
                            timeline.edit_drag_move(x, event.modifiers.shift, cx);
                        } else {
                            timeline.end_edit_drag(cx);
                        }
                    });
                });
                let released = timeline.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        released.update(cx, |timeline, cx| timeline.end_edit_drag(cx));
                    }
                });
            },
        )
        .absolute()
        .size_0()
        .into_any_element()
    }
}
