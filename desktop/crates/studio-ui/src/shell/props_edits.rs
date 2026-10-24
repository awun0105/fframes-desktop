//! Props edits on their way to the engine, and the selection the props panel follows.
//! Like timeline edits they wait here in order while a command runs: a slider drag sends
//! a value per pointer move, and a value replaces a queued value of the same field.

use gpui::Context;
use studio_engine::engine::backend::Command;
use studio_engine::props::PropsCommand;

use super::StudioShell;

impl StudioShell {
    /// Queues `command` and sends it when the engine is free.
    pub(super) fn props_edit(&mut self, command: PropsCommand, cx: &mut Context<Self>) {
        if replaces(&command, self.props_queue.back()) {
            self.props_queue.pop_back();
        }
        self.props_queue.push_back(command);
        self.flush_props_edits(cx);
    }

    /// Sends the oldest queued edit when no command is running.
    pub(super) fn flush_props_edits(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(command) = self.props_queue.pop_front() else {
            return;
        };
        self.dispatch(Command::Props(command.clone()), cx);
        self.props_in_flight = self.in_flight.map(|(id, _)| (id, command));
    }

    /// The engine answered the edit it was sent. A refused `BeginGesture` never opened its
    /// gesture, so the values of that drag are dropped: outside a gesture each would be
    /// written and undone on its own.
    pub(super) fn props_edit_finished(&mut self, command: PropsCommand, refused: bool) {
        if command != PropsCommand::BeginGesture || !refused {
            return;
        }
        match self
            .props_queue
            .iter()
            .position(|queued| *queued == PropsCommand::EndGesture)
        {
            Some(end) => {
                self.props_queue.drain(..=end);
            }
            None => self
                .props_queue
                .retain(|queued| !matches!(queued, PropsCommand::Set { .. })),
        }
    }

    /// Hands a changed canvas selection to the props panel and asks the preview for its
    /// props.
    pub(super) fn sync_props_selection(&mut self, cx: &mut Context<Self>) {
        let selected = self.canvas.selection_identity().cloned();
        if self.props_selection == selected {
            return;
        }
        self.props_selection = selected.clone();
        if let Some(preview) = &self.preview {
            preview.want_props_info(selected.clone());
        }
        self.props_panel
            .update(cx, |panel, cx| panel.set_selection(selected, cx));
    }
}

/// A value replaces the queued value of the same field of the same object.
fn replaces(command: &PropsCommand, queued: Option<&PropsCommand>) -> bool {
    match (command, queued) {
        (
            PropsCommand::Set {
                identity, field, ..
            },
            Some(PropsCommand::Set {
                identity: queued_identity,
                field: queued_field,
                ..
            }),
        ) => identity == queued_identity && field == queued_field,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fframes_studio_protocol::EditorObjectIdentity;
    use serde_json::json;

    fn set(object: &str, field: &str, value: f64) -> PropsCommand {
        PropsCommand::Set {
            identity: EditorObjectIdentity {
                scene_instance_key: "intro".into(),
                component_key: "Title".into(),
                object_key: object.into(),
                repeat_key: "main".into(),
            },
            field: field.into(),
            value: Some(json!(value)),
        }
    }

    #[test]
    fn a_value_replaces_only_a_queued_value_of_the_same_field() {
        assert!(replaces(&set("a", "size", 2.), Some(&set("a", "size", 1.))));
        assert!(!replaces(&set("a", "size", 2.), Some(&set("a", "gap", 1.))));
        assert!(!replaces(
            &set("a", "size", 2.),
            Some(&set("b", "size", 1.))
        ));
        assert!(!replaces(
            &set("a", "size", 2.),
            Some(&PropsCommand::BeginGesture)
        ));
        assert!(!replaces(&PropsCommand::Undo, Some(&PropsCommand::Undo)));
        assert!(!replaces(&set("a", "size", 2.), None));
    }
}
