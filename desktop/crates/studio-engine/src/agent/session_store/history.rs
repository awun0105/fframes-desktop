//! Listing and loading stored sessions. Pure reads over the store (apart from repairing a bad
//! `current` pointer): nothing here creates a directory, repairs a log or touches the actor.
//!
//! [`SessionStore::load_session`] is the seam the restore wiring calls to reopen a session: it
//! returns the stored rows (the single source of rows; replayed `session/update`
//! notifications from `session/load` must be dropped by the driver side) and the manifest that
//! says how to resume it (`SessionManifest::is_resumable`, `wire_session_id`).

use std::sync::Arc;

use studio_project::ProjectId;

use super::layout::{self, is_valid_session_id};
use super::{SessionManifest, SessionStore, truncate_utf8};
use crate::agent::log;
use crate::agent::model::{MAX_RESIDENT_ROWS, OutcomeKind, Row, RowKind, UserSource};
use crate::app_paths::AppPaths;

/// Bytes of prompt text a summary keeps for searching.
pub const MAX_PROMPT_TEXT_BYTES: usize = 8 * 1024;
/// Characters of the first prompt used as a title.
const TITLE_CHARS: usize = 80;
pub const UNTITLED: &str = "Untitled session";
pub const UNREADABLE: &str = "Unreadable session";

/// One line of the history list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: String,
    /// The stored title, else the first prompt's first line, else [`UNTITLED`].
    pub title: String,
    /// Provider that ran the session; `None` before a wire session was ever opened.
    pub agent: Option<String>,
    pub created_unix: u64,
    /// Latest of the manifest update and the newest row.
    pub updated_unix: u64,
    /// Kind of the newest outcome card.
    pub outcome: Option<OutcomeKind>,
    /// The user's briefs and clarifications joined by newlines, bounded by
    /// [`MAX_PROMPT_TEXT_BYTES`]. Automatic repair prompts are not included.
    pub prompt_text: String,
    pub rows: usize,
    pub is_current: bool,
    pub resumable: bool,
    /// The log could not be read (it exists but fails to open or read). The title says so.
    pub unreadable: bool,
}

/// A stored session, ready to be shown (and resumed by the wiring).
#[derive(Debug, Clone)]
pub struct StoredSession {
    pub id: String,
    pub manifest: Option<SessionManifest>,
    /// The newest rows, ascending by id, at most the requested limit.
    pub rows: Vec<Arc<Row>>,
    /// Whether rows older than `rows` exist in the log.
    pub has_older: bool,
}

impl SessionStore {
    /// All stored sessions, most recently updated first (ties: newest id first). A session
    /// whose files cannot be read still appears, with what could be read (see
    /// [`SessionSummary::unreadable`]). A project without sessions yields an empty list.
    ///
    /// Reads every log in full, so call it off the UI thread and not on every keystroke.
    pub fn list_sessions(
        paths: &AppPaths,
        project: &ProjectId,
    ) -> Result<Vec<SessionSummary>, String> {
        let entries = match std::fs::read_dir(paths.agent_sessions(project)) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("cannot read sessions directory: {e}")),
        };
        let current = Self::current_session_id(paths, project);
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_valid_session_id(&name) && entry.path().is_dir() {
                let is_current = current.as_deref() == Some(name.as_str());
                out.push(summarize(paths, project, name, is_current));
            }
        }
        out.sort_by(|a, b| {
            b.updated_unix
                .cmp(&a.updated_unix)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(out)
    }

    /// The newest `limit` rows (clamped to the resident window) and the manifest of a stored
    /// session. Errors for an id that is invalid or names no session.
    pub fn load_session(
        paths: &AppPaths,
        project: &ProjectId,
        session: &str,
        limit: usize,
    ) -> Result<StoredSession, String> {
        if !is_valid_session_id(session) {
            return Err(format!("invalid session id `{session}`"));
        }
        if !paths.agent_session_dir(project, session).is_dir() {
            return Err(format!("unknown session `{session}`"));
        }
        let mut rows = log::read_rows(&layout::log_file(paths, project, session))
            .map_err(|e| e.to_string())?;
        let limit = limit.clamp(1, MAX_RESIDENT_ROWS);
        let has_older = rows.len() > limit;
        rows.drain(..rows.len().saturating_sub(limit));
        Ok(StoredSession {
            id: session.to_owned(),
            manifest: layout::read_manifest(&layout::manifest_file(paths, project, session)),
            rows: rows.into_iter().map(Arc::new).collect(),
            has_older,
        })
    }
}

fn summarize(
    paths: &AppPaths,
    project: &ProjectId,
    id: String,
    is_current: bool,
) -> SessionSummary {
    let manifest = layout::read_manifest(&layout::manifest_file(paths, project, &id));
    let meta = layout::read_meta(&layout::meta_file(paths, project, &id));
    let (rows, unreadable) = match log::read_rows(&layout::log_file(paths, project, &id)) {
        Ok(rows) => (rows, false),
        Err(_) => (Vec::new(), true),
    };

    let mut prompts: Vec<&str> = Vec::new();
    let mut outcome = None;
    for row in &rows {
        match &row.kind {
            RowKind::User { text, source } if *source != UserSource::Repair => prompts.push(text),
            RowKind::Outcome(card) => outcome = Some(card.kind),
            _ => {}
        }
    }
    let mut prompt_text = prompts.join("\n");
    truncate_utf8(&mut prompt_text, MAX_PROMPT_TEXT_BYTES);

    let stored_title = meta
        .as_ref()
        .and_then(|m| m.title.as_deref())
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let title = match stored_title {
        Some(title) => title.to_owned(),
        None => prompts
            .iter()
            .flat_map(|p| p.lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map_or_else(
                || (if unreadable { UNREADABLE } else { UNTITLED }).to_owned(),
                shorten,
            ),
    };

    let manifest_time = |value: fn(&SessionManifest) -> &str| {
        manifest
            .as_ref()
            .and_then(|m| value(m).parse::<u64>().ok())
            .unwrap_or(0)
    };
    let first_row = rows.first().map_or(0, |r| r.at_unix);
    let last_row = rows.iter().map(|r| r.at_unix).max().unwrap_or(0);
    let created_unix = [
        meta.as_ref().map_or(0, |m| m.created_unix),
        manifest_time(|m| &m.created_at),
        first_row,
    ]
    .into_iter()
    .find(|t| *t > 0)
    .unwrap_or(0);
    let updated_unix = created_unix
        .max(manifest_time(|m| &m.updated_at))
        .max(last_row);

    SessionSummary {
        id,
        title,
        agent: manifest.as_ref().map(|m| m.provider_id.clone()),
        created_unix,
        updated_unix,
        outcome,
        prompt_text,
        rows: rows.len(),
        is_current,
        resumable: manifest.as_ref().is_some_and(SessionManifest::is_resumable),
        unreadable,
    }
}

fn shorten(line: &str) -> String {
    if line.chars().count() <= TITLE_CHARS {
        return line.to_owned();
    }
    let mut short: String = line.chars().take(TITLE_CHARS - 1).collect();
    short.push('\u{2026}');
    short
}
