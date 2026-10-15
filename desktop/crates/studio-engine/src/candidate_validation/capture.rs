//! Capture of a quiesced draft into checkpoint objects, and revision restore.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CandidateError {
    #[error("the draft changed while it was being captured; retry after the writer is quiescent")]
    DraftChanged,
    #[error("candidate studio.json belongs to project {found}, not {expected}")]
    ProjectMismatch { expected: String, found: String },
    #[error("the draft is not a valid project: {0}")]
    InvalidDraft(String),
    #[error("the task source base is not available in history: {0}")]
    BaseMissing(String),
    #[error("candidate storage: {0}")]
    Storage(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    /// Same bytes, different executable bit.
    Mode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedPath {
    pub path: String,
    pub change: ChangeKind,
    pub kind: FileKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChangeSet {
    pub entries: Vec<ChangedPath>,
    /// Total changes, including entries beyond the bounded list.
    pub total: usize,
    pub truncated: bool,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Diff of two verified inventories (both sorted by path).
    pub fn between(base: &SourceInventory, candidate: &SourceInventory) -> Self {
        let mut set = Self::default();
        let mut push = |path: &str, change: ChangeKind, kind: FileKind| {
            set.total += 1;
            if set.entries.len() < MAX_CHANGED_PATHS {
                set.entries.push(ChangedPath {
                    path: path.to_owned(),
                    change,
                    kind,
                });
            } else {
                set.truncated = true;
            }
        };
        let (mut a, mut b) = (
            base.files.iter().peekable(),
            candidate.files.iter().peekable(),
        );
        loop {
            match (a.peek(), b.peek()) {
                (None, None) => break,
                (Some(old), None) => {
                    push(old.path.as_str(), ChangeKind::Deleted, old.kind);
                    a.next();
                }
                (None, Some(new)) => {
                    push(new.path.as_str(), ChangeKind::Added, new.kind);
                    b.next();
                }
                (Some(old), Some(new)) => match old.path.cmp(&new.path) {
                    std::cmp::Ordering::Less => {
                        push(old.path.as_str(), ChangeKind::Deleted, old.kind);
                        a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        push(new.path.as_str(), ChangeKind::Added, new.kind);
                        b.next();
                    }
                    std::cmp::Ordering::Equal => {
                        if old.sha256 != new.sha256 || old.size != new.size {
                            push(new.path.as_str(), ChangeKind::Modified, new.kind);
                        } else if old.executable != new.executable {
                            push(new.path.as_str(), ChangeKind::Mode, new.kind);
                        }
                        a.next();
                        b.next();
                    }
                },
            }
        }
        set
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRecord {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub executable: bool,
}

/// Immutable bytes proposed by one task, retained by hash. The identities are the
/// capture's own: nothing here is the task base or an M1 checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturedCandidate {
    pub(super) identity: TaskIdentity,
    pub(super) source_base: TaskSourceBase,
    pub(super) scope: TaskScope,
    pub(super) candidate: CandidateRevision,
    /// The candidate's revision without its timing document: what its build is keyed by.
    #[serde(skip)]
    pub(super) build_revision: SourceRevision,
    pub(super) manifest_sha256: String,
    pub(super) objects: Vec<ObjectRecord>,
    pub(super) changes: ChangeSet,
}

impl CapturedCandidate {
    pub fn identity(&self) -> &TaskIdentity {
        &self.identity
    }
    pub fn source_base(&self) -> &TaskSourceBase {
        &self.source_base
    }
    pub fn scope(&self) -> &TaskScope {
        &self.scope
    }
    pub fn candidate(&self) -> &CandidateRevision {
        &self.candidate
    }
    /// The revision the candidate's build names: its source without `timeline.json`.
    pub fn build_revision(&self) -> &SourceRevision {
        &self.build_revision
    }
    /// SHA-256 of the immutable checkpoint manifest file.
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
    pub fn objects(&self) -> &[ObjectRecord] {
        &self.objects
    }
    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }
}

pub(super) fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        hash.update(&buffer[..count]);
    }
}

/// Points of [`capture_candidate_observed`] where a test may mutate the draft to prove
/// that a changing tree fails capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePhase {
    /// The draft was scanned and is about to be captured into objects.
    Scanned,
    /// Objects were captured and reloaded; the final verification scan is next.
    Captured,
}

/// Capture the quiesced draft as immutable checkpoint objects. Verifies twice (scan +
/// capture/reload, then a final scan) and retains manifest/object hashes. A tree that
/// changes at any point fails with [`CandidateError::DraftChanged`].
pub fn capture_candidate(
    ticket: &CaptureTicket,
    checkpoints: &Checkpoints,
) -> Result<CapturedCandidate, CandidateError> {
    capture_candidate_observed(ticket, checkpoints, &mut |_| {})
}

/// Scan, capture into checkpoint objects, reload by hash, then scan again. Any
/// difference is [`CandidateError::DraftChanged`]: a changing tree is never captured.
pub(super) fn capture_verified(
    draft: &Path,
    checkpoints: &Checkpoints,
    observe: &mut dyn FnMut(CapturePhase),
) -> Result<(SourceRevision, SourceInventory), CandidateError> {
    let before =
        SourceInventory::scan(draft).map_err(|e| CandidateError::InvalidDraft(e.to_string()))?;
    observe(CapturePhase::Scanned);
    let revision = checkpoints.capture(draft).map_err(|e| {
        let text = e.to_string();
        // A file that vanished or shrank mid-capture is also a changing tree: compare
        // against what was scanned before deciding this is a storage problem.
        let moved = text.contains("changed while checkpointing")
            || SourceInventory::scan(draft).is_ok_and(|now| now.revision != before.revision);
        if moved {
            CandidateError::DraftChanged
        } else {
            CandidateError::Storage(text)
        }
    })?;
    if revision != before.revision {
        return Err(CandidateError::DraftChanged);
    }
    // Second verification: reload the manifest and every object by hash.
    let inventory = checkpoints
        .load(&revision)
        .map_err(|e| CandidateError::Storage(e.to_string()))?;
    observe(CapturePhase::Captured);
    let after =
        SourceInventory::scan(draft).map_err(|e| CandidateError::InvalidDraft(e.to_string()))?;
    if after.revision != revision || inventory.revision != revision {
        return Err(CandidateError::DraftChanged);
    }
    Ok((revision, inventory))
}

/// Capture a labelled immutable snapshot of `draft` (verified twice) without any task
/// ticket: used by read-only project tools while the writer gate is free.
pub fn capture_draft_revision(
    checkpoints: &Checkpoints,
    draft: &Path,
) -> Result<SourceRevision, CandidateError> {
    capture_verified(draft, checkpoints, &mut |_| {}).map(|(revision, _)| revision)
}

pub fn capture_candidate_observed(
    ticket: &CaptureTicket,
    checkpoints: &Checkpoints,
    observe: &mut dyn FnMut(CapturePhase),
) -> Result<CapturedCandidate, CandidateError> {
    capture_candidate_scoped_observed(
        ticket,
        checkpoints,
        TaskScope::whole_project(
            String::from(ticket.identity().project.clone()),
            ticket.source_base().revision().as_str(),
        ),
        observe,
    )
}

pub fn capture_candidate_scoped_observed(
    ticket: &CaptureTicket,
    checkpoints: &Checkpoints,
    scope: TaskScope,
    observe: &mut dyn FnMut(CapturePhase),
) -> Result<CapturedCandidate, CandidateError> {
    capture_tree(
        ticket.identity(),
        ticket.source_base(),
        ticket.draft(),
        checkpoints,
        scope,
        observe,
    )
}

/// Captures an Undo candidate: the current source with an inverse delta applied in a
/// private tree. There is no agent writer, ticket or lease; the identity is the Undo's
/// own (fresh task id, current session, a freshly allocated generation) and the source
/// base is the current source the inverse delta was formed from. The result is validated
/// through the same Stage 2 pipeline as any candidate.
pub fn capture_undo_candidate(
    identity: &TaskIdentity,
    source_base: &TaskSourceBase,
    tree: &Path,
    checkpoints: &Checkpoints,
) -> Result<CapturedCandidate, CandidateError> {
    capture_tree(
        identity,
        source_base,
        tree,
        checkpoints,
        TaskScope::whole_project(
            String::from(identity.project.clone()),
            source_base.revision().as_str(),
        ),
        &mut |_| {},
    )
}

pub(super) fn capture_tree(
    identity: &TaskIdentity,
    source_base: &TaskSourceBase,
    tree: &Path,
    checkpoints: &Checkpoints,
    scope: TaskScope,
    observe: &mut dyn FnMut(CapturePhase),
) -> Result<CapturedCandidate, CandidateError> {
    scope
        .validate()
        .map_err(|error| CandidateError::InvalidDraft(error.to_string()))?;
    if scope.project_id != String::from(identity.project.clone())
        || scope.source_revision != source_base.revision().as_str()
    {
        return Err(CandidateError::InvalidDraft(
            "task scope does not match the captured project/source base".into(),
        ));
    }
    let (revision, inventory) = capture_verified(tree, checkpoints, observe)?;
    let manifest_file = checkpoints.manifest_path(&revision);
    let manifest_sha256 =
        sha256_file(&manifest_file).map_err(|e| CandidateError::Storage(e.to_string()))?;
    let manifest_entry = inventory
        .files
        .iter()
        .find(|f| f.path.as_str() == "studio.json")
        .ok_or_else(|| CandidateError::InvalidDraft("studio.json is missing".into()))?;
    let manifest_bytes = std::fs::read(
        checkpoints
            .root
            .join("objects")
            .join(&manifest_entry.sha256),
    )
    .map_err(|e| CandidateError::Storage(e.to_string()))?;
    let manifest = Manifest::parse(&manifest_bytes)
        .map_err(|e| CandidateError::InvalidDraft(e.to_string()))?;
    if manifest.project_id != identity.project {
        return Err(CandidateError::ProjectMismatch {
            expected: String::from(identity.project.clone()),
            found: String::from(manifest.project_id),
        });
    }
    let base = checkpoints
        .load(source_base.revision())
        .map_err(|e| CandidateError::BaseMissing(e.to_string()))?;
    Ok(CapturedCandidate {
        identity: identity.clone(),
        source_base: source_base.clone(),
        scope,
        candidate: CandidateRevision::new(revision),
        build_revision: inventory.build_revision(),
        manifest_sha256,
        objects: inventory
            .files
            .iter()
            .map(|f| ObjectRecord {
                path: f.path.as_str().to_owned(),
                sha256: f.sha256.clone(),
                size: f.size,
                executable: f.executable,
            })
            .collect(),
        changes: ChangeSet::between(&base, &inventory),
    })
}

/// A private, immutable materialization of a captured revision, removed on drop. Build
/// requests keep it alive while compiling.
#[derive(Debug)]
pub struct RestoredRevision {
    pub project: OpenProject,
    pub(super) root: PathBuf,
}

impl RestoredRevision {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for RestoredRevision {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Restore `revision` (already present in `checkpoints`) into a private tree and open it
/// as a project. Objects are verified by the checkpoint store.
pub fn restore_revision(
    checkpoints: &Checkpoints,
    revision: &SourceRevision,
) -> Result<RestoredRevision, CandidateError> {
    let name = format!("candidate-{}", uuid::Uuid::new_v4().simple());
    let root = checkpoints
        .draft(revision, &name)
        .map_err(|e| CandidateError::Storage(e.to_string()))?;
    let guard_root = root.clone();
    match studio_project::open(&root) {
        Ok(project) => Ok(RestoredRevision { project, root }),
        Err(e) => {
            let _ = std::fs::remove_dir_all(guard_root);
            Err(CandidateError::InvalidDraft(e.to_string()))
        }
    }
}
