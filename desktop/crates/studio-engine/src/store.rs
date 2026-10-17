use crate::{EngineError, ProjectState, edit_transaction::TaskRevisionRecord};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub location: PathBuf,
    pub name: String,
    pub state: ProjectState,
    pub draft: Option<PathBuf>,
    pub sdk_path: Option<PathBuf>,
}

/// Schema version of the history database, the only one this Studio reads or writes.
pub const SCHEMA_VERSION: u32 = 2;

const SCHEMA: &str = "CREATE TABLE projects (id TEXT PRIMARY KEY, location TEXT NOT NULL, record TEXT NOT NULL, opened INTEGER NOT NULL, recent INTEGER NOT NULL DEFAULT 1);
     CREATE TABLE task_revisions (project TEXT NOT NULL, tx TEXT NOT NULL, position INTEGER NOT NULL, record TEXT NOT NULL, PRIMARY KEY (project, tx));";

/// Owned by the background controller/serialized command queue, never the GPUI render thread.
pub struct Store {
    connection: Connection,
    read_only: bool,
}
impl Store {
    /// Opens the history database, creating it when the file does not exist yet. A
    /// database of any other version (or without a version) is refused and left as it is.
    pub fn open(path: &Path) -> Result<Self, EngineError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        if Self::is_fresh(&connection)? {
            // Another opener may create it first: look again once the write lock is held.
            connection.execute_batch("BEGIN IMMEDIATE")?;
            if Self::is_fresh(&connection)? {
                connection
                    .execute_batch(&format!("{SCHEMA} PRAGMA user_version={SCHEMA_VERSION};"))?;
            }
            connection.execute_batch("COMMIT")?;
        }
        Self::check_format(&connection, path)?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self {
            connection,
            read_only: false,
        })
    }
    /// No version and no tables: a database file SQLite just created.
    fn is_fresh(connection: &Connection) -> Result<bool, EngineError> {
        let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let tables: u32 =
            connection.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))?;
        Ok(version == 0 && tables == 0)
    }
    fn check_format(connection: &Connection, path: &Path) -> Result<(), EngineError> {
        let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let what = format!("{} (history database)", path.display());
        match version.cmp(&SCHEMA_VERSION) {
            std::cmp::Ordering::Equal => Ok(()),
            std::cmp::Ordering::Greater => Err(EngineError::NewerFormat {
                what,
                found: version,
                supported: SCHEMA_VERSION,
            }),
            std::cmp::Ordering::Less => Err(EngineError::OlderFormat {
                what,
                found: version,
                supported: SCHEMA_VERSION,
            }),
        }
    }
    /// Refuses the open read-only database unless it is the current version, without
    /// writing anything. A file without tables counts as current (nothing to read).
    pub fn ensure_current(&self, path: &Path) -> Result<(), EngineError> {
        if Self::is_fresh(&self.connection)? {
            return Ok(());
        }
        Self::check_format(&self.connection, path)
    }
    /// Opens any existing database for inspection only: no migration, no backup, no
    /// write. Works for databases written by a newer Studio.
    pub fn open_read_only(path: &Path) -> Result<Self, EngineError> {
        let connection = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.busy_timeout(Duration::from_secs(5))?;
        Ok(Self {
            connection,
            read_only: true,
        })
    }
    /// `PRAGMA user_version` of the open database.
    pub fn version(&self) -> Result<u32, EngineError> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }
    fn writable(&self) -> Result<(), EngineError> {
        if self.read_only {
            return Err(EngineError::Diagnostic(
                "History database is open read-only for diagnostics; nothing was changed".into(),
            ));
        }
        Ok(())
    }
    pub fn get(&self, id: &studio_project::ProjectId) -> Result<Option<Record>, EngineError> {
        let json: Option<String> = self
            .connection
            .query_row(
                "SELECT record FROM projects WHERE id=?",
                [String::from(id.clone())],
                |r| r.get(0),
            )
            .optional()?;
        json.map(|j| serde_json::from_str(&j).map_err(EngineError::from))
            .transpose()
    }
    pub fn save(&mut self, record: &Record) -> Result<(), EngineError> {
        self.writable()?;
        let tx = self.connection.transaction()?;
        tx.execute("INSERT INTO projects(id,location,record,opened,recent) VALUES(?1,?2,?3,unixepoch(),1) ON CONFLICT(id) DO UPDATE SET location=excluded.location,record=excluded.record,opened=excluded.opened,recent=1", params![String::from(record.state.project().clone()), record.location.to_string_lossy(), serde_json::to_string(record)?])?;
        tx.commit()?;
        Ok(())
    }
    pub fn recents(&self) -> Result<Vec<Record>, EngineError> {
        let mut statement = self.connection.prepare(
            "SELECT record FROM projects WHERE recent=1 ORDER BY opened DESC, id LIMIT 50",
        )?;
        let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    /// Removes only the recent-list entry, never history, projects or assets.
    pub fn remove_recent(&mut self, id: &studio_project::ProjectId) -> Result<(), EngineError> {
        self.writable()?;
        self.connection.execute(
            "UPDATE projects SET recent=0 WHERE id=?",
            [String::from(id.clone())],
        )?;
        Ok(())
    }
    /// Committed task revisions of `project` in commit order. The projection of the
    /// task journal, which stays the source of truth.
    pub fn task_revisions(
        &self,
        project: &studio_project::ProjectId,
    ) -> Result<Vec<TaskRevisionRecord>, EngineError> {
        let mut statement = self
            .connection
            .prepare("SELECT record FROM task_revisions WHERE project=? ORDER BY position")?;
        let rows =
            statement.query_map([String::from(project.clone())], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    /// Replaces the whole projection of `project` (rebuild from the journal).
    pub fn replace_task_revisions(
        &mut self,
        project: &studio_project::ProjectId,
        records: &[TaskRevisionRecord],
    ) -> Result<(), EngineError> {
        self.writable()?;
        let tx = self.connection.transaction()?;
        let id = String::from(project.clone());
        tx.execute("DELETE FROM task_revisions WHERE project=?", [&id])?;
        for (position, record) in records.iter().enumerate() {
            tx.execute(
                "INSERT INTO task_revisions(project,tx,position,record) VALUES(?1,?2,?3,?4)",
                params![
                    id,
                    record.id,
                    position as i64,
                    serde_json::to_string(record)?
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database_with(path: &Path, version: u32) -> Connection {
        let original = Connection::open(path).unwrap();
        original
            .execute_batch(
                "CREATE TABLE projects (id TEXT PRIMARY KEY, location TEXT NOT NULL, record TEXT NOT NULL, opened INTEGER NOT NULL, recent INTEGER NOT NULL DEFAULT 1);
                 INSERT INTO projects VALUES ('p','/x','{}',1,1);",
            )
            .unwrap();
        original
            .pragma_update(None, "user_version", version)
            .unwrap();
        original
    }

    #[test]
    fn a_missing_database_is_created_at_the_current_version() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("studio.sqlite3");
        let store = Store::open(&path).unwrap();
        assert_eq!(store.version().unwrap(), SCHEMA_VERSION);
        assert!(store.recents().unwrap().is_empty());
        let id = studio_project::ProjectId::try_from("project-1".to_owned()).unwrap();
        assert!(store.task_revisions(&id).unwrap().is_empty());
        drop(store);
        // Reopening a current database changes nothing and creates nothing.
        assert!(Store::open(&path).unwrap().recents().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_database_of_an_older_version_is_refused_and_left_byte_identical() {
        // Version 1 (before the task-revision projection) and a database that never had
        // a version at all.
        for version in [0, 1] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("studio.sqlite3");
            drop(database_with(&path, version));
            let before = std::fs::read(&path).unwrap();
            let error = Store::open(&path).err().unwrap();
            assert!(
                matches!(
                    error,
                    EngineError::OlderFormat { found, supported: SCHEMA_VERSION, .. }
                        if found == version
                ),
                "{error}"
            );
            let message = error.to_string();
            assert!(message.contains("studio.sqlite3"), "{message}");
            assert!(message.contains("older Studio"), "{message}");
            assert_eq!(std::fs::read(&path).unwrap(), before, "version {version}");
            // No backup, journal or recreated table appeared, and the data is still there.
            assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
            let diagnostic = Store::open_read_only(&path).unwrap();
            assert_eq!(diagnostic.version().unwrap(), version);
            assert!(diagnostic.ensure_current(&path).is_err());
            assert_eq!(
                diagnostic
                    .connection
                    .query_row("SELECT record FROM projects WHERE id='p'", [], |r| r
                        .get::<_, String>(0))
                    .unwrap(),
                "{}"
            );
            drop(diagnostic);
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn a_newer_database_is_refused_for_writing_but_readable_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("studio.sqlite3");
        drop(database_with(&path, 99));
        let before = std::fs::read(&path).unwrap();
        let error = Store::open(&path).err().unwrap();
        assert!(
            matches!(
                error,
                EngineError::NewerFormat {
                    found: 99,
                    supported: SCHEMA_VERSION,
                    ..
                }
            ),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut diagnostic = Store::open_read_only(&path).unwrap();
        assert_eq!(diagnostic.version().unwrap(), 99);
        let id = studio_project::ProjectId::try_from("project-1".to_owned()).unwrap();
        assert!(diagnostic.replace_task_revisions(&id, &[]).is_err());
        assert!(diagnostic.remove_recent(&id).is_err());
        drop(diagnostic);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }
}
