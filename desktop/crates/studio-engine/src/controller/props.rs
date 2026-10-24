//! Props edits of the open project: the props panel's field changes, undo and redo, and
//! the write of `props.json` at the end of each gesture.
//!
//! Like a timing edit, a props edit is not a source transaction: it writes no journal
//! record and never invalidates the build (the build revision leaves the props document
//! out). It has the same gates: an agent task, a blocked publication gate or an
//! unresolved earlier mutation refuses it.
use super::Controller;
use crate::{
    EngineError,
    edit_transaction::{ApplyGate, Boundary, Fault, PromotionError},
    props::{PROPS, PropsCommand, PropsEditing, PropsSnapshot},
    timing::file::{self, WriteError},
};
use fframes_studio_protocol::PropsDocument;
use studio_project::revision::PROPS_FILE;

/// Shown when the live file changed under the session.
const CHANGED_OUTSIDE: &str =
    "props.json changed outside the studio; the props were reloaded and their undo history cleared";
/// Shown when an agent edit published a new document.
const PUBLISHED_BY_AGENT: &str = "An agent edit published a new props.json; the props were reloaded and their undo history cleared";

impl Controller {
    /// Whether the props can be edited now, with the reason when they cannot.
    pub fn props_mutation(&mut self) -> PropsEditing {
        if let Err(error) = self.ensure_mutable() {
            return PropsEditing::Disabled(error.to_string());
        }
        if let ApplyGate::Blocked(reason) = self.apply_gate() {
            return PropsEditing::Disabled(format!("Source publication is blocked: {reason}"));
        }
        if self.agent.active().is_some() {
            return PropsEditing::Disabled(
                "An agent task is running; finish or cancel it before editing props".into(),
            );
        }
        match self.props.doc() {
            Ok(_) => PropsEditing::Enabled,
            Err(error) => {
                PropsEditing::Disabled(format!("{error}; fix or remove {PROPS_FILE} to edit props"))
            }
        }
    }

    fn require_props_mutation(&mut self) -> Result<(), EngineError> {
        match self.props_mutation() {
            PropsEditing::Enabled => Ok(()),
            PropsEditing::Disabled(reason) => Err(EngineError::Diagnostic(reason)),
        }
    }

    pub fn props_snapshot(&mut self) -> PropsSnapshot {
        PropsSnapshot {
            generation: self.props.generation(),
            props_json: self.props.props_json(),
            doc: self.props.doc().ok().cloned(),
            can_undo: self.props.history.can_undo() && !self.props.history.in_gesture(),
            can_redo: self.props.history.can_redo() && !self.props.history.in_gesture(),
            editing: self.props_mutation(),
            notice: self.props.notice().map(str::to_owned),
        }
    }

    /// The props document and its generation, for a preview.
    pub fn props_for_preview(&self) -> (Option<String>, u64) {
        (self.props.props_json(), self.props.generation())
    }

    /// The document an export draws: `None` (the code's values) unless it has objects.
    /// An invalid document refuses the export with its reason.
    pub fn props_for_export(&self) -> Result<Option<String>, String> {
        let doc = self
            .props
            .doc()
            .map_err(|error| format!("{error}; fix or remove {PROPS_FILE} to export"))?;
        Ok((!doc.objects.is_empty()).then(|| doc.to_json()))
    }

    /// Applies one props command. Edits outside a gesture are written at once; inside one
    /// they only preview, and the gesture's end writes them as one undo step.
    pub fn props_command(&mut self, command: PropsCommand) -> Result<(), EngineError> {
        match command {
            PropsCommand::Set {
                identity,
                field,
                value,
            } => {
                self.require_props_mutation()?;
                if self.props.history.gesture_aborted() {
                    return Err(EngineError::Diagnostic(CHANGED_OUTSIDE.into()));
                }
                let doc = self.props.doc().map_err(EngineError::Diagnostic)?.clone();
                let mut next = doc.clone();
                next.set(&identity, &field, value);
                next.validate()
                    .map_err(|error| EngineError::Diagnostic(format!("{PROPS_FILE}: {error}")))?;
                if next.to_json().len() > fframes_studio_protocol::MAX_PROPS_DOC_BYTES {
                    return Err(EngineError::Diagnostic(format!(
                        "{PROPS_FILE} would exceed {} bytes",
                        fframes_studio_protocol::MAX_PROPS_DOC_BYTES
                    )));
                }
                if next == doc {
                    return Ok(());
                }
                self.props.history.record(&doc);
                self.props.set_doc(next);
                if self.props.history.in_gesture() {
                    Ok(())
                } else {
                    self.write_props()
                }
            }
            PropsCommand::BeginGesture => {
                self.require_props_mutation()?;
                let doc = self.props.doc().map_err(EngineError::Diagnostic)?.clone();
                self.props.history.begin_gesture(&doc);
                Ok(())
            }
            PropsCommand::EndGesture => self.end_props_gesture(),
            PropsCommand::Undo | PropsCommand::Redo => {
                self.require_props_mutation()?;
                if self.props.history.in_gesture() {
                    return Err(EngineError::Diagnostic(
                        "Finish the drag before undoing a props edit".into(),
                    ));
                }
                let current = self.props.doc().map_err(EngineError::Diagnostic)?.clone();
                let next = if command == PropsCommand::Undo {
                    self.props.history.undo(&current)
                } else {
                    self.props.history.redo(&current)
                };
                let Some(next) = next else {
                    return Err(EngineError::Diagnostic(
                        match (command, self.props.notice()) {
                            (_, Some(notice)) => notice.to_owned(),
                            (PropsCommand::Undo, None) => "There is no props edit to undo".into(),
                            _ => "There is no props edit to redo".into(),
                        },
                    ));
                };
                self.props.set_doc(next);
                self.write_props()
            }
        }
    }

    fn end_props_gesture(&mut self) -> Result<(), EngineError> {
        if !self.props.history.in_gesture() {
            return Ok(());
        }
        if let PropsEditing::Disabled(reason) = self.props_mutation() {
            // Nothing of the gesture is written: the file stands, and the session goes
            // back to it.
            let root = self.project.root.clone();
            self.props.history.end_gesture(&PropsDocument::default());
            self.props.reload(&root, reason.clone());
            return Err(EngineError::Diagnostic(reason));
        }
        let doc = self.props.doc().map_err(EngineError::Diagnostic)?.clone();
        if self.props.history.end_gesture(&doc) {
            self.write_props()
        } else {
            Ok(())
        }
    }

    /// Writes the session's document to `props.json` and rescans; the rescan sees a
    /// props-only change, which keeps the build.
    fn write_props(&mut self) -> Result<(), EngineError> {
        let json = self.props.doc().map_err(EngineError::Diagnostic)?.to_json();
        let root = self.project.root.clone();
        let hooks = self.hooks.clone();
        let mut crashed = false;
        let written =
            file::write_doc(
                &root,
                &PROPS,
                &json,
                self.props.known(),
                &mut |step| match hooks.at(&Boundary::PropsWrite(step)) {
                    Ok(()) => Ok(()),
                    Err(Fault::Crash) => {
                        crashed = true;
                        Err(std::io::Error::other("interrupted at an injected boundary"))
                    }
                    Err(Fault::Io(kind)) => Err(std::io::Error::new(kind, "injected")),
                },
            );
        match written {
            Ok(known) => {
                self.props.set_known(known);
                self.reconcile()
            }
            // The process "died" here: nothing is repaired until the project reopens.
            Err(_) if crashed => {
                self.recovery_status.suspended = Some("interrupted at an injected boundary".into());
                Err(PromotionError::Crashed.into())
            }
            Err(error) => {
                if matches!(error, WriteError::Io(_)) {
                    // An interrupted write may have moved the live file aside.
                    let _ = file::recover_doc(&root, &PROPS);
                }
                let reason = error.to_string();
                self.props.reload(&root, reason.clone());
                let _ = self.reconcile();
                Err(EngineError::Diagnostic(reason))
            }
        }
    }

    /// Reloads the document when the scan found a `props.json` the session did not write;
    /// its own writes are recognised by their hash.
    pub(super) fn observe_props_file(&mut self) {
        let live = self
            .project
            .inventory
            .props_file()
            .map(|file| file.sha256.clone());
        if !self.props.is_known(live.as_deref()) {
            let state = self.state();
            let published = state
                .task_history()
                .head()
                .is_some_and(|record| record.published == *state.source());
            let reason = if published {
                PUBLISHED_BY_AGENT
            } else {
                CHANGED_OUTSIDE
            };
            let root = self.project.root.clone();
            self.props.reload(&root, reason.into());
        }
    }
}
