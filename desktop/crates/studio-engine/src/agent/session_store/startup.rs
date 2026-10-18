//! What opening a project's agent workflow does to the stored sessions before the actor owns
//! the log: make sure a session exists and name its log.

use std::path::PathBuf;

use studio_project::ProjectId;

use super::SessionStore;
use super::layout;
use crate::app_paths::AppPaths;

/// Where the actor starts.
#[derive(Debug, Clone)]
pub struct Startup {
    /// The log to open: the current session's.
    pub log: PathBuf,
    /// Row ids of the log start here at least (see `RowStore::open_with_floor`).
    pub row_floor: u64,
}

impl SessionStore {
    /// Creates the project's first session when it has none, so every conversation lives in a
    /// listed session from the start. Files of an earlier single-session layout are neither
    /// read nor changed: a project that only has those starts with an empty history.
    ///
    /// Blocks on file I/O: call it where the workflow is opened, off the UI thread, before any
    /// actor opens the log. Fails when no session can be created, because then there is nowhere
    /// to keep the conversation.
    pub fn prepare(paths: &AppPaths, project: &ProjectId) -> Result<Startup, String> {
        if let Err(error) = layout::remove_staging(&paths.agent_sessions(project)) {
            eprintln!("agent sessions: {error}");
        }
        if Self::current_session_id(paths, project).is_none() {
            let id = Self::create_session(paths, project, None)
                .map_err(|error| format!("cannot create the first session: {error}"))?;
            Self::set_current(paths, project, &id)
                .map_err(|error| format!("cannot select the first session: {error}"))?;
        }
        let log = Self::current_log_path(paths, project)
            .ok_or_else(|| "the project has no current session".to_owned())?;
        Ok(Startup {
            log,
            row_floor: Self::project_row_floor(paths, project),
        })
    }
}
