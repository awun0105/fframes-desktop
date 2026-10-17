//! Versioned durable session manifest and bounded handoff context storage.
//!
//! Stores opaque native session IDs, provider/profile identity, project/draft identity,
//! last source and accepted revisions, capability snapshots, and transcript boundaries
//! using owner-only atomic storage. Redacts opaque session IDs for telemetry/UI.

use crate::{TaskScope, app_paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use studio_harness::driver::AgentCapabilityInfo;
use studio_project::ProjectId;

pub mod history;
pub mod layout;
pub mod startup;

/// The only manifest schema this Studio reads and writes. A manifest of any other version is
/// refused and left as it is (see [`SessionStore::load`]).
pub const SESSION_MANIFEST_SCHEMA_VERSION: u32 = 2;
pub const CONTEXT_SCHEMA_VERSION: &str = "m6-context/1";
pub const MAX_CONTEXT_ENVELOPE_BYTES: usize = 64 * 1024; // 64 KiB

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionManifestState {
    Ready,
    Ended,
    Unsafe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionManifest {
    pub schema_version: u32,
    pub provider_id: String,
    /// Opaque native driver session ID (stored securely, redacted in UI/logs).
    pub wire_session_id: Option<String>,
    pub project_id: String,
    pub draft_path: PathBuf,
    pub launch_digest: String,
    pub last_source_revision: String,
    pub last_accepted_revision: String,
    pub context_schema: String,
    pub capability_snapshot: Option<AgentCapabilityInfo>,
    pub transcript_boundary: usize,
    pub state: SessionManifestState,
    pub created_at: String,
    pub updated_at: String,
}

impl SessionManifest {
    pub fn is_resumable(&self) -> bool {
        self.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
            && self.state == SessionManifestState::Ready
            && self.wire_session_id.is_some()
    }

    /// Redacted view of the session manifest for safe logging and UI display.
    pub fn redacted_id(&self) -> String {
        match &self.wire_session_id {
            Some(id) if id.len() > 12 => format!("{}…{}", &id[..6], &id[id.len() - 4..]),
            Some(id) => format!("{id}…"),
            None => "none".to_string(),
        }
    }
}

/// Bounded context transferred between provider sessions on handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedHandoffContext {
    pub schema: String,
    pub source_revision: String,
    pub draft_revision: Option<String>,
    pub brief: String,
    pub outcome_summary: Option<String>,
    pub scope: Option<TaskScope>,
    pub style_snapshot: Option<String>,
    pub diagnostics: Vec<String>,
    pub omissions: Vec<String>,
}

impl BoundedHandoffContext {
    pub fn new(
        source_revision: String,
        draft_revision: Option<String>,
        brief: String,
        outcome_summary: Option<String>,
        scope: Option<TaskScope>,
        style_snapshot: Option<String>,
        diagnostics: Vec<String>,
    ) -> Self {
        let mut omissions = Vec::new();
        let mut ctx = Self {
            schema: CONTEXT_SCHEMA_VERSION.to_string(),
            source_revision,
            draft_revision,
            brief,
            outcome_summary,
            scope,
            style_snapshot,
            diagnostics,
            omissions: Vec::new(),
        };

        for diagnostic in &mut ctx.diagnostics {
            truncate_utf8(diagnostic, 4096);
        }
        if ctx.diagnostics.len() > 32 {
            ctx.diagnostics.truncate(32);
            omissions.push("diagnostics_truncated".to_string());
        }
        if let Some(outcome) = &mut ctx.outcome_summary {
            truncate_utf8(outcome, 4096);
        }
        truncate_utf8(&mut ctx.brief, 16 * 1024);
        if serialized_len(&ctx) > MAX_CONTEXT_ENVELOPE_BYTES {
            ctx.scope = None;
            omissions.push("scope_omitted".to_string());
        }
        if serialized_len(&ctx) > MAX_CONTEXT_ENVELOPE_BYTES {
            ctx.style_snapshot = None;
            omissions.push("style_snapshot_omitted".to_string());
        }
        if serialized_len(&ctx) > MAX_CONTEXT_ENVELOPE_BYTES {
            if let Some(outcome) = &mut ctx.outcome_summary {
                truncate_utf8(outcome, 1024);
            }
            omissions.push("outcome_summary_truncated".to_string());
        }
        while serialized_len(&ctx) > MAX_CONTEXT_ENVELOPE_BYTES && !ctx.diagnostics.is_empty() {
            ctx.diagnostics.pop();
            if !omissions.iter().any(|item| item == "diagnostics_truncated") {
                omissions.push("diagnostics_truncated".to_string());
            }
        }
        if serialized_len(&ctx) > MAX_CONTEXT_ENVELOPE_BYTES {
            truncate_utf8(&mut ctx.brief, 1024);
            omissions.push("brief_truncated".to_string());
        }
        if serialized_len(&ctx) > MAX_CONTEXT_ENVELOPE_BYTES {
            truncate_utf8(&mut ctx.source_revision, 128);
            if let Some(revision) = &mut ctx.draft_revision {
                truncate_utf8(revision, 128);
            }
            omissions.push("revision_identity_truncated".to_string());
        }
        ctx.omissions = omissions;
        ctx
    }

    pub fn to_json_envelope(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

fn serialized_len(context: &BoundedHandoffContext) -> usize {
    serde_json::to_vec(context).map_or(usize::MAX, |bytes| bytes.len())
}

fn truncate_utf8(text: &mut String, max_bytes: usize) {
    if text.len() > max_bytes {
        let mut end = max_bytes;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
}

pub struct SessionStore;

/// What the stored manifest of a session allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestStatus {
    /// The session has no manifest.
    Missing,
    /// A manifest of the current format.
    Ok,
    /// A manifest this Studio cannot use, with the reason shown to the user. The file is left
    /// as it is.
    Refused(String),
}

/// Reads a manifest file's text: the current schema only. A manifest of another version is
/// refused with a message that names the file and says which Studio made it; nothing is
/// rewritten.
pub(super) fn parse_manifest(text: &str, path: &Path) -> Result<SessionManifest, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("cannot parse session manifest {}: {e}", path.display()))?;
    check_manifest_version(&value, path)?;
    serde_json::from_value(value).map_err(|e| {
        format!(
            "The saved agent session {} is not in the format this Studio reads ({e}). It is left as it is and cannot be restored.",
            path.display()
        )
    })
}

fn check_manifest_version(value: &serde_json::Value, path: &Path) -> Result<(), String> {
    let path = path.display();
    match value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
    {
        Some(version) if version < u64::from(SESSION_MANIFEST_SCHEMA_VERSION) => Err(format!(
            "The saved agent session {path} was made by an older Studio (manifest format {version}, this Studio reads format {SESSION_MANIFEST_SCHEMA_VERSION}). It is left as it is and cannot be restored."
        )),
        Some(version) if version > u64::from(SESSION_MANIFEST_SCHEMA_VERSION) => Err(format!(
            "The saved agent session {path} was made by a newer Studio (manifest format {version}, this Studio reads format {SESSION_MANIFEST_SCHEMA_VERSION}). It is left as it is and cannot be restored."
        )),
        _ => Ok(()),
    }
}

/// Refuses to replace or remove a manifest file that is valid JSON but not the current
/// format. Text that is not JSON at all, or a file that cannot be read, stays replaceable.
fn refuse_foreign_manifest(path: &Path) -> Result<(), String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(());
    };
    if serde_json::from_str::<serde_json::Value>(&text).is_err() {
        return Ok(());
    }
    parse_manifest(&text, path).map(|_| ())
}
impl SessionStore {
    /// Where the single-session API keeps its manifest: the current session's directory.
    /// `None` while the project holds no session.
    fn manifest_path(paths: &AppPaths, project_id: &ProjectId) -> Option<PathBuf> {
        Self::current_session_id(paths, project_id)
            .map(|session| layout::manifest_file(paths, project_id, &session))
    }

    /// Writes the current session's manifest. A manifest of another schema version already
    /// there is refused and stays as it is.
    pub fn save(
        paths: &AppPaths,
        project_id: &ProjectId,
        manifest: &SessionManifest,
    ) -> Result<(), String> {
        let path = Self::manifest_path(paths, project_id)
            .ok_or_else(|| "the project has no current session".to_owned())?;
        refuse_foreign_manifest(&path)?;
        let bytes = serde_json::to_vec_pretty(manifest)
            .map_err(|e| format!("cannot serialize session manifest: {e}"))?;
        write_atomic(&path, &bytes)
    }

    /// The current session's manifest; `None` when it has none. A manifest of another schema
    /// version is an error that says which Studio made it; the file is left untouched.
    pub fn load(
        paths: &AppPaths,
        project_id: &ProjectId,
    ) -> Result<Option<SessionManifest>, String> {
        let Some(path) = Self::manifest_path(paths, project_id) else {
            return Ok(None);
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => parse_manifest(&text, &path).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!(
                "cannot read session manifest {}: {e}",
                path.display()
            )),
        }
    }

    /// Removes the current session's manifest (a manifest of another schema version stays).
    pub fn clear(paths: &AppPaths, project_id: &ProjectId) -> Result<(), String> {
        let Some(path) = Self::manifest_path(paths, project_id) else {
            return Ok(());
        };
        refuse_foreign_manifest(&path)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("cannot clear session manifest: {e}")),
        }
    }
    /// What the stored manifest of `session` allows: the reason is what to show when a switch
    /// or a restore is refused. Reads only; the file is never changed.
    pub fn manifest_status(
        paths: &AppPaths,
        project_id: &ProjectId,
        session: &str,
    ) -> ManifestStatus {
        if !layout::is_valid_session_id(session) {
            return ManifestStatus::Missing;
        }
        let path = layout::manifest_file(paths, project_id, session);
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_manifest(&text, &path) {
                Ok(_) => ManifestStatus::Ok,
                Err(reason) => ManifestStatus::Refused(reason),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => ManifestStatus::Missing,
            Err(e) => ManifestStatus::Refused(format!(
                "cannot read session manifest {}: {e}",
                path.display()
            )),
        }
    }
}
static TEMPORARY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::Ordering;

    let parent = path.parent().ok_or_else(|| "invalid path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| format!("cannot create directory: {e}"))?;

    let temporary = path.with_extension(format!(
        "manifest.{}.{}.tmp",
        std::process::id(),
        TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let written = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(|e| format!("cannot write session manifest: {e}"))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| format!("cannot write session manifest: {e}"))?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| format!("cannot save session manifest: {e}"))
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}
