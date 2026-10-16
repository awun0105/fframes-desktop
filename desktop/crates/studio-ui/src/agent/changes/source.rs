//! Where the viewer's data comes from: the engine's per-file text mapped to [`DiffFile`]s, the
//! small interface the host calls on the workflow (so tests can stand in for it), and the facts
//! the host reads from a workflow snapshot.

use studio_engine::agent::{
    AgentWorkflow, NoticeLevel, RowKind, TaskPhase, WorkflowError, WorkflowSnapshot,
};
use studio_engine::candidate_validation::{
    FileDiff as EngineDiff, FileText, NotShown as EngineNotShown,
};

use super::model::{DiffFile, NotShown};

/// The engine's listing as the viewer takes it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Loaded {
    pub files: Vec<DiffFile>,
    /// Changed files the listing left out. They have no entry in `files`.
    pub more: usize,
}

fn not_shown(reason: &EngineNotShown) -> Option<NotShown> {
    match reason {
        EngineNotShown::Binary => Some(NotShown::Binary),
        EngineNotShown::TooLarge { .. } => Some(NotShown::TooLarge),
        EngineNotShown::Budget => Some(NotShown::Budget),
        EngineNotShown::Unreadable(_) => Some(NotShown::Unreadable),
        // Not a reason a file is hidden: the marker for files the listing omitted.
        EngineNotShown::More { .. } => None,
    }
}

/// One side of a file: its text, `None` when absent. A hidden side keeps an empty text so the
/// file still reads as added or deleted; the viewer never diffs a file with a reason.
fn side(text: FileText) -> (Option<String>, Option<NotShown>) {
    match text {
        FileText::Absent => (None, None),
        FileText::Text(text) => (Some(text), None),
        FileText::NotShown(reason) => (Some(String::new()), not_shown(&reason)),
    }
}

/// Why the reader could not read a side, when that is what hid it.
fn unreadable_detail(text: &FileText) -> Option<String> {
    match text {
        FileText::NotShown(EngineNotShown::Unreadable(reason)) => Some(reason.clone()),
        _ => None,
    }
}

/// How many files the trailing marker entry stands for, if `diff` is that marker.
fn more_files(diff: &EngineDiff) -> Option<usize> {
    [&diff.before, &diff.after]
        .into_iter()
        .find_map(|text| match text {
            FileText::NotShown(EngineNotShown::More { files }) => Some(*files),
            _ => None,
        })
}

/// Maps the engine's per-file text. The trailing `More` marker (it has an empty path) becomes
/// a count and never a file, so no empty path can reach a toggle or a Reject.
pub fn map_diffs(diffs: Vec<EngineDiff>) -> Loaded {
    let mut loaded = Loaded::default();
    for diff in diffs {
        if let Some(files) = more_files(&diff) {
            loaded.more += files;
            continue;
        }
        if diff.path.is_empty() {
            continue;
        }
        let detail = [&diff.before, &diff.after]
            .into_iter()
            .find_map(unreadable_detail);
        let (before, hidden_before) = side(diff.before);
        let (after, hidden_after) = side(diff.after);
        let mut file = DiffFile::new(diff.path, before, after);
        file.not_shown = hidden_before.or(hidden_after);
        file.detail = detail;
        loaded.files.push(file);
    }
    loaded
}

/// A finished diff read: the listing and its prepared diffs, built off the UI thread.
pub struct Read {
    pub loaded: Loaded,
    pub prepared: super::Prepared,
}

impl Read {
    pub fn from_diffs(diffs: Vec<EngineDiff>) -> Self {
        let loaded = map_diffs(diffs);
        let prepared = super::Prepared::compute(&loaded.files);
        Self { loaded, prepared }
    }
}

/// What the review host asks of the workflow. Every call may take the engine's controller
/// lock or read files, so the host only makes them off the UI thread.
pub trait ReviewBackend: Send + Sync + 'static {
    fn candidate_diffs(&self) -> Result<Vec<EngineDiff>, String>;
    fn revision_diffs(&self, revision: &str) -> Result<Vec<EngineDiff>, String>;
    /// Leaves `paths` out of the candidate. The result arrives through the snapshot.
    fn reject_files(&self, paths: &[String]) -> Result<(), String>;
    /// Reverts one file of an applied revision. The result arrives through the snapshot.
    fn revert_path(&self, revision: &str, path: &str) -> Result<(), String>;
    /// Drops the candidate under review.
    fn discard(&self) -> Result<(), String>;
}

fn text(error: WorkflowError) -> String {
    error.to_string()
}

impl ReviewBackend for AgentWorkflow {
    fn candidate_diffs(&self) -> Result<Vec<EngineDiff>, String> {
        self.candidate_file_diffs().map_err(text)
    }

    fn revision_diffs(&self, revision: &str) -> Result<Vec<EngineDiff>, String> {
        self.revision_file_diffs(revision).map_err(text)
    }

    fn reject_files(&self, paths: &[String]) -> Result<(), String> {
        AgentWorkflow::reject_files(self, paths).map_err(text)
    }

    fn revert_path(&self, revision: &str, path: &str) -> Result<(), String> {
        AgentWorkflow::revert_path(self, revision, path).map_err(text)
    }

    fn discard(&self) -> Result<(), String> {
        AgentWorkflow::discard(self).map_err(text)
    }
}

/// The parts of a workflow snapshot the review host acts on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub phase: Option<TaskPhase>,
    /// The task and the candidate it is reviewing; changes when a reject replaces the
    /// candidate.
    pub candidate: Option<String>,
    /// A task is running or waiting (anything short of an ended task).
    pub task_active: bool,
    pub undo_in_progress: bool,
    /// The files of the watched revision that were reverted one by one.
    pub reverted: Vec<String>,
    /// The newest row of the conversation.
    pub newest_row: u64,
    /// The newest error or warning row: its id and what it says.
    pub refusal: Option<(u64, String)>,
}

/// Reads [`Facts`] from a snapshot; `revision` is the applied revision the host watches.
pub fn facts(snapshot: &WorkflowSnapshot, revision: Option<&str>) -> Facts {
    let task = snapshot.task.as_ref();
    let mut newest_row = 0;
    let mut refusal: Option<(u64, String)> = None;
    for row in &snapshot.rows {
        newest_row = newest_row.max(row.id.0);
        let said = match &row.kind {
            RowKind::Error(error) => Some(if error.detail.is_empty() {
                error.title.clone()
            } else if error.detail.starts_with(&error.title) {
                error.detail.clone()
            } else {
                format!("{}: {}", error.title, error.detail)
            }),
            RowKind::Notice {
                level: NoticeLevel::Warning,
                text,
            } => Some(text.clone()),
            _ => None,
        };
        if let Some(said) = said
            && refusal.as_ref().is_none_or(|(id, _)| row.id.0 >= *id)
        {
            refusal = Some((row.id.0, said));
        }
    }
    Facts {
        phase: task.map(|t| t.phase),
        candidate: task.and_then(|t| {
            t.review
                .as_ref()
                .map(|review| format!("{}/{}", t.id.0, review.candidate))
        }),
        task_active: task.is_some_and(|t| !t.phase.is_terminal()),
        undo_in_progress: matches!(
            snapshot.undo,
            studio_engine::agent::UndoView::InProgress { .. }
        ),
        reverted: revision
            .and_then(|revision| snapshot.history.iter().find(|entry| entry.id == revision))
            .map(|entry| entry.reverted_paths.clone())
            .unwrap_or_default(),
        newest_row,
        refusal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_engine::candidate_validation::ChangeKind;

    fn diff(path: &str, before: FileText, after: FileText) -> EngineDiff {
        EngineDiff {
            path: path.into(),
            change: ChangeKind::Modified,
            before,
            after,
        }
    }

    fn marker(files: usize) -> EngineDiff {
        diff(
            "",
            FileText::NotShown(EngineNotShown::More { files }),
            FileText::NotShown(EngineNotShown::More { files }),
        )
    }

    #[test]
    fn text_and_absent_sides_map_to_options() {
        let loaded = map_diffs(vec![
            diff("a.rs", FileText::Absent, FileText::Text("x\n".into())),
            diff("b.rs", FileText::Text("y\n".into()), FileText::Absent),
        ]);
        assert_eq!(loaded.more, 0);
        assert_eq!(
            loaded.files,
            vec![
                DiffFile::new("a.rs", None, Some("x\n".into())),
                DiffFile::new("b.rs", Some("y\n".into()), None),
            ]
        );
    }

    #[test]
    fn every_reason_a_file_is_hidden_has_its_own_variant() {
        let cases = [
            (EngineNotShown::Binary, NotShown::Binary),
            (
                EngineNotShown::TooLarge { size: 9_000_000 },
                NotShown::TooLarge,
            ),
            (EngineNotShown::Budget, NotShown::Budget),
            (
                EngineNotShown::Unreadable("denied".into()),
                NotShown::Unreadable,
            ),
        ];
        for (engine, expected) in cases {
            // The reason on either side hides the whole file.
            for hidden_before in [true, false] {
                let (before, after) = if hidden_before {
                    (
                        FileText::NotShown(engine.clone()),
                        FileText::Text("x\n".into()),
                    )
                } else {
                    (
                        FileText::Text("x\n".into()),
                        FileText::NotShown(engine.clone()),
                    )
                };
                let loaded = map_diffs(vec![diff("f.bin", before, after)]);
                assert_eq!(loaded.files[0].not_shown, Some(expected), "{engine:?}");
            }
        }
    }

    #[test]
    fn an_unreadable_file_keeps_the_readers_reason_in_its_notice() {
        let loaded = map_diffs(vec![diff(
            "secret.txt",
            FileText::Text("x\n".into()),
            FileText::NotShown(EngineNotShown::Unreadable("permission denied".into())),
        )]);
        assert_eq!(loaded.files[0].detail.as_deref(), Some("permission denied"));
        let prepared = Read::from_diffs(vec![diff(
            "secret.txt",
            FileText::Absent,
            FileText::NotShown(EngineNotShown::Unreadable("permission denied".into())),
        )]);
        assert_eq!(
            prepared.prepared.diffs[0]
                .notices
                .last()
                .map(String::as_str),
            Some("File could not be read, contents not shown: permission denied")
        );
    }

    #[test]
    fn a_hidden_added_file_still_reads_as_added() {
        let loaded = map_diffs(vec![diff(
            "new.bin",
            FileText::Absent,
            FileText::NotShown(EngineNotShown::Binary),
        )]);
        let file = &loaded.files[0];
        assert!(file.before.is_none() && file.after.is_some());
        assert_eq!(file.not_shown, Some(NotShown::Binary));
    }

    #[test]
    fn the_more_marker_is_a_count_and_never_a_file() {
        let loaded = map_diffs(vec![
            diff("a.rs", FileText::Absent, FileText::Text("x\n".into())),
            marker(37),
        ]);
        assert_eq!(loaded.more, 37);
        assert_eq!(loaded.files.len(), 1);
        assert!(loaded.files.iter().all(|file| !file.path.is_empty()));
    }
}
