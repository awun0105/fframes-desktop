//! On-disk layout of the stored agent sessions and the one-time migration into it.
//!
//! ```text
//! agent/sessions/
//!   current                  session id the single-session API serves
//!   <id>/manifest.json       [`SessionManifest`], schema version 2 (absent until a wire session exists)
//!   <id>/meta.json           [`SessionMeta`]: stored title, creation time, first row id
//!   <id>/conversation.jsonl  the row log of that session
//! ```
//!
//! Attachment bytes are never stored: a `User` row carries text only.
//!
//! A project counts as migrated as soon as `sessions/` holds a session directory. From then on
//! the single-session API does not fall back to the legacy files: a missing, unreadable or
//! dangling `current` pointer is repaired by pointing it at the most recently updated session.
//! The one exception is a legacy conversation that was not migrated (its files are still there
//! and no `legacy` session exists): no session is picked in its place, so the legacy files keep
//! being served until a later migration succeeds.
//!
//! # How the workflow uses it
//!
//! `AgentWorkflow::open` runs `SessionStore::prepare` (which calls
//! [`SessionStore::migrate_legacy`] before any log is open: Windows will not remove an open
//! legacy log) and opens the log at [`SessionStore::current_log_path`] with
//! `RowStore::open_with_floor`, so row ids stay unique across the sessions of a project. New
//! chat and opening a stored session flush the open log, then use
//! [`SessionStore::create_session`] / [`SessionStore::set_current`], never `RowStore::start_new`,
//! which refuses a log that lives in a session directory.
//!
//! [`SessionStore::load`], [`SessionStore::save`] and [`SessionStore::clear`] follow the
//! `current` pointer, so they need no change. `clear` removes only the current session's
//! `manifest.json`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use studio_project::ProjectId;

pub use crate::app_paths::is_valid_session_id;

use super::{SESSION_MANIFEST_SCHEMA_VERSION, SessionManifest, SessionStore, write_atomic};
use crate::agent::log;
use crate::app_paths::AppPaths;

pub const SESSION_META_SCHEMA_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const META_FILE: &str = "meta.json";
const LOG_FILE: &str = "conversation.jsonl";
const STAGING_PREFIX: &str = ".staging-";
/// Session id of the single session that existed before the multi-session layout.
pub const LEGACY_SESSION_ID: &str = "legacy";
const MAX_ID_LEN: usize = 64;
const RENAME_ATTEMPTS: u32 = 6;
const RENAME_FIRST_DELAY: Duration = Duration::from_millis(25);

fn default_first_row_id() -> u64 {
    1
}

/// Small facts about a session that its manifest (which only exists once a wire session is
/// open) and its log cannot give.
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
    /// Fingerprint of the legacy files this session was imported from, so legacy files that
    /// cannot be deleted are not imported again on every start.
    #[serde(default)]
    pub source_hash: Option<String>,
}

/// What [`SessionStore::migrate_legacy`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    /// Session ids created from legacy files by this call.
    pub migrated: Vec<String>,
    /// Sessions created because a legacy file differed from the copy migrated earlier (the
    /// legacy log was written again after a migration): the new content is kept as its own session.
    pub imported: Vec<String>,
    /// Legacy files removed because the new layout holds an identical copy.
    pub removed_legacy: Vec<PathBuf>,
    /// Legacy files that could not be removed.
    pub kept_legacy: Vec<PathBuf>,
    /// Items that could not be migrated, with the reason. They stay where they are and the
    /// other items are unaffected.
    pub failed: Vec<(String, String)>,
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

/// Legacy single-session files exist and no `legacy` session holds them.
fn legacy_files_unmigrated(paths: &AppPaths, project: &ProjectId) -> bool {
    (paths.legacy_conversation_log(project).is_file()
        || paths.agent_session_manifest(project).is_file())
        && !paths.agent_session_dir(project, LEGACY_SESSION_ID).is_dir()
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
    /// session directory (not migrated yet). A missing, unreadable or dangling pointer is
    /// repaired: it is rewritten to name the most recently updated session (ties: greatest
    /// id), and the repair is reported on stderr.
    pub fn current_session_id(paths: &AppPaths, project: &ProjectId) -> Option<String> {
        if let Some(id) = Self::pointer_target(paths, project) {
            return Some(id);
        }
        // The legacy conversation did not make it into a session: never point at another
        // session in its place, the legacy files keep being served until it does.
        if legacy_files_unmigrated(paths, project) {
            return None;
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

    /// The log of the current session; the legacy project log while the project has not been
    /// migrated. This is what `present::log_path` is to become.
    pub fn current_log_path(paths: &AppPaths, project: &ProjectId) -> PathBuf {
        match Self::current_session_id(paths, project) {
            Some(session) => log_file(paths, project, &session),
            None => paths.legacy_conversation_log(project),
        }
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
            source_hash: None,
        };
        publish_dir(&sessions, &id, |dir| write_meta(dir, &meta))?;
        Ok(id)
    }

    /// One-time, idempotent move of the single-session layout into the multi-session one.
    ///
    /// Migrates `<project>/conversation.jsonl` plus `agent/session-manifest.json` as session
    /// [`LEGACY_SESSION_ID`], and every `conversation-<stamp>.jsonl` that `RowStore::start_new`
    /// archived as session `archive-<stamp>`. Then, if there is no valid current pointer,
    /// points `current` at the legacy session (when it was migrated by this call) or repairs it.
    ///
    /// Crash safety: each session is built in `sessions/.staging-<id>` and renamed into place
    /// (retrying a denied rename), so a session directory is either absent or complete; the
    /// `current` pointer is written after that; on Unix the directory entries are flushed
    /// before any legacy file goes. Legacy files are removed last, and only when the new copy
    /// is identical. A legacy file that differs from the copy of an earlier migration (it was
    /// written again) is imported as a new session `legacy-<unix-seconds>`, then removed. A
    /// manifest that cannot be parsed (or is from a newer schema) is copied byte for byte, so
    /// `load` answers the same before and after. One item that fails does not stop the others;
    /// see [`MigrationReport::failed`].
    ///
    /// Not guarded against a concurrent call for the same project. Call it before the workflow
    /// opens the log (see the module docs).
    pub fn migrate_legacy(
        paths: &AppPaths,
        project: &ProjectId,
    ) -> Result<MigrationReport, String> {
        let sessions = paths.agent_sessions(project);
        remove_staging(&sessions)?;

        let legacy_manifest = paths.agent_session_manifest(project);
        let legacy_log = paths.legacy_conversation_log(project);
        let mut items = Vec::new();
        if legacy_manifest.is_file() || legacy_log.is_file() {
            items.push(LegacyItem {
                id: LEGACY_SESSION_ID.to_owned(),
                log: legacy_log.is_file().then_some(legacy_log),
                manifest: legacy_manifest.is_file().then_some(legacy_manifest),
            });
        }
        items.extend(archived_logs(&paths.project(project))?);

        let mut report = MigrationReport::default();
        let mut ready = Vec::new();
        for item in &items {
            if !paths.agent_session_dir(project, &item.id).exists() {
                match publish_dir(&sessions, &item.id, |dir| stage(dir, item)) {
                    Ok(()) => report.migrated.push(item.id.clone()),
                    Err(e) => {
                        report.failed.push((item.id.clone(), e));
                        continue;
                    }
                }
            }
            ready.push(item);
        }

        if Self::pointer_target(paths, project).is_none() {
            if ready.iter().any(|item| item.id == LEGACY_SESSION_ID) {
                Self::set_current(paths, project, LEGACY_SESSION_ID)?;
            } else {
                let _ = Self::current_session_id(paths, project);
            }
        }
        sync_dir_best_effort(&sessions);

        for item in ready {
            settle_item(paths, project, item, &mut report);
        }
        Ok(report)
    }
}

/// A legacy session's files as found before migration.
struct LegacyItem {
    id: String,
    log: Option<PathBuf>,
    manifest: Option<PathBuf>,
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

fn remove_staging(sessions: &Path) -> Result<(), String> {
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

/// Session id of an archived log: `archive-<stamp>`, shortened with a hash of the stamp when
/// that would not fit an id.
fn archive_id(stamp: &str) -> String {
    let id = format!("archive-{stamp}");
    if id.len() <= MAX_ID_LEN {
        return id;
    }
    format!("archive-{}-{:016x}", &stamp[..30], fnv1a(stamp.bytes()))
}

fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

/// Identifies the content of a legacy item's files (`None` when one cannot be read).
fn fingerprint(item: &LegacyItem) -> Option<String> {
    let mut bytes = Vec::new();
    for path in legacy_files(item) {
        bytes.extend(std::fs::read(path).ok()?);
        bytes.push(0);
    }
    Some(format!(
        "{:016x}-{}",
        fnv1a(bytes.iter().copied()),
        bytes.len()
    ))
}

/// `conversation-<stamp>[-n].jsonl` files next to the project log, oldest first.
fn archived_logs(project_dir: &Path) -> Result<Vec<LegacyItem>, String> {
    let entries = match std::fs::read_dir(project_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot read project directory: {e}")),
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stamp) = name
            .strip_prefix("conversation-")
            .and_then(|rest| rest.strip_suffix(".jsonl"))
        else {
            continue;
        };
        if stamp.is_empty() || !stamp.chars().all(|c| c.is_ascii_digit() || c == '-') {
            continue;
        }
        items.push(LegacyItem {
            id: archive_id(stamp),
            log: Some(entry.path()),
            manifest: None,
        });
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(items)
}

fn stage(dir: &Path, item: &LegacyItem) -> Result<(), String> {
    if let Some(source) = &item.log {
        let target = dir.join(LOG_FILE);
        std::fs::copy(source, &target).map_err(|e| format!("cannot copy conversation log: {e}"))?;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&target)
            .and_then(|file| file.sync_all())
            .map_err(|e| format!("cannot copy conversation log: {e}"))?;
    }
    let mut created = 0;
    if let Some(path) = &item.manifest {
        if let Some(manifest) = read_manifest(path) {
            created = manifest.created_at.parse().unwrap_or(0);
            let bytes = serde_json::to_vec_pretty(&upgraded(manifest))
                .map_err(|e| format!("cannot serialize session manifest: {e}"))?;
            write_atomic(&dir.join(MANIFEST_FILE), &bytes)?;
        } else {
            // Corrupt or from a newer schema: keep the bytes so `load` answers as before.
            let bytes =
                std::fs::read(path).map_err(|e| format!("cannot read session manifest: {e}"))?;
            write_atomic(&dir.join(MANIFEST_FILE), &bytes)?;
        }
    }
    let rows = log::read_rows(&dir.join(LOG_FILE)).unwrap_or_default();
    if created == 0 {
        created = rows.first().map_or(0, |row| row.at_unix);
    }
    write_meta(
        dir,
        &SessionMeta {
            schema_version: SESSION_META_SCHEMA_VERSION,
            title: None,
            created_unix: created,
            first_row_id: 1,
            last_row_id: rows.iter().map(|row| row.id.0).max().unwrap_or(0),
            source_hash: fingerprint(item),
        },
    )
}

fn write_meta(dir: &Path, meta: &SessionMeta) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(meta)
        .map_err(|e| format!("cannot serialize session meta: {e}"))?;
    write_atomic(&dir.join(META_FILE), &bytes)
}

/// A manifest of this or an older schema; `None` for a missing, unparsable or newer file.
pub(super) fn read_manifest(path: &Path) -> Option<SessionManifest> {
    let text = std::fs::read_to_string(path).ok()?;
    let manifest: SessionManifest = serde_json::from_str(&text).ok()?;
    (manifest.schema_version <= SESSION_MANIFEST_SCHEMA_VERSION).then_some(manifest)
}

pub(super) fn read_meta(path: &Path) -> Option<SessionMeta> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn upgraded(mut manifest: SessionManifest) -> SessionManifest {
    manifest.schema_version = SESSION_MANIFEST_SCHEMA_VERSION;
    manifest
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    matches!((std::fs::read(a), std::fs::read(b)), (Ok(a), Ok(b)) if a == b)
}

/// Whether the copy of a legacy manifest holds the same data: parsed and upgraded when the
/// legacy file parses, byte for byte otherwise.
fn manifest_matches(legacy: &Path, copy: &Path) -> bool {
    match read_manifest(legacy) {
        Some(old) => read_manifest(copy).is_some_and(|new| new == upgraded(old)),
        None => same_bytes(legacy, copy),
    }
}

fn copy_matches(item: &LegacyItem, dir: &Path) -> bool {
    item.log
        .as_deref()
        .is_none_or(|path| same_bytes(path, &dir.join(LOG_FILE)))
        && item
            .manifest
            .as_deref()
            .is_none_or(|path| manifest_matches(path, &dir.join(MANIFEST_FILE)))
}

/// Removes the legacy files of an item whose copy is identical, importing the item as a new
/// session first when the copy it was migrated to differs.
fn settle_item(
    paths: &AppPaths,
    project: &ProjectId,
    item: &LegacyItem,
    report: &mut MigrationReport,
) {
    if copy_matches(item, &paths.agent_session_dir(project, &item.id))
        || already_imported(paths, project, item)
    {
        remove_legacy(item, report);
        return;
    }
    let sessions = paths.agent_sessions(project);
    let unix = now_unix();
    let mut id = format!("{LEGACY_SESSION_ID}-{unix}");
    let mut n = 1;
    while sessions.join(&id).exists() {
        n += 1;
        id = format!("{LEGACY_SESSION_ID}-{unix}-{n}");
    }
    match publish_dir(&sessions, &id, |dir| stage(dir, item)) {
        Ok(()) => {
            report.imported.push(id.clone());
            if copy_matches(item, &paths.agent_session_dir(project, &id)) {
                remove_legacy(item, report);
            } else {
                report.kept_legacy.extend(legacy_files(item).cloned());
            }
        }
        Err(e) => report.failed.push((item.id.clone(), e)),
    }
}

/// A session was made from exactly these legacy files (they could not be removed then, so the
/// session has moved on since and no longer equals them).
fn already_imported(paths: &AppPaths, project: &ProjectId, item: &LegacyItem) -> bool {
    let Some(hash) = fingerprint(item) else {
        return false;
    };
    session_ids(paths, project).iter().any(|id| {
        read_meta(&meta_file(paths, project, id))
            .and_then(|meta| meta.source_hash)
            .is_some_and(|stored| stored == hash)
    })
}

fn legacy_files(item: &LegacyItem) -> impl Iterator<Item = &PathBuf> {
    item.log.iter().chain(item.manifest.iter())
}

fn remove_legacy(item: &LegacyItem, report: &mut MigrationReport) {
    for path in legacy_files(item) {
        if retry_denied(|| std::fs::remove_file(path)).is_ok() {
            report.removed_legacy.push(path.clone());
        } else {
            report.kept_legacy.push(path.clone());
        }
    }
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

    #[test]
    fn archive_ids_fit_the_id_limit_and_stay_distinct() {
        assert_eq!(archive_id("123-4"), "archive-123-4");
        let a = archive_id(&"7".repeat(80));
        let b = archive_id(&format!("{}8", "7".repeat(79)));
        assert!(is_valid_session_id(&a) && is_valid_session_id(&b));
        assert_ne!(a, b);
    }
}
