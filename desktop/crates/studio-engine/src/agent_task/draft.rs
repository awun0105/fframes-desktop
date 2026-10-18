//! The stable draft: one working copy per project that a task owns while it writes.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DraftState {
    /// A task owns the draft; seen after a restart it means the app ended abnormally.
    Active { task: String },
    /// A failed/cancelled/interrupted task's draft; archived before the next refresh.
    Retained { task: String, reason: String },
    /// The task's bytes were published; the draft may be replaced.
    Accepted { task: String },
    /// A writer may still be running; never refreshed until acknowledged.
    UnsafeWriter { task: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSnapshot {
    pub base: SourceRevision,
    pub state: DraftState,
}

/// Evidence that no writer can still be modifying a draft marked `UnsafeWriter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriterGoneEvidence {
    /// The user explicitly confirmed that nothing is writing to the draft.
    UserConfirmed,
    /// Verified termination under a qualified process-group model.
    QualifiedAndVerified {
        ownership: WriterOwnership,
        termination: TerminationReport,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDraft {
    pub path: PathBuf,
    pub archived: Option<PathBuf>,
}

/// The stable per-project agent working directory plus its durable ownership marker.
#[derive(Debug, Clone)]
pub struct DraftStore {
    pub(super) dir: PathBuf,
    pub(super) marker: PathBuf,
    pub(super) archive: PathBuf,
}

impl DraftStore {
    pub fn new(paths: &crate::app_paths::AppPaths, project: &ProjectId) -> Self {
        Self {
            dir: paths.agent_draft(project),
            marker: paths.agent_draft_state(project),
            archive: paths.agent_archive(project),
        }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    pub fn archive_dir(&self) -> &Path {
        &self.archive
    }

    pub fn snapshot(&self) -> Result<Option<DraftSnapshot>, TaskError> {
        match fs::read(&self.marker) {
            Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| {
                TaskError::DraftUnsafe(format!("draft ownership marker is unreadable ({e})"))
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(storage(e)),
        }
    }

    pub(super) fn write(&self, snapshot: &DraftSnapshot) -> Result<(), TaskError> {
        let parent = self.marker.parent().expect("marker has a parent");
        fs::create_dir_all(parent).map_err(storage)?;
        atomic_write(
            &self.marker,
            &serde_json::to_vec(snapshot).map_err(storage)?,
        )
        .map_err(storage)
    }

    /// After a restart an `Active` marker means the previous session never released the
    /// lease: a writer may still be running, so the draft becomes unsafe.
    pub fn recover_after_restart(&self) -> Result<(), TaskError> {
        if let Some(mut snapshot) = self.snapshot()?
            && let DraftState::Active { task } = &snapshot.state
        {
            snapshot.state = DraftState::UnsafeWriter {
                task: task.clone(),
                reason: "the previous session ended without releasing the writer lease; its adapter may still be running".into(),
            };
            self.write(&snapshot)?;
        }
        Ok(())
    }

    /// Updates the marker for the draft's current owner (task end, acknowledgement).
    pub fn set_state(&self, state: DraftState) -> Result<(), TaskError> {
        let mut snapshot = self
            .snapshot()?
            .ok_or_else(|| TaskError::Storage("draft ownership marker is missing".into()))?;
        snapshot.state = state;
        self.write(&snapshot)
    }

    /// Clears an `UnsafeWriter` marker given evidence that no writer remains. The
    /// draft becomes `Retained` and is archived before its next refresh.
    pub fn acknowledge_writer_gone(&self, evidence: &WriterGoneEvidence) -> Result<(), TaskError> {
        let Some(snapshot) = self.snapshot()? else {
            return Ok(());
        };
        let DraftState::UnsafeWriter { task, .. } = snapshot.state else {
            return Ok(());
        };
        match evidence {
            WriterGoneEvidence::UserConfirmed => {}
            WriterGoneEvidence::QualifiedAndVerified {
                ownership,
                termination,
            } => {
                if !ownership.is_qualified() || !termination.verified() {
                    return Err(TaskError::DraftUnsafe(
                        "evidence does not prove the writer is gone".into(),
                    ));
                }
            }
        }
        self.set_state(DraftState::Retained {
            task,
            reason: "writer confirmed gone".into(),
        })
    }

    /// Materializes the stable draft from `base` for `task`.
    ///
    /// Never runs under a live provider session or while an unknown writer may
    /// survive; a retained failed draft is archived first, not overwritten.
    /// Materializes the stable draft from `content_revision` for `task`, setting its
    /// source base to `base`.
    ///
    /// Never runs under a live provider session or while an unknown writer may
    /// survive; a retained failed draft is archived first, not overwritten.
    pub fn prepare_from(
        &self,
        checkpoints: &Checkpoints,
        content_revision: &studio_project::SourceRevision,
        base: &TaskSourceBase,
        task: &AgentTaskId,
        session_live: bool,
    ) -> Result<PreparedDraft, TaskError> {
        if session_live {
            return Err(TaskError::DraftBusy);
        }
        let previous = self.snapshot()?;
        let mut archived = None;
        match previous.as_ref().map(|s| &s.state) {
            Some(DraftState::Active { task }) => {
                return Err(TaskError::DraftUnsafe(format!(
                    "task {task} still owns the draft and a writer may be running"
                )));
            }
            Some(DraftState::UnsafeWriter { reason, .. }) => {
                return Err(TaskError::DraftUnsafe(reason.clone()));
            }
            Some(DraftState::Retained { task, .. }) if self.dir.exists() => {
                archived = Some(self.archive_current(task)?);
            }
            // No marker but a directory: an interrupted refresh that never ran a writer.
            None if self.dir.exists() => {
                archived = Some(self.archive_current("unmarked")?);
            }
            Some(DraftState::Accepted { .. }) if self.dir.exists() => {
                fs::remove_dir_all(&self.dir).map_err(storage)?;
            }
            _ => {}
        }
        let parent = self.dir.parent().expect("draft has a parent");
        fs::create_dir_all(parent).map_err(storage)?;
        let staging = format!("agent-{}", task.0);
        let staged = checkpoints
            .draft(content_revision, &staging)
            .map_err(storage)?;
        if let Err(error) = fs::rename(&staged, &self.dir) {
            let _ = fs::remove_dir_all(&staged);
            return Err(storage(error));
        }
        sync_directory(parent).map_err(storage)?;
        self.write(&DraftSnapshot {
            base: base.revision().clone(),
            state: DraftState::Active {
                task: task.0.clone(),
            },
        })?;
        Ok(PreparedDraft {
            path: self.dir.clone(),
            archived,
        })
    }

    /// Materializes the stable draft from `base` for `task`.
    pub fn prepare(
        &self,
        checkpoints: &Checkpoints,
        base: &TaskSourceBase,
        task: &AgentTaskId,
        session_live: bool,
    ) -> Result<PreparedDraft, TaskError> {
        self.prepare_from(checkpoints, base.revision(), base, task, session_live)
    }

    pub(super) fn archive_current(&self, owner: &str) -> Result<PathBuf, TaskError> {
        fs::create_dir_all(&self.archive).map_err(storage)?;
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut target = self.archive.join(format!("{secs}-{owner}"));
        let mut counter = 1;
        while target.exists() {
            target = self.archive.join(format!("{secs}-{owner}-{counter}"));
            counter += 1;
        }
        fs::rename(&self.dir, &target).map_err(storage)?;
        sync_directory(&self.archive).map_err(storage)?;
        Ok(target)
    }
}
