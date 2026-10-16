use crate::{
    EngineError,
    edit_transaction::{
        ConflictReport, OLDEST_TASK_FORMAT, Progress, TASK_FORMAT, TaskEvent, TaskRevisionRecord,
        TransactionIntent, TransactionKind,
    },
    store::Record,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
};
use studio_project::lifecycle::sync_directory;

const MAX_ENTRY_BYTES: usize = 1024 * 1024;

fn truncate_and_sync(path: &std::path::Path, length: u64) -> std::io::Result<()> {
    // The journal's append handle lacks the write-data permission Windows requires
    // for SetEndOfFile, so repair truncates through a dedicated write handle.
    let file = OpenOptions::new().write(true).open(path)?;
    file.set_len(length)?;
    file.sync_all()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    version: u32,
    sequence: u64,
    transaction: String,
    event: Event,
}
#[derive(Serialize, Deserialize)]
enum Event {
    Intent(Box<Record>),
    Commit,
}

pub struct Journal {
    path: PathBuf,
    file: File,
    sequence: u64,
    failed: bool,
}
pub struct Replay {
    pub committed: Option<Record>,
    pub pending: Option<Record>,
    pub quarantined_tail: Option<PathBuf>,
}
/// Reads a lifecycle journal. With `repair`, an incomplete last line is quarantined and
/// truncated (it was never durable); without it nothing is created, truncated or
/// rewritten and the tail is simply not part of the replay.
fn scan_lifecycle(
    path: &std::path::Path,
    file: &File,
    repair: bool,
) -> Result<(Replay, u64), EngineError> {
    let mut reader = BufReader::new(file.try_clone()?);
    let mut sequence = 0;
    let mut offset = 0;
    let mut pending: Option<(String, Record)> = None;
    let mut committed = None;
    let mut quarantined_tail = None;
    loop {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take(MAX_ENTRY_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            break;
        }
        if count > MAX_ENTRY_BYTES {
            return Err(EngineError::Diagnostic(format!(
                "{}: journal entry exceeds 1 MiB at byte {offset}; preserve and repair history",
                path.display()
            )));
        }
        if line.last() != Some(&b'\n') {
            if repair {
                let quarantine = path.with_extension(format!("tail-{}", uuid::Uuid::new_v4()));
                studio_project::lifecycle::atomic_write(&quarantine, &line)?;
                truncate_and_sync(path, offset)?;
                quarantined_tail = Some(quarantine);
            }
            break;
        }
        #[derive(Deserialize)]
        struct Header {
            version: u32,
        }
        if let Ok(header) = serde_json::from_slice::<Header>(&line)
            && header.version > 1
        {
            return Err(EngineError::NewerFormat {
                what: format!("{} (lifecycle journal)", path.display()),
                found: header.version,
                supported: 1,
            });
        }
        let entry: Entry = serde_json::from_slice(&line).map_err(|e| EngineError::Diagnostic(format!("{}: malformed interior journal at byte {offset}: {e}; preserve and repair the journal", path.display())))?;
        if entry.version != 1 || entry.sequence != sequence + 1 {
            return Err(EngineError::Diagnostic(format!(
                "{}: unsupported journal version or non-monotonic sequence; preserve and repair history",
                path.display()
            )));
        }
        sequence = entry.sequence;
        offset += count as u64;
        match entry.event {
            Event::Intent(record) => pending = Some((entry.transaction, *record)),
            Event::Commit => {
                let (transaction, record) = pending.take().ok_or_else(|| {
                    EngineError::Diagnostic("Journal commit without intent; repair history".into())
                })?;
                if transaction != entry.transaction {
                    return Err(EngineError::Diagnostic(
                        "Journal transaction mismatch; repair history".into(),
                    ));
                }
                committed = Some(record);
            }
        }
    }
    Ok((
        Replay {
            committed,
            pending: pending.map(|(_, r)| r),
            quarantined_tail,
        },
        sequence,
    ))
}

/// A handle every write to which fails, for the simulated full disk.
#[cfg(target_os = "linux")]
fn failing_writer(_journal: &std::path::Path) -> std::io::Result<File> {
    OpenOptions::new().write(true).open("/dev/full")
}

#[cfg(not(target_os = "linux"))]
fn failing_writer(journal: &std::path::Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(journal)
}

impl Journal {
    pub fn open(path: PathBuf) -> Result<(Self, Replay), EngineError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        let (replay, sequence) = scan_lifecycle(&path, &file, true)?;
        sync_directory(path.parent().unwrap())?;
        Ok((
            Self {
                path,
                file,
                sequence,
                failed: false,
            },
            replay,
        ))
    }
    /// Reads a journal without creating, truncating or quarantining anything: the
    /// non-mutating compatibility preflight. A missing file is an empty journal.
    pub fn inspect(path: &std::path::Path) -> Result<Replay, EngineError> {
        match File::open(path) {
            Ok(file) => Ok(scan_lifecycle(path, &file, false)?.0),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Replay {
                committed: None,
                pending: None,
                quarantined_tail: None,
            }),
            Err(e) => Err(e.into()),
        }
    }
    /// Whether an append or sync failed: the journal then refuses everything until the
    /// project is reopened (and a partial write recovered).
    pub fn is_failed(&self) -> bool {
        self.failed
    }
    /// Makes the next append fail like a full disk. For tests of the failure handling
    /// only. Linux redirects appends to `/dev/full` (`ENOSPC`); elsewhere there is no such
    /// device, so appends go to a read-only handle of the journal itself and fail
    /// (`EBADF`, `ERROR_ACCESS_DENIED`), leaving the file unchanged either way.
    #[doc(hidden)]
    pub fn simulate_full_disk(&mut self) -> std::io::Result<()> {
        self.file = failing_writer(&self.path)?;
        Ok(())
    }
    pub fn intent(&mut self, record: &Record) -> Result<String, EngineError> {
        let transaction = uuid::Uuid::new_v4().to_string();
        self.append(&transaction, Event::Intent(Box::new(record.clone())))?;
        Ok(transaction)
    }
    pub fn commit(&mut self, transaction: &str) -> Result<(), EngineError> {
        self.append(transaction, Event::Commit)
    }
    fn append(&mut self, transaction: &str, event: Event) -> Result<(), EngineError> {
        if self.failed {
            return Err(EngineError::Diagnostic("Journal write failed; repair disk/permissions, then close and reopen to recover before retrying".into()));
        }
        let next = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| EngineError::Diagnostic("Journal sequence exhausted".into()))?;
        let mut bytes = serde_json::to_vec(&Entry {
            version: 1,
            sequence: next,
            transaction: transaction.into(),
            event,
        })?;
        bytes.push(b'\n');
        if bytes.len() > MAX_ENTRY_BYTES {
            return Err(EngineError::Diagnostic(
                "Journal entry exceeds 1 MiB; shorten project metadata".into(),
            ));
        }
        let mut write = || -> Result<(), EngineError> {
            self.file.write_all(&bytes)?;
            self.file.sync_all()?;
            sync_directory(self.path.parent().unwrap())?;
            Ok(())
        };
        if let Err(error) = write() {
            // Continuing after a partial write would turn a recoverable tail into
            // interior corruption, or reuse a sequence whose sync result was unknown.
            self.failed = true;
            return Err(error);
        }
        self.sequence = next;
        Ok(())
    }
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

// ---- task journal (formats 2, 3 and 4) -------------------------------------------------
//
// `tasks.jsonl` records file-set transactions. Every line carries an explicit `format`
// (`TASK_FORMAT`, 4; format-2 and format-3 lines still replay, see `TASK_FORMAT`); the lifecycle
// journal above keeps its own version-1 records untouched, so previously written
// history stays readable. A line from a newer format is refused without truncating or
// rewriting anything.

const TASK_MAX_ENTRY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct TaskEntry {
    format: u32,
    sequence: u64,
    transaction: String,
    at: u64,
    event: TaskEvent,
}

#[derive(Deserialize)]
struct EntryHeader {
    format: u32,
}

/// How far a transaction got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxStatus {
    /// Intent journalled, no terminal event: recovery must roll it back.
    Pending,
    Committed(Box<TaskRevisionRecord>),
    RolledBack(String),
    /// Halted on unknown bytes; blocks new mutation until resolved.
    Conflicted(Box<ConflictReport>),
    Resolved(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayedTransaction {
    pub id: String,
    pub intent: TransactionIntent,
    pub progress: Vec<Progress>,
    pub status: TxStatus,
}

#[derive(Debug, Clone, Default)]
pub struct TaskReplay {
    pub transactions: Vec<ReplayedTransaction>,
    pub quarantined_tail: Option<PathBuf>,
    /// Bytes of an incomplete last line that a read-only inspection ignored.
    pub ignored_tail_bytes: usize,
}

impl TaskReplay {
    /// Committed *task* revisions in commit order: the source of the history projection.
    /// Studio preset mutations and migrations are committed transactions too, but never
    /// task history.
    pub fn committed(&self) -> Vec<TaskRevisionRecord> {
        self.committed_where(|kind| matches!(kind, TransactionKind::Apply | TransactionKind::Undo))
    }

    /// Committed Studio preset mutations in commit order.
    pub fn committed_presets(&self) -> Vec<TaskRevisionRecord> {
        self.committed_where(|kind| kind == TransactionKind::Preset)
    }

    /// Committed project migrations and their undos in commit order.
    pub fn committed_migrations(&self) -> Vec<TaskRevisionRecord> {
        self.committed_where(|kind| kind == TransactionKind::Migration)
    }

    fn committed_where(&self, kind: impl Fn(TransactionKind) -> bool) -> Vec<TaskRevisionRecord> {
        self.transactions
            .iter()
            .filter_map(|t| match &t.status {
                TxStatus::Committed(record) if kind(record.kind) => Some((**record).clone()),
                _ => None,
            })
            .collect()
    }

    pub fn unresolved_conflicts(&self) -> Vec<ConflictReport> {
        self.transactions
            .iter()
            .filter_map(|t| match &t.status {
                TxStatus::Conflicted(report) => Some((**report).clone()),
                _ => None,
            })
            .collect()
    }

    pub fn pending(&self) -> Vec<&ReplayedTransaction> {
        self.transactions
            .iter()
            .filter(|t| t.status == TxStatus::Pending)
            .collect()
    }

    fn apply(&mut self, entry: TaskEntry) -> Result<(), EngineError> {
        let bad = |what: &str| {
            EngineError::Diagnostic(format!(
                "Task journal: {what} (transaction {}); preserve and repair the journal",
                entry.transaction
            ))
        };
        let position = self
            .transactions
            .iter()
            .position(|t| t.id == entry.transaction);
        match entry.event {
            TaskEvent::Intent(intent) => {
                if position.is_some() || intent.id != entry.transaction {
                    return Err(bad("duplicate or mismatched intent"));
                }
                if self
                    .transactions
                    .iter()
                    .any(|t| t.status == TxStatus::Pending)
                {
                    return Err(bad("a new intent while another transaction is unfinished"));
                }
                self.transactions.push(ReplayedTransaction {
                    id: intent.id.clone(),
                    intent: *intent,
                    progress: Vec::new(),
                    status: TxStatus::Pending,
                });
            }
            event => {
                let tx =
                    &mut self.transactions[position.ok_or_else(|| bad("event without intent"))?];
                match (event, &tx.status) {
                    (TaskEvent::Progress(p), TxStatus::Pending | TxStatus::Committed(_)) => {
                        tx.progress.push(p)
                    }
                    (TaskEvent::Commit(record), TxStatus::Pending) => {
                        tx.status = TxStatus::Committed(record)
                    }
                    (TaskEvent::RolledBack { reason }, TxStatus::Pending) => {
                        tx.status = TxStatus::RolledBack(reason)
                    }
                    (TaskEvent::Conflict(report), TxStatus::Pending) => {
                        tx.status = TxStatus::Conflicted(report)
                    }
                    (TaskEvent::Resolved { note }, TxStatus::Conflicted(_)) => {
                        tx.status = TxStatus::Resolved(note)
                    }
                    _ => return Err(bad("event out of order")),
                }
            }
        }
        Ok(())
    }
}

pub struct TaskJournal {
    path: PathBuf,
    file: File,
    sequence: u64,
    failed: bool,
}

fn scan_task_journal(
    path: &std::path::Path,
    file: &File,
    repair: bool,
) -> Result<(TaskReplay, u64, u64), EngineError> {
    let mut reader = BufReader::new(file.try_clone()?);
    let mut replay = TaskReplay::default();
    let mut sequence = 0;
    let mut offset = 0;
    loop {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take(TASK_MAX_ENTRY_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            break;
        }
        if count > TASK_MAX_ENTRY_BYTES {
            return Err(EngineError::Diagnostic(format!(
                "{}: task journal entry exceeds 4 MiB at byte {offset}; preserve and repair history",
                path.display()
            )));
        }
        if line.last() != Some(&b'\n') {
            // An incomplete last append was never durable: its event did not happen.
            if repair {
                let quarantine = path.with_extension(format!("tail-{}", uuid::Uuid::new_v4()));
                studio_project::lifecycle::atomic_write(&quarantine, &line)?;
                truncate_and_sync(path, offset)?;
                replay.quarantined_tail = Some(quarantine);
            } else {
                replay.ignored_tail_bytes = line.len();
            }
            break;
        }
        let corrupt = |e: &dyn std::fmt::Display| {
            EngineError::Diagnostic(format!(
                "{}: malformed interior task journal at byte {offset}: {e}; preserve and repair the journal",
                path.display()
            ))
        };
        let header: EntryHeader = serde_json::from_slice(&line).map_err(|e| corrupt(&e))?;
        if header.format > TASK_FORMAT {
            return Err(EngineError::NewerFormat {
                what: format!("{} (task journal)", path.display()),
                found: header.format,
                supported: TASK_FORMAT,
            });
        }
        // Format 2 differs only in the `FileId` field names, which still decode.
        if header.format < OLDEST_TASK_FORMAT {
            return Err(corrupt(&format!("unsupported format {}", header.format)));
        }
        let entry: TaskEntry = serde_json::from_slice(&line).map_err(|e| corrupt(&e))?;
        if entry.sequence != sequence + 1 {
            return Err(corrupt(&"non-monotonic sequence"));
        }
        sequence = entry.sequence;
        offset += count as u64;
        replay.apply(entry)?;
    }
    Ok((replay, sequence, offset))
}

impl TaskJournal {
    pub fn open(path: PathBuf) -> Result<(Self, TaskReplay), EngineError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        let (replay, sequence, _) = scan_task_journal(&path, &file, true)?;
        sync_directory(path.parent().unwrap())?;
        Ok((
            Self {
                path,
                file,
                sequence,
                failed: false,
            },
            replay,
        ))
    }

    /// Reads a journal without creating, truncating or quarantining anything.
    pub fn inspect(path: &std::path::Path) -> Result<TaskReplay, EngineError> {
        match File::open(path) {
            Ok(file) => Ok(scan_task_journal(path, &file, false)?.0),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(TaskReplay::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// After a failed (or suspect) write the journal refuses everything until reopened,
    /// so a partial line can never become interior corruption.
    pub fn mark_failed(&mut self) {
        self.failed = true;
    }

    pub fn is_failed(&self) -> bool {
        self.failed
    }

    pub fn append(&mut self, transaction: &str, event: &TaskEvent) -> Result<(), EngineError> {
        if self.failed {
            return Err(EngineError::Diagnostic("Task journal write failed; repair disk/permissions, then close and reopen to recover before retrying".into()));
        }
        let next = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| EngineError::Diagnostic("Task journal sequence exhausted".into()))?;
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut bytes = serde_json::to_vec(&TaskEntry {
            format: TASK_FORMAT,
            sequence: next,
            transaction: transaction.into(),
            at,
            event: event.clone(),
        })?;
        bytes.push(b'\n');
        if bytes.len() > TASK_MAX_ENTRY_BYTES {
            self.failed = true;
            return Err(EngineError::Diagnostic(
                "Task journal entry exceeds 4 MiB; too many changed files for one revision".into(),
            ));
        }
        let mut write = || -> Result<(), EngineError> {
            self.file.write_all(&bytes)?;
            self.file.sync_all()?;
            sync_directory(self.path.parent().unwrap())?;
            Ok(())
        };
        if let Err(error) = write() {
            self.failed = true;
            return Err(error);
        }
        self.sequence = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn truncated_tail_is_quarantined_but_interior_corruption_stops_replay() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("journal");
        fs::write(&path, b"{truncated").unwrap();
        let (_, replay) = Journal::open(path.clone()).unwrap();
        assert!(replay.quarantined_tail.unwrap().is_file());
        assert_eq!(fs::read(&path).unwrap(), b"");
        fs::write(&path, b"{bad}\n{tail").unwrap();
        assert!(Journal::open(path.clone()).is_err());
        assert_eq!(fs::read(path).unwrap(), b"{bad}\n{tail");
    }

    #[test]
    fn oversized_entry_is_refused_without_truncating_history() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("journal");
        let bytes = vec![b'x'; MAX_ENTRY_BYTES + 1];
        fs::write(&path, &bytes).unwrap();
        assert!(
            Journal::open(path.clone())
                .err()
                .unwrap()
                .to_string()
                .contains("exceeds")
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    fn sample_intent(id: &str) -> TransactionIntent {
        use crate::edit_transaction::{TaskRevisionRecord, TransactionKind};
        let revision =
            |c: char| studio_project::SourceRevision::try_from(c.to_string().repeat(64)).unwrap();
        TransactionIntent {
            format: TASK_FORMAT,
            id: id.into(),
            kind: TransactionKind::Apply,
            base: revision('a'),
            expected: revision('b'),
            ops: vec![],
            dirs: vec![],
            remove_dirs: vec![],
            record: TaskRevisionRecord {
                id: id.into(),
                kind: TransactionKind::Apply,
                task: "task".into(),
                generation: 1,
                session: crate::OpenSession::new(),
                task_base: revision('a'),
                prior_history: None,
                prior_checkpoint: revision('a'),
                candidate: revision('b'),
                published: revision('b'),
                undoes: None,
                undoes_paths: None,
                prompt_summary: "edit".into(),
                changes: vec![],
                created_dirs: vec![],
                build: None,
                validation_report_sha256: "0".repeat(64),
                committed_unix: 0,
                preset: None,
                migration: None,
            },
        }
    }

    /// Rewrites every `{"volume": v, "file": f}` object into the format-2 `{"dev", "ino"}`.
    fn as_format_2(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                if map.len() == 2 && map.contains_key("volume") && map.contains_key("file") {
                    let volume = map.remove("volume").unwrap();
                    let file = map.remove("file").unwrap();
                    map.insert("dev".into(), volume);
                    map.insert("ino".into(), file);
                }
                map.values_mut().for_each(as_format_2);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(as_format_2),
            _ => (),
        }
    }

    #[test]
    fn a_format_2_journal_with_a_pending_transaction_replays_and_takes_current_format_appends() {
        use crate::edit_transaction::{DirIdentity, FileId, Step};
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tasks.jsonl");
        let mut intent = sample_intent("t1");
        intent.format = 2;
        intent.dirs = vec![DirIdentity {
            path: String::new(),
            id: FileId {
                volume: 2049,
                file: 4242,
            },
        }];
        let stage = Progress {
            op: Some(0),
            step: Step::StageCreated,
            detail: Some("2049:77".into()),
        };
        let mut lines = Vec::new();
        for (sequence, event) in [
            TaskEvent::Intent(Box::new(intent)),
            TaskEvent::Progress(stage),
        ]
        .into_iter()
        .enumerate()
        {
            let mut entry = serde_json::to_value(TaskEntry {
                format: 2,
                sequence: sequence as u64 + 1,
                transaction: "t1".into(),
                at: 1,
                event,
            })
            .unwrap();
            as_format_2(&mut entry);
            lines.extend(serde_json::to_vec(&entry).unwrap());
            lines.push(b'\n');
        }
        let written = String::from_utf8(lines.clone()).unwrap();
        assert!(written.contains(r#""dev":2049,"ino":4242"#), "{written}");
        assert!(!written.contains("volume"), "{written}");
        fs::write(&path, &lines).unwrap();
        let (mut journal, replay) = TaskJournal::open(path.clone()).unwrap();
        let pending = replay.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(
            pending[0].intent.dirs[0].id,
            FileId {
                volume: 2049,
                file: 4242
            }
        );
        assert_eq!(
            pending[0].progress[0]
                .detail
                .as_deref()
                .map(str::parse::<FileId>),
            Some(Ok(FileId {
                volume: 2049,
                file: 77
            }))
        );
        // Recovery appends in the current format after the format-2 lines.
        journal
            .append(
                "t1",
                &TaskEvent::RolledBack {
                    reason: "recovered".into(),
                },
            )
            .unwrap();
        drop(journal);
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.lines()
                .last()
                .unwrap()
                .starts_with(&format!("{{\"format\":{TASK_FORMAT},"))
        );
        let (_, replay) = TaskJournal::open(path).unwrap();
        assert!(replay.pending().is_empty());
    }

    /// Literal lines as builds of each format wrote them: format 2 (before the file
    /// identity rename), format 3 (before per-path Undo) and format 4.
    const MIXED_FORMAT_LINES: [&str; 6] = [
        r#"{"format":2,"sequence":1,"transaction":"tx2","at":1,"event":{"Intent":{"format":2,"id":"tx2","kind":"apply","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","expected":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","ops":[],"dirs":[],"remove_dirs":[],"record":{"id":"tx2","kind":"apply","task":"task","generation":1,"session":[216,98,108,159,0,28,70,147,181,187,208,45,221,178,232,140],"task_base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","prior_history":null,"prior_checkpoint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidate":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","published":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","undoes":null,"prompt_summary":"edit","changes":[],"created_dirs":[],"build":null,"validation_report_sha256":"0000000000000000000000000000000000000000000000000000000000000000","committed_unix":7}}}}"#,
        r#"{"format":2,"sequence":2,"transaction":"tx2","at":1,"event":{"Commit":{"id":"tx2","kind":"apply","task":"task","generation":1,"session":[216,98,108,159,0,28,70,147,181,187,208,45,221,178,232,140],"task_base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","prior_history":null,"prior_checkpoint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidate":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","published":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","undoes":null,"prompt_summary":"edit","changes":[],"created_dirs":[],"build":null,"validation_report_sha256":"0000000000000000000000000000000000000000000000000000000000000000","committed_unix":7}}}"#,
        r#"{"format":3,"sequence":3,"transaction":"tx3","at":1,"event":{"Intent":{"format":3,"id":"tx3","kind":"apply","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","expected":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","ops":[],"dirs":[],"remove_dirs":[],"record":{"id":"tx3","kind":"apply","task":"task","generation":1,"session":[63,221,233,111,229,7,72,106,152,148,137,111,52,105,131,251],"task_base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","prior_history":null,"prior_checkpoint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidate":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","published":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","undoes":null,"prompt_summary":"edit","changes":[],"created_dirs":[],"build":null,"validation_report_sha256":"0000000000000000000000000000000000000000000000000000000000000000","committed_unix":7}}}}"#,
        r#"{"format":3,"sequence":4,"transaction":"tx3","at":1,"event":{"Commit":{"id":"tx3","kind":"apply","task":"task","generation":1,"session":[63,221,233,111,229,7,72,106,152,148,137,111,52,105,131,251],"task_base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","prior_history":null,"prior_checkpoint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidate":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","published":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","undoes":null,"prompt_summary":"edit","changes":[],"created_dirs":[],"build":null,"validation_report_sha256":"0000000000000000000000000000000000000000000000000000000000000000","committed_unix":7}}}"#,
        r#"{"format":4,"sequence":5,"transaction":"tx4","at":1,"event":{"Intent":{"format":4,"id":"tx4","kind":"undo","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","expected":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","ops":[],"dirs":[],"remove_dirs":[],"record":{"id":"tx4","kind":"undo","task":"task","generation":1,"session":[250,29,45,151,46,16,74,198,139,127,131,229,59,183,210,61],"task_base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","prior_history":null,"prior_checkpoint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidate":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","published":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","undoes":"tx3","undoes_paths":["notes.txt"],"prompt_summary":"edit","changes":[],"created_dirs":[],"build":null,"validation_report_sha256":"0000000000000000000000000000000000000000000000000000000000000000","committed_unix":7}}}}"#,
        r#"{"format":4,"sequence":6,"transaction":"tx4","at":1,"event":{"Commit":{"id":"tx4","kind":"undo","task":"task","generation":1,"session":[250,29,45,151,46,16,74,198,139,127,131,229,59,183,210,61],"task_base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","prior_history":null,"prior_checkpoint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidate":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","published":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","undoes":"tx3","undoes_paths":["notes.txt"],"prompt_summary":"edit","changes":[],"created_dirs":[],"build":null,"validation_report_sha256":"0000000000000000000000000000000000000000000000000000000000000000","committed_unix":7}}}"#,
    ];

    #[test]
    fn a_journal_mixing_formats_2_3_and_4_decodes_every_record() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tasks.jsonl");
        let mut bytes = Vec::new();
        for line in MIXED_FORMAT_LINES {
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
        }
        fs::write(&path, &bytes).unwrap();
        let (mut journal, replay) = TaskJournal::open(path.clone()).unwrap();
        assert!(replay.pending().is_empty());
        let committed = replay.committed();
        let ids: Vec<_> = committed.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["tx2", "tx3", "tx4"]);
        // Older formats carry no per-path Undo; the format-4 record names its path.
        assert_eq!(committed[0].undoes_paths, None);
        assert_eq!(committed[1].undoes_paths, None);
        assert_eq!(committed[2].undoes.as_deref(), Some("tx3"));
        assert_eq!(
            committed[2].undoes_paths,
            Some(vec!["notes.txt".to_owned()])
        );
        // Reading never rewrites what is there.
        assert_eq!(fs::read(&path).unwrap(), bytes);
        // Later appends take the current format after the older lines.
        journal
            .append("tx5", &TaskEvent::Intent(Box::new(sample_intent("tx5"))))
            .unwrap();
        drop(journal);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(&MIXED_FORMAT_LINES[0].to_owned()));
        assert!(
            text.lines()
                .last()
                .unwrap()
                .starts_with(&format!("{{\"format\":{TASK_FORMAT},"))
        );
        assert!(TaskJournal::open(path).is_ok());
    }

    #[test]
    fn a_format_4_record_round_trips_its_reverted_paths() {
        let mut record = sample_intent("rt").record;
        record.undoes = Some("earlier".into());
        record.undoes_paths = Some(vec!["a.txt".into(), "dir/b.txt".into()]);
        let json = serde_json::to_string(&record).unwrap();
        assert!(
            json.contains(r#""undoes_paths":["a.txt","dir/b.txt"]"#),
            "{json}"
        );
        assert_eq!(
            serde_json::from_str::<crate::TaskRevisionRecord>(&json).unwrap(),
            record
        );
        // A whole-revision record writes no such field, so format-3 readers see the
        // same bytes they always did.
        record.undoes_paths = None;
        assert!(
            !serde_json::to_string(&record)
                .unwrap()
                .contains("undoes_paths")
        );
    }

    #[test]
    fn task_journal_replays_intent_progress_and_terminal_events() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tasks.jsonl");
        let (mut journal, replay) = TaskJournal::open(path.clone()).unwrap();
        assert!(replay.transactions.is_empty());
        journal
            .append("t1", &TaskEvent::Intent(Box::new(sample_intent("t1"))))
            .unwrap();
        journal
            .append(
                "t1",
                &TaskEvent::Progress(Progress {
                    op: Some(0),
                    step: crate::edit_transaction::Step::Staged,
                    detail: None,
                }),
            )
            .unwrap();
        drop(journal);
        let (mut journal, replay) = TaskJournal::open(path.clone()).unwrap();
        assert_eq!(replay.pending().len(), 1);
        assert_eq!(replay.transactions[0].progress.len(), 1);
        // A second intent while one is unfinished is corruption, not a queue.
        journal
            .append("t2", &TaskEvent::Intent(Box::new(sample_intent("t2"))))
            .unwrap();
        drop(journal);
        assert!(TaskJournal::open(path).is_err());
    }

    #[test]
    fn task_journal_truncated_tail_is_quarantined_and_interior_damage_stops_replay() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tasks.jsonl");
        let (mut journal, _) = TaskJournal::open(path.clone()).unwrap();
        journal
            .append("t1", &TaskEvent::Intent(Box::new(sample_intent("t1"))))
            .unwrap();
        drop(journal);
        let intact = fs::read(&path).unwrap();
        let mut torn = intact.clone();
        torn.extend_from_slice(b"{\"format\":2,\"sequ");
        fs::write(&path, &torn).unwrap();
        // Read-only inspection ignores the tail and touches nothing.
        let replay = TaskJournal::inspect(&path).unwrap();
        assert_eq!(replay.ignored_tail_bytes, 17);
        assert_eq!(fs::read(&path).unwrap(), torn);
        let (_, replay) = TaskJournal::open(path.clone()).unwrap();
        assert!(replay.quarantined_tail.unwrap().is_file());
        assert_eq!(fs::read(&path).unwrap(), intact);
        let mut damaged = b"{\"format\":2,garbage}\n".to_vec();
        damaged.extend_from_slice(&intact);
        fs::write(&path, &damaged).unwrap();
        assert!(TaskJournal::open(path.clone()).is_err());
        assert_eq!(fs::read(&path).unwrap(), damaged);
    }

    #[test]
    fn a_newer_task_journal_is_refused_without_being_changed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tasks.jsonl");
        let newer = format!(
            "{{\"format\":{},\"sequence\":1,\"future\":true}}\n",
            TASK_FORMAT + 1
        );
        let newer = newer.as_bytes();
        fs::write(&path, newer).unwrap();
        let error = TaskJournal::open(path.clone()).err().unwrap();
        assert!(
            matches!(
                error,
                EngineError::NewerFormat { found, supported, .. }
                    if found == TASK_FORMAT + 1 && supported == TASK_FORMAT
            ),
            "{error}"
        );
        assert!(TaskJournal::inspect(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), newer);
    }

    #[test]
    fn inspecting_a_lifecycle_journal_never_creates_quarantines_or_truncates() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("journal");
        // Missing: an empty journal, and nothing is created.
        assert!(Journal::inspect(&path).unwrap().committed.is_none());
        assert!(!path.exists());
        // A torn tail is ignored by the read and left exactly as it is.
        fs::write(&path, b"{truncated").unwrap();
        let replay = Journal::inspect(&path).unwrap();
        assert!(replay.quarantined_tail.is_none());
        assert_eq!(fs::read(&path).unwrap(), b"{truncated");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
        // Interior damage still refuses, again without touching a byte.
        fs::write(&path, b"{bad}\n{tail").unwrap();
        assert!(Journal::inspect(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{bad}\n{tail");
    }

    #[test]
    fn a_newer_lifecycle_journal_is_refused_as_a_newer_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("journal");
        let newer = b"{\"version\":2,\"sequence\":1,\"future\":true}\n";
        fs::write(&path, newer).unwrap();
        for error in [
            Journal::inspect(&path).err().unwrap(),
            Journal::open(path.clone()).err().unwrap(),
        ] {
            assert!(
                matches!(
                    error,
                    EngineError::NewerFormat {
                        found: 2,
                        supported: 1,
                        ..
                    }
                ),
                "{error}"
            );
        }
        assert_eq!(fs::read(&path).unwrap(), newer);
    }

    #[test]
    fn disk_full_writer_refuses_further_appends_until_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut journal, _) = Journal::open(tmp.path().join("journal")).unwrap();
        journal.simulate_full_disk().unwrap();
        assert!(journal.commit("test").is_err());
        assert!(
            journal
                .commit("retry")
                .unwrap_err()
                .to_string()
                .contains("close and reopen")
        );
        assert_eq!(journal.sequence, 0);
        assert_eq!(fs::read(journal.path()).unwrap(), b"");
    }
}
