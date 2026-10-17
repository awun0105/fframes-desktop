//! On-disk layout of the stored agent sessions.
//!
//! ```text
//! agent/sessions/
//!   current                  session id the single-session API serves
//!   <id>/manifest.json       [`SessionManifest`], schema version 2 (absent until a wire session exists)
//!   <id>/meta.json           [`SessionMeta`]: stored title, creation time, row id bounds
//!   <id>/conversation.jsonl  the row log of that session
//! ```
//!
//! Attachment bytes are never stored: a `User` row carries text only.
//!
//! Files of an earlier layout (a `conversation.jsonl` next to the project, an
//! `agent/session-manifest.json`) are not read and not touched: a project that holds only
//! those starts with an empty history.
//!
//! A missing, unreadable or dangling `current` pointer is repaired by pointing it at the most
//! recently updated session.
//!
//! # How the workflow uses it
//!
//! `AgentWorkflow::open` runs `SessionStore::prepare` and opens the log it names with
//! `RowStore::open_with_floor`, so row ids stay unique across the sessions of a project. New
//! chat and opening a stored session flush the open log, then use
//! [`SessionStore::create_session`] / [`SessionStore::set_current`].
//!
//! [`SessionStore::load`], [`SessionStore::save`] and [`SessionStore::clear`] follow the
//! `current` pointer. `clear` removes only the current session's `manifest.json`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use studio_project::ProjectId;

pub use crate::app_paths::is_valid_session_id;

use super::{SessionManifest, SessionStore, write_atomic};
use crate::agent::log;
use crate::app_paths::AppPaths;

pub const SESSION_META_SCHEMA_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const META_FILE: &str = "meta.json";
const LOG_FILE: &str = "conversation.jsonl";
const STAGING_PREFIX: &str = ".staging-";
const RENAME_ATTEMPTS: u32 = 6;
const RENAME_FIRST_DELAY: Duration = Duration::from_millis(25);

fn default_first_row_id() -> u64 {
    1
}

/// Small facts about a session that its manifest (which only exists once a wire session is
/// open) and its log cannot give. A meta file of another version or with a field this Studio
/// does not know is not used and never rewritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMeta {
    pub schema_version: u32,
    /// A title chosen by the user or the agent; the list falls back to the first prompt.
    #[serde(default)]
    pub title: Option<String>,
    pub created_unix: u64,
    /// Row ids of this session start here at least, which keeps them unique across the
    /// sessions of a project.
    #[serde(default = "default_first_row_id")]
    pub first_row_id: u64,
    /// The newest row id this session is known to have used (0 while unknown). Recorded when
    /// the session is left, so the ids of every other session can start after it.
    #[serde(default)]
    pub last_row_id: u64,
}

pub(super) fn manifest_file(paths: &AppPaths, project: &ProjectId, session: &str) -> PathBuf {
    paths
        .agent_session_dir(project, session)
        .join(MANIFEST_FILE)
}

pub(super) fn meta_file(paths: &AppPaths, project: &ProjectId, session: &str) -> PathBuf {
    paths.agent_session_dir(project, session).join(META_FILE)
}

pub(super) fn log_file(paths: &AppPaths, project: &ProjectId, session: &str) -> PathBuf {
    paths.agent_session_dir(project, session).join(LOG_FILE)
}

pub(super) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Flushes a directory entry change to disk. Unix only: Windows cannot open a directory like
/// this and flushes metadata on its own.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<(), String> {
    std::fs::File::open(dir)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("cannot sync directory: {e}"))
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<(), String> {
    Ok(())
}

/// A directory flush is a durability hint taken after the change succeeded, not a gate: a failure
/// is reported and the operation carries on.
fn sync_dir_best_effort(dir: &Path) {
    if let Err(e) = sync_dir(dir) {
        eprintln!("agent sessions: {e} ({})", dir.display());
    }
}

/// Runs `op`, retrying with a doubling delay while it fails with `PermissionDenied`: on Windows
/// a rename or removal is denied for a moment while a scanner or indexer holds the file.
fn retry_denied<T>(op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    retry_denied_after(RENAME_FIRST_DELAY, op)
}

fn retry_denied_after<T>(
    first_delay: Duration,
    mut op: impl FnMut() -> std::io::Result<T>,
) -> std::io::Result<T> {
    let mut delay = first_delay;
    let mut attempt = 1;
    loop {
        match op() {
            Err(e)
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    && attempt < RENAME_ATTEMPTS =>
            {
                std::thread::sleep(delay);
                delay *= 2;
                attempt += 1;
            }
            other => return other,
        }
    }
}

/// Ids of the session directories, in no particular order.
pub(super) fn session_ids(paths: &AppPaths, project: &ProjectId) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(paths.agent_sessions(project)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (is_valid_session_id(&name) && entry.path().is_dir()).then_some(name)
        })
        .collect()
}

fn last_modified(paths: &AppPaths, project: &ProjectId, session: &str) -> std::time::SystemTime {
    [
        manifest_file(paths, project, session),
        meta_file(paths, project, session),
        log_file(paths, project, session),
    ]
    .iter()
    .filter_map(|path| std::fs::metadata(path).and_then(|m| m.modified()).ok())
    .max()
    .unwrap_or(std::time::UNIX_EPOCH)
}

impl SessionStore {
    /// What the `current` pointer names, if it names an existing session. Never repairs.
    fn pointer_target(paths: &AppPaths, project: &ProjectId) -> Option<String> {
        let text = std::fs::read_to_string(paths.agent_session_current(project)).ok()?;
        let id = text.trim();
        (is_valid_session_id(id) && paths.agent_session_dir(project, id).is_dir())
            .then(|| id.to_owned())
    }

    /// The session the single-session API serves. `None` only while the project holds no
    /// session directory. A missing, unreadable or dangling pointer is repaired: it is
    /// rewritten to name the most recently updated session (ties: greatest id), and the repair
    /// is reported on stderr.
    pub fn current_session_id(paths: &AppPaths, project: &ProjectId) -> Option<String> {
        if let Some(id) = Self::pointer_target(paths, project) {
            return Some(id);
        }
        let id = session_ids(paths, project).into_iter().max_by(|a, b| {
            last_modified(paths, project, a)
                .cmp(&last_modified(paths, project, b))
                .then_with(|| a.cmp(b))
        })?;
        let saved = Self::write_pointer(paths, project, &id);
        eprintln!(
            "agent sessions: the current-session pointer was missing or invalid; using `{id}`{}",
            saved.err().map_or(String::new(), |e| format!(" ({e})"))
        );
        Some(id)
    }

    fn write_pointer(paths: &AppPaths, project: &ProjectId, session: &str) -> Result<(), String> {
        write_atomic(&paths.agent_session_current(project), session.as_bytes())?;
        sync_dir_best_effort(&paths.agent_sessions(project));
        Ok(())
    }

    /// Makes `session` the one the single-session API serves. Call it only while no actor owns
    /// the log: the actor keeps writing to the log it opened.
    pub fn set_current(paths: &AppPaths, project: &ProjectId, session: &str) -> Result<(), String> {
        if !is_valid_session_id(session) || !paths.agent_session_dir(project, session).is_dir() {
            return Err(format!("unknown session `{session}`"));
        }
        Self::write_pointer(paths, project, session)
    }

    /// The log of the current session; `None` while the project holds no session.
    pub fn current_log_path(paths: &AppPaths, project: &ProjectId) -> Option<PathBuf> {
        Self::current_session_id(paths, project).map(|session| log_file(paths, project, &session))
    }

    /// The log of one stored session (not checked for existence).
    pub fn session_log_path(
        paths: &AppPaths,
        project: &ProjectId,
        session: &str,
    ) -> Result<PathBuf, String> {
        if !is_valid_session_id(session) {
            return Err(format!("invalid session id `{session}`"));
        }
        Ok(log_file(paths, project, session))
    }

    /// The smallest row id a log of `session` may hand out: after its recorded floor and after
    /// every row already in the log. Flush the log first, only flushed rows are counted.
    /// Pass it to `RowStore::open_with_floor`.
    pub fn row_id_floor(paths: &AppPaths, project: &ProjectId, session: &str) -> u64 {
        let first = read_meta(&meta_file(paths, project, session))
            .map_or(1, |meta| meta.first_row_id)
            .max(1);
        let after_rows = log::read_rows(&log_file(paths, project, session))
            .ok()
            .and_then(|rows| rows.iter().map(|row| row.id.0).max())
            .map_or(1, |max| max.saturating_add(1));
        first.max(after_rows)
    }

    /// The smallest row id a new session of the project may hand out: after the ids every
    /// session recorded (`first_row_id` and `last_row_id` of the metas). Reads only the small
    /// meta files.
    pub fn project_row_floor(paths: &AppPaths, project: &ProjectId) -> u64 {
        session_ids(paths, project)
            .iter()
            .filter_map(|id| read_meta(&meta_file(paths, project, id)))
            .map(|meta| meta.first_row_id.max(meta.last_row_id.saturating_add(1)))
            .max()
            .unwrap_or(1)
            .max(1)
    }

    /// Records that `session` used row ids up to `last` (see `SessionMeta::last_row_id`).
    /// Does nothing when the meta already says as much or does not exist.
    pub fn record_last_row_id(
        paths: &AppPaths,
        project: &ProjectId,
        session: &str,
        last: u64,
    ) -> Result<(), String> {
        let path = meta_file(paths, project, session);
        let Some(mut meta) = read_meta(&path) else {
            return Ok(());
        };
        if meta.last_row_id >= last {
            return Ok(());
        }
        meta.last_row_id = last;
        write_meta(&paths.agent_session_dir(project, session), &meta)
    }

    /// The manifest of one stored session; `None` when it has none or it cannot be used.
    pub fn session_manifest(
        paths: &AppPaths,
        project: &ProjectId,
        session: &str,
    ) -> Option<SessionManifest> {
        is_valid_session_id(session)
            .then(|| read_manifest(&manifest_file(paths, project, session)))
            .flatten()
    }

    /// Creates an empty session and returns its id. It does not become current, unless the
    /// project has no valid pointer yet (see the module docs). Its row ids continue after
    /// those of every session, and after the rows of the session that is current now: flush
    /// that session's log first.
    pub fn create_session(
        paths: &AppPaths,
        project: &ProjectId,
        title: Option<String>,
    ) -> Result<String, String> {
        Self::create_session_at(paths, project, title, now_unix())
    }

    /// [`Self::create_session`] with an explicit creation time (unix seconds).
    pub fn create_session_at(
        paths: &AppPaths,
        project: &ProjectId,
        title: Option<String>,
        created_unix: u64,
    ) -> Result<String, String> {
        let current_floor = Self::current_session_id(paths, project)
            .map_or(1, |current| Self::row_id_floor(paths, project, &current));
        let first_row_id = current_floor.max(Self::project_row_floor(paths, project));
        Self::create_session_from(paths, project, title, created_unix, first_row_id)
    }

    /// Creates an empty session whose row ids start at `first_row_id` at least, without
    /// reading any log (the caller knows the next id of the log it is leaving).
    pub fn create_session_from(
        paths: &AppPaths,
        project: &ProjectId,
        title: Option<String>,
        created_unix: u64,
        first_row_id: u64,
    ) -> Result<String, String> {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sessions = paths.agent_sessions(project);
        let first_row_id = first_row_id.max(Self::project_row_floor(paths, project));
        let now = now_unix();
        let id = loop {
            let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // Zero-padded, so ids made within one second sort in creation order.
            let id = format!("s-{now:x}-{n:08x}-{:x}", std::process::id());
            if !sessions.join(&id).exists() {
                break id;
            }
        };
        let meta = SessionMeta {
            schema_version: SESSION_META_SCHEMA_VERSION,
            title: title.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()),
            created_unix,
            first_row_id,
            last_row_id: 0,
        };
        publish_dir(&sessions, &id, |dir| write_meta(dir, &meta))?;
        Ok(id)
    }
}

/// Builds `sessions/<id>` by filling a staging directory and renaming it into place.
fn publish_dir(
    sessions: &Path,
    id: &str,
    fill: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    std::fs::create_dir_all(sessions)
        .map_err(|e| format!("cannot create sessions directory: {e}"))?;
    let staging = sessions.join(format!("{STAGING_PREFIX}{id}"));
    remove_path(&staging).map_err(|e| format!("cannot clear staging directory: {e}"))?;
    std::fs::create_dir(&staging).map_err(|e| format!("cannot create staging directory: {e}"))?;
    let built = fill(&staging).and_then(|()| {
        retry_denied(|| std::fs::rename(&staging, sessions.join(id)))
            .map_err(|e| format!("cannot publish session `{id}`: {e}"))
    });
    match &built {
        Ok(()) => sync_dir_best_effort(sessions),
        Err(_) => {
            let _ = remove_path(&staging);
        }
    }
    built
}

/// Removes a file or a directory tree; a missing path is fine.
fn remove_path(path: &Path) -> std::io::Result<()> {
    let result = retry_denied(|| match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) => Err(e),
    });
    match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Removes the staging directories a crash left behind while a session was being created.
pub(super) fn remove_staging(sessions: &Path) -> Result<(), String> {
    let entries = match std::fs::read_dir(sessions) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("cannot read sessions directory: {e}")),
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(STAGING_PREFIX)
        {
            remove_path(&entry.path())
                .map_err(|e| format!("cannot clear staging directory: {e}"))?;
        }
    }
    Ok(())
}

fn write_meta(dir: &Path, meta: &SessionMeta) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(meta)
        .map_err(|e| format!("cannot serialize session meta: {e}"))?;
    write_atomic(&dir.join(META_FILE), &bytes)
}

/// A manifest of the current schema; `None` for a missing, unparsable or other-version file.
pub(super) fn read_manifest(path: &Path) -> Option<SessionManifest> {
    super::parse_manifest(&std::fs::read_to_string(path).ok()?, path).ok()
}

/// A meta file of the current schema; `None` for a missing, unparsable, other-version or
/// unknown-field file, which is then neither used nor rewritten.
pub(super) fn read_meta(path: &Path) -> Option<SessionMeta> {
    let meta: SessionMeta = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    (meta.schema_version == SESSION_META_SCHEMA_VERSION).then_some(meta)
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use super::*;

    #[test]
    fn a_denied_operation_is_retried_until_it_succeeds() {
        let mut calls = 0;
        let result = retry_denied_after(Duration::from_millis(1), || {
            calls += 1;
            if calls < 4 {
                Err(Error::from(ErrorKind::PermissionDenied))
            } else {
                Ok(calls)
            }
        });
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn a_persistent_denial_gives_up_after_a_bounded_number_of_attempts() {
        let mut calls = 0;
        let result: std::io::Result<()> = retry_denied_after(Duration::from_millis(1), || {
            calls += 1;
            Err(Error::from(ErrorKind::PermissionDenied))
        });
        assert_eq!(result.unwrap_err().kind(), ErrorKind::PermissionDenied);
        assert_eq!(calls, RENAME_ATTEMPTS);
    }

    #[test]
    fn other_errors_are_not_retried() {
        let mut calls = 0;
        let result: std::io::Result<()> = retry_denied_after(Duration::from_millis(1), || {
            calls += 1;
            Err(Error::from(ErrorKind::NotFound))
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}
