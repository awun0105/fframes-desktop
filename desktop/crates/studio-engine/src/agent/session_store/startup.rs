//! What opening a project's agent workflow does to the stored sessions before the actor owns
//! the log: migrate the single-session layout, make sure a session exists and name its log.

use std::path::PathBuf;

use studio_project::ProjectId;

use super::SessionStore;
use crate::app_paths::AppPaths;

/// Where the actor starts, and what the user should be told about getting there.
#[derive(Debug, Clone)]
pub struct Startup {
    /// The log to open: the current session's (the legacy project log only when the migration
    /// failed and left the old files in place).
    pub log: PathBuf,
    /// Row ids of the log start here at least (see `RowStore::open_with_floor`).
    pub row_floor: u64,
    /// Warnings to show as notice rows: what could not be migrated or created.
    pub notices: Vec<String>,
}

impl SessionStore {
    /// Runs [`SessionStore::migrate_legacy`] and creates the project's first session when it
    /// has none, so every conversation lives in a listed session from the start.
    ///
    /// Blocks on file I/O (and copies logs on the first run of a legacy project): call it where
    /// the workflow is opened, off the UI thread, before any actor opens the log. A failure
    /// never stops the workflow from opening; it is logged on stderr and returned as `notices`.
    /// `imported` and `failed` migration items are logged as well.
    pub fn prepare(paths: &AppPaths, project: &ProjectId) -> Startup {
        let mut notices = Vec::new();
        match Self::migrate_legacy(paths, project) {
            Ok(report) => {
                for id in &report.imported {
                    eprintln!(
                        "agent sessions: imported a legacy log written after its migration as `{id}`"
                    );
                }
                for path in &report.kept_legacy {
                    eprintln!(
                        "agent sessions: legacy file {} could not be removed",
                        path.display()
                    );
                }
                for (item, reason) in &report.failed {
                    eprintln!("agent sessions: could not migrate `{item}`: {reason}");
                }
                if let Some((item, reason)) = report.failed.first() {
                    notices.push(format!(
                        "{} earlier conversation(s) could not be moved into the session history and stay where they were ({item}: {reason}). Another attempt is made the next time the project opens.",
                        report.failed.len()
                    ));
                }
            }
            Err(error) => {
                eprintln!("agent sessions: migration failed: {error}");
                notices.push(format!(
                    "Earlier conversations could not be moved into the session history ({error}). Another attempt is made the next time the project opens."
                ));
            }
        }

        // Legacy files that are still there were not migrated: keep serving them rather than
        // hiding that conversation behind a new empty session.
        let legacy_left = paths.legacy_conversation_log(project).is_file()
            || paths.agent_session_manifest(project).is_file();
        if Self::current_session_id(paths, project).is_none() && !legacy_left {
            match Self::create_session(paths, project, None) {
                Ok(id) => {
                    if let Err(error) = Self::set_current(paths, project, &id) {
                        eprintln!("agent sessions: cannot select the first session: {error}");
                    }
                }
                Err(error) => {
                    eprintln!("agent sessions: cannot create the first session: {error}");
                    notices.push(format!(
                        "A session could not be created for this conversation ({error}); it is saved in the project's single conversation log."
                    ));
                }
            }
        }

        let log = Self::current_log_path(paths, project);
        Startup {
            log,
            row_floor: Self::project_row_floor(paths, project),
            notices,
        }
    }
}
