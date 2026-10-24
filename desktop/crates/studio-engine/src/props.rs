//! The project's props document (`props.json`) as an open project edits it: the values the
//! props panel gives editable components, with snapshot undo and redo and the write at
//! the end of a gesture that never touches the build. It works like the timing document
//! ([`crate::timing`]) and shares its file and history code.
//!
//! The session lives on the [`crate::Controller`], one per open project.
use std::path::Path;

use fframes_studio_protocol::{EditorObjectIdentity, MAX_PROPS_DOC_BYTES, PropsDocument};
use serde_json::Value;
use studio_project::revision::PROPS_FILE;

use crate::timing::{
    file::{self, DocFile, KnownFile},
    history::History,
};

/// Where staged and displaced copies of `props.json` live, relative to the project root.
pub const PROPS_CACHE: &str = ".fframes/cache/props";

/// `props.json`.
pub const PROPS: DocFile = DocFile {
    name: PROPS_FILE,
    cache: PROPS_CACHE,
    max_bytes: MAX_PROPS_DOC_BYTES,
    view: "the props",
    valid: |json| PropsDocument::parse(json).is_ok(),
};

/// An edit of the props document.
#[derive(Debug, Clone, PartialEq)]
pub enum PropsCommand {
    /// Sets one field of an object; `None` goes back to the code's value.
    Set {
        identity: EditorObjectIdentity,
        field: String,
        value: Option<Value>,
    },
    /// Starts a drag: its edits preview live and become one undo step and one write.
    BeginGesture,
    EndGesture,
    Undo,
    Redo,
}

/// Whether Studio may change the project's props (`props.json`) now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsEditing {
    Enabled,
    /// Disabled with the reason to show next to the controls.
    Disabled(String),
}

/// What the props panel shows and what the preview draws.
#[derive(Debug, Clone, PartialEq)]
pub struct PropsSnapshot {
    /// Changes with every change of the document the preview should draw.
    pub generation: u64,
    /// The document for `SetProps`; `None` draws the code's values.
    pub props_json: Option<String>,
    /// The document edits start from; `None` when the live file is invalid.
    pub doc: Option<PropsDocument>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub editing: PropsEditing,
    /// Why the document was reloaded or could not be read, until the next edit.
    pub notice: Option<String>,
}

/// The props document of the open project and its edit history.
#[derive(Debug)]
pub struct PropsSession {
    /// The document edits start from; `Err` when the live file cannot be read as one.
    doc: Result<PropsDocument, String>,
    /// The live file as last loaded or written (`None`: the project has none).
    known: Option<KnownFile>,
    generation: u64,
    pub(crate) history: History<PropsDocument>,
    notice: Option<String>,
}

impl PropsSession {
    /// Loads the live file of the project at `root`.
    pub fn load(root: &Path) -> Self {
        let mut session = Self {
            doc: Ok(PropsDocument::default()),
            known: None,
            generation: 0,
            history: History::default(),
            notice: None,
        };
        session.read(root);
        session
    }

    fn read(&mut self, root: &Path) {
        self.generation += 1;
        match file::read_doc(root, &PROPS) {
            Ok(None) => {
                self.doc = Ok(PropsDocument::default());
                self.known = None;
            }
            Ok(Some(live)) => {
                self.doc = live.text.and_then(|json| {
                    PropsDocument::parse(&json).map_err(|error| format!("{PROPS_FILE}: {error}"))
                });
                self.known = Some(live.known);
            }
            Err(error) => {
                self.doc = Err(format!("{PROPS_FILE} could not be read: {error}"));
                self.known = None;
            }
        }
    }

    /// Reloads after an outside change: the history is cleared (a running gesture is
    /// aborted) and `reason` is shown.
    pub fn reload(&mut self, root: &Path, reason: String) {
        self.read(root);
        self.history.clear();
        self.notice = Some(reason);
    }

    pub fn doc(&self) -> Result<&PropsDocument, String> {
        self.doc.as_ref().map_err(Clone::clone)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn known(&self) -> Option<&KnownFile> {
        self.known.as_ref()
    }

    pub(crate) fn set_known(&mut self, known: KnownFile) {
        self.known = Some(known);
    }

    /// Whether an inventory hash of the live file (`None`: no file) is what the session
    /// last loaded or wrote.
    pub fn is_known(&self, sha256: Option<&str>) -> bool {
        self.known.as_ref().map(|known| known.sha256.as_str()) == sha256
    }

    /// Replaces the document the preview draws.
    pub(crate) fn set_doc(&mut self, doc: PropsDocument) {
        self.doc = Ok(doc);
        self.generation += 1;
        self.notice = None;
    }

    /// The document for the preview and exports; `None` draws the code's values, also
    /// when the file is unreadable (its reason is the notice).
    pub fn props_json(&self) -> Option<String> {
        match &self.doc {
            Ok(doc) if !doc.objects.is_empty() => Some(doc.to_json()),
            _ => None,
        }
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn identity() -> EditorObjectIdentity {
        EditorObjectIdentity {
            scene_instance_key: "video".into(),
            component_key: "Title".into(),
            object_key: "headline".into(),
            repeat_key: "main".into(),
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_document_and_draws_the_code() {
        let temp = tempfile::tempdir().unwrap();
        let session = PropsSession::load(temp.path());
        assert_eq!(session.doc().unwrap(), &PropsDocument::default());
        assert_eq!(session.props_json(), None);
        assert!(session.is_known(None));
    }

    #[test]
    fn writes_round_trip_and_invalid_files_are_reported() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut doc = PropsDocument::default();
        doc.set(&identity(), "text", Some(json!("Hello")));
        let known = file::write_doc(root, &PROPS, &doc.to_json(), None, &mut |_| Ok(())).unwrap();
        let session = PropsSession::load(root);
        assert_eq!(session.doc().unwrap(), &doc);
        assert_eq!(session.known(), Some(&known));
        assert_eq!(session.props_json(), Some(doc.to_json()));

        std::fs::write(root.join(PROPS_FILE), "{\"version\":2,\"objects\":[]}").unwrap();
        let session = PropsSession::load(root);
        let error = session.doc().unwrap_err();
        assert!(error.starts_with(PROPS_FILE), "{error}");
        assert_eq!(session.props_json(), None);
    }

    #[test]
    fn an_interrupted_write_is_recovered() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut doc = PropsDocument::default();
        doc.set(&identity(), "size", Some(json!(48)));
        let crashed = file::write_doc(root, &PROPS, &doc.to_json(), None, &mut |_| {
            Err(std::io::Error::other("crash"))
        });
        assert!(crashed.is_err());
        // Staged before the crash, with no live file: the staged copy is published.
        let note = file::recover_doc(root, &PROPS).unwrap().unwrap();
        assert!(note.contains(PROPS_FILE), "{note}");
        assert_eq!(PropsSession::load(root).doc().unwrap(), &doc);
    }
}
