//! Before and after text of every file a candidate (or an applied revision) changed,
//! read from the immutable checkpoint objects, never from the project folder.
//!
//! Files that are not text, or that are too large to show, carry a reason instead of
//! content, and the bytes shown in one result are bounded.

use super::*;
use crate::edit_transaction::{FileDelta, FileState};

/// Largest file shown as text.
pub const MAX_TEXT_BYTES: u64 = 512 * 1024;
/// Most bytes of text one result carries; files beyond it are not shown.
pub const MAX_TOTAL_TEXT_BYTES: u64 = 8 * 1024 * 1024;
/// Most files one result lists; the rest are summed up in one trailing entry with an
/// empty `path` whose sides are [`NotShown::More`].
pub const MAX_DIFF_FILES: usize = 2_000;

/// Why a side of a file carries no text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotShown {
    /// Not valid UTF-8 text.
    Binary,
    TooLarge {
        size: u64,
    },
    /// The result already holds [`MAX_TOTAL_TEXT_BYTES`] of text.
    Budget,
    Unreadable(String),
    /// Marks the trailing entry that stands for `files` more changed files than
    /// [`MAX_DIFF_FILES`].
    More {
        files: usize,
    },
}

impl NotShown {
    /// The reason, as a sentence fragment for the viewer.
    pub fn reason(&self) -> String {
        match self {
            Self::Binary => "binary file".into(),
            Self::TooLarge { size } => format!(
                "file too large to show ({} KiB; the limit is {} KiB)",
                size.div_ceil(1024),
                MAX_TEXT_BYTES / 1024
            ),
            Self::Budget => "too much changed text to show everything".into(),
            Self::Unreadable(why) => format!("could not be read: {why}"),
            Self::More { files } => format!("{files} more changed files are not listed"),
        }
    }
}

/// The trailing entry for the files past [`MAX_DIFF_FILES`].
fn more_files(files: usize) -> FileDiff {
    FileDiff {
        path: String::new(),
        change: ChangeKind::Modified,
        before: FileText::NotShown(NotShown::More { files }),
        after: FileText::NotShown(NotShown::More { files }),
    }
}

/// Collects `entries` into at most [`MAX_DIFF_FILES`] diffs plus the trailing marker.
fn bounded<T>(entries: Vec<T>, mut diff: impl FnMut(T) -> FileDiff) -> Vec<FileDiff> {
    let omitted = entries.len().saturating_sub(MAX_DIFF_FILES);
    let mut diffs: Vec<FileDiff> = entries
        .into_iter()
        .take(MAX_DIFF_FILES)
        .map(&mut diff)
        .collect();
    if omitted > 0 {
        diffs.push(more_files(omitted));
    }
    diffs
}

/// Accepts at most `limit` bytes: a damaged object cannot grow the buffer past what its
/// recorded size allows.
struct Capped<'a> {
    bytes: &'a mut Vec<u8>,
    limit: usize,
}

impl std::io::Write for Capped<'_> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.bytes.len() + data.len() > self.limit {
            return Err(std::io::Error::other(
                "object is larger than its recorded size",
            ));
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// One side of a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileText {
    /// The file does not exist on this side (before an addition, after a deletion).
    Absent,
    Text(String),
    NotShown(NotShown),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub change: ChangeKind,
    pub before: FileText,
    pub after: FileText,
}

struct Reader<'a> {
    checkpoints: &'a Checkpoints,
    remaining: u64,
}

impl Reader<'_> {
    fn read(&mut self, file: Option<(&str, u64)>) -> FileText {
        let Some((sha256, size)) = file else {
            return FileText::Absent;
        };
        if size > MAX_TEXT_BYTES {
            return FileText::NotShown(NotShown::TooLarge { size });
        }
        if size > self.remaining {
            return FileText::NotShown(NotShown::Budget);
        }
        let mut bytes = Vec::with_capacity(size as usize);
        let mut out = Capped {
            bytes: &mut bytes,
            limit: size as usize,
        };
        if let Err(error) = self.checkpoints.copy_object(sha256, size, &mut out) {
            return FileText::NotShown(NotShown::Unreadable(error.to_string()));
        }
        self.remaining -= size;
        match String::from_utf8(bytes) {
            Ok(text) if !text.contains('\0') => FileText::Text(text),
            _ => FileText::NotShown(NotShown::Binary),
        }
    }
}

/// Per-file text of everything `candidate` changes against `base`, in path order.
pub fn candidate_file_diffs(
    checkpoints: &Checkpoints,
    base: &SourceRevision,
    candidate: &SourceRevision,
) -> Result<Vec<FileDiff>, CandidateError> {
    let base = checkpoints
        .load(base)
        .map_err(|e| CandidateError::BaseMissing(e.to_string()))?;
    let candidate = checkpoints
        .load(candidate)
        .map_err(|e| CandidateError::Storage(e.to_string()))?;
    let mut reader = Reader {
        checkpoints,
        remaining: MAX_TOTAL_TEXT_BYTES,
    };
    Ok(bounded(file_changes(&base, &candidate), |change| {
        FileDiff {
            path: change.path.to_owned(),
            change: change.change,
            before: reader.read(change.before.map(|f| (f.sha256.as_str(), f.size))),
            after: reader.read(change.after.map(|f| (f.sha256.as_str(), f.size))),
        }
    }))
}

/// Per-file text of the changes an applied revision recorded.
pub fn delta_file_diffs(checkpoints: &Checkpoints, deltas: &[FileDelta]) -> Vec<FileDiff> {
    let mut reader = Reader {
        checkpoints,
        remaining: MAX_TOTAL_TEXT_BYTES,
    };
    let state = |state: &Option<FileState>| state.as_ref().map(|s| (s.sha256.clone(), s.size));
    bounded(deltas.iter().collect(), |delta| {
        let (before, after) = (state(&delta.before), state(&delta.after));
        let change = match (&before, &after) {
            (None, _) => ChangeKind::Added,
            (_, None) => ChangeKind::Deleted,
            (Some(b), Some(a)) if b == a => ChangeKind::Mode,
            _ => ChangeKind::Modified,
        };
        FileDiff {
            path: delta.path.clone(),
            change,
            before: reader.read(before.as_ref().map(|(h, s)| (h.as_str(), *s))),
            after: reader.read(after.as_ref().map(|(h, s)| (h.as_str(), *s))),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_long_list_is_cut_with_a_trailing_count() {
        let entries: Vec<usize> = (0..MAX_DIFF_FILES + 3).collect();
        let diffs = bounded(entries, |n| FileDiff {
            path: n.to_string(),
            change: ChangeKind::Added,
            before: FileText::Absent,
            after: FileText::Absent,
        });
        assert_eq!(diffs.len(), MAX_DIFF_FILES + 1);
        let last = diffs.last().unwrap();
        assert!(last.path.is_empty());
        assert_eq!(last.after, FileText::NotShown(NotShown::More { files: 3 }));
    }

    #[test]
    fn an_object_cannot_grow_past_its_recorded_size() {
        let mut bytes = Vec::new();
        let mut out = Capped {
            bytes: &mut bytes,
            limit: 4,
        };
        out.write_all(b"abcd").unwrap();
        assert!(out.write_all(b"e").is_err());
        assert_eq!(bytes, b"abcd");
    }
}
