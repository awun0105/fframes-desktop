//! The diff model: what the viewer takes in, the line-level diff built from it, and the flat row
//! list the viewer paints.
//!
//! A [`DiffFile`] carries the full text of a file before and after a change. [`FileDiff::compute`]
//! diffs it by lines with `similar` and keeps three lines of context around each change, the
//! shape a patch has. Nothing here touches gpui.

use std::time::Duration;

use similar::{ChangeTag, TextDiff};
use unicode_width::UnicodeWidthChar as _;

/// Context lines kept around each change.
pub const CONTEXT_LINES: usize = 3;

/// Spaces between tab stops when a line is drawn.
pub const TAB_SIZE: usize = 4;

/// Files whose text is larger than this on either side are listed but not diffed.
pub const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;

/// The text `\ No newline at end of file` row that follows a last line without a terminator.
pub const NO_NEWLINE_MARKER: &str = "\\ No newline at end of file";

/// Time one file's diff may take before similar returns its best answer so far.
const DIFF_DEADLINE: Duration = Duration::from_secs(1);

/// Why a file's contents are not shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotShown {
    Binary,
    TooLarge,
}

/// One changed file as the viewer takes it. `None` means the file is absent on that side: an
/// added file has no `before`, a deleted one no `after`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFile {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    /// Set for a file the caller could not or would not read as text.
    pub not_shown: Option<NotShown>,
}

impl DiffFile {
    pub fn new(path: impl Into<String>, before: Option<String>, after: Option<String>) -> Self {
        Self {
            path: path.into(),
            before,
            after,
            not_shown: None,
        }
    }

    /// A file with binary content: listed, never diffed.
    pub fn binary(path: impl Into<String>) -> Self {
        Self {
            not_shown: Some(NotShown::Binary),
            ..Self::new(path, None, None)
        }
    }

    /// A file too large to read: listed, never diffed.
    pub fn too_large(path: impl Into<String>) -> Self {
        Self {
            not_shown: Some(NotShown::TooLarge),
            ..Self::new(path, None, None)
        }
    }
}

/// How the diff is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// One column: deletions above additions.
    #[default]
    Unified,
    /// Two columns: old on the left, new on the right.
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Add,
    Del,
    /// The no-newline marker, a note about the line above it.
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    /// The line without its terminator.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub not_shown: Option<NotShown>,
    /// One-line facts shown above the hunks ("New file", "No changes", ...).
    pub notices: Vec<String>,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
    /// The largest line number on either side, which sizes the gutters.
    pub max_line: u32,
}

impl FileDiff {
    /// Diffs `file` by lines.
    pub fn compute(file: &DiffFile) -> Self {
        let status = match (&file.before, &file.after) {
            (None, Some(_)) => FileStatus::Added,
            (Some(_), None) => FileStatus::Deleted,
            _ => FileStatus::Modified,
        };
        let mut diff = Self {
            path: file.path.clone(),
            status,
            not_shown: file.not_shown,
            notices: Vec::new(),
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            max_line: 0,
        };
        match status {
            FileStatus::Added => diff.notices.push("New file".to_owned()),
            FileStatus::Deleted => diff.notices.push("Deleted file".to_owned()),
            FileStatus::Modified => {}
        }
        let (before, after) = (
            file.before.as_deref().unwrap_or(""),
            file.after.as_deref().unwrap_or(""),
        );
        if diff.not_shown.is_none() && before.len().max(after.len()) > MAX_DIFF_BYTES {
            diff.not_shown = Some(NotShown::TooLarge);
        }
        match diff.not_shown {
            Some(NotShown::Binary) => {
                diff.notices
                    .push("Binary file, contents not shown".to_owned());
                return diff;
            }
            Some(NotShown::TooLarge) => {
                diff.notices
                    .push("File too large, contents not shown".to_owned());
                return diff;
            }
            None => {}
        }
        diff.fill_hunks(before, after);
        if diff.hunks.is_empty() && status == FileStatus::Modified {
            diff.notices.push("No changes".to_owned());
        } else if status == FileStatus::Modified && only_line_endings_differ(before, after) {
            diff.notices.push("Only line endings changed".to_owned());
        }
        diff
    }

    fn fill_hunks(&mut self, before: &str, after: &str) {
        let text_diff = TextDiff::configure()
            .timeout(DIFF_DEADLINE)
            .diff_lines(before, after);
        for group in text_diff.grouped_ops(CONTEXT_LINES) {
            let (Some(first), Some(last)) = (group.first(), group.last()) else {
                continue;
            };
            let old = first.old_range().start..last.old_range().end;
            let new = first.new_range().start..last.new_range().end;
            let mut lines = Vec::new();
            for op in &group {
                for change in text_diff.iter_changes(op) {
                    let raw = change.value();
                    let terminated = raw.ends_with('\n');
                    let text = raw
                        .strip_suffix('\n')
                        .map(|rest| rest.strip_suffix('\r').unwrap_or(rest))
                        .unwrap_or(raw)
                        .to_owned();
                    let number = |ix: Option<usize>| ix.map(|ix| ix as u32 + 1);
                    let (kind, old_no, new_no) = match change.tag() {
                        ChangeTag::Equal => (
                            LineKind::Context,
                            number(change.old_index()),
                            number(change.new_index()),
                        ),
                        ChangeTag::Delete => {
                            self.deletions += 1;
                            (LineKind::Del, number(change.old_index()), None)
                        }
                        ChangeTag::Insert => {
                            self.additions += 1;
                            (LineKind::Add, None, number(change.new_index()))
                        }
                    };
                    self.max_line = self
                        .max_line
                        .max(old_no.unwrap_or(0).max(new_no.unwrap_or(0)));
                    lines.push(DiffLine {
                        kind,
                        old_no,
                        new_no,
                        text,
                    });
                    if !terminated {
                        lines.push(DiffLine {
                            kind: LineKind::Meta,
                            old_no: None,
                            new_no: None,
                            text: NO_NEWLINE_MARKER.to_owned(),
                        });
                    }
                }
            }
            self.hunks.push(Hunk {
                header: hunk_header(&old, &new),
                lines,
            });
        }
    }
}

fn hunk_header(old: &std::ops::Range<usize>, new: &std::ops::Range<usize>) -> String {
    let side = |range: &std::ops::Range<usize>| {
        let start = if range.is_empty() {
            range.start
        } else {
            range.start + 1
        };
        format!("{start},{}", range.len())
    };
    format!("@@ -{} +{} @@", side(old), side(new))
}

fn only_line_endings_differ(before: &str, after: &str) -> bool {
    before != after && before.replace("\r\n", "\n") == after.replace("\r\n", "\n")
}

/// Width of one line-number column for a file whose largest line number is `max_line`.
pub fn gutter_width(max_line: u32) -> f32 {
    let digits = max_line.max(1).ilog10() + 1;
    (digits as f32 * 6.6 + 14.0).max(36.0)
}

/// Appends `text` to `out` with tabs expanded, starting at `column` and leaving it at the end.
pub fn push_expanded(text: &str, column: &mut usize, out: &mut String) {
    for c in text.chars() {
        let step = advance(c, *column);
        if c == '\t' {
            out.extend(std::iter::repeat_n(' ', step));
        } else {
            out.push(c);
        }
        *column += step;
    }
}

fn advance(c: char, column: usize) -> usize {
    if c == '\t' {
        TAB_SIZE - column % TAB_SIZE
    } else {
        c.width().unwrap_or(0)
    }
}

/// One split row: indices into the hunk's lines for the old (left) and new (right) column.
pub type LinePair = (Option<u32>, Option<u32>);

/// Pairs a hunk's lines into split rows.
///
/// Context sits on both sides. A run of deletions followed by additions is a change block whose
/// two sides line up index for index; the longer side's leftovers get one-sided rows. A deletion
/// after additions opens a new block. A no-newline marker trails the line it annotates and pairs
/// with the other side's marker, so a file that lost its final newline on both sides is one
/// aligned row plus one marker row.
pub fn split_pairs(lines: &[DiffLine]) -> Vec<LinePair> {
    #[derive(Default)]
    struct Block {
        dels: Vec<u32>,
        adds: Vec<u32>,
        del_meta: Vec<u32>,
        add_meta: Vec<u32>,
    }

    fn flush(pairs: &mut Vec<LinePair>, block: &mut Block) {
        for (left, right) in [
            (&mut block.dels, &mut block.adds),
            (&mut block.del_meta, &mut block.add_meta),
        ] {
            for ix in 0..left.len().max(right.len()) {
                pairs.push((left.get(ix).copied(), right.get(ix).copied()));
            }
            left.clear();
            right.clear();
        }
    }

    let mut pairs = Vec::with_capacity(lines.len());
    let mut block = Block::default();
    let mut pending: Option<LineKind> = None;
    for (ix, line) in lines.iter().enumerate() {
        let ix = ix as u32;
        match line.kind {
            LineKind::Del => {
                if !block.adds.is_empty()
                    || !block.del_meta.is_empty()
                    || !block.add_meta.is_empty()
                {
                    flush(&mut pairs, &mut block);
                }
                block.dels.push(ix);
                pending = Some(LineKind::Del);
            }
            LineKind::Add => {
                if !block.add_meta.is_empty() {
                    flush(&mut pairs, &mut block);
                }
                block.adds.push(ix);
                pending = Some(LineKind::Add);
            }
            LineKind::Meta => match pending {
                Some(LineKind::Del) => block.del_meta.push(ix),
                Some(LineKind::Add) => block.add_meta.push(ix),
                _ => {
                    block.del_meta.push(ix);
                    block.add_meta.push(ix);
                }
            },
            LineKind::Context => {
                flush(&mut pairs, &mut block);
                pairs.push((Some(ix), Some(ix)));
                pending = Some(LineKind::Context);
            }
        }
    }
    flush(&mut pairs, &mut block);
    pairs
}

/// One painted row of the flattened diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRow {
    FileHeader {
        file: u32,
    },
    Notice {
        file: u32,
        notice: u32,
    },
    HunkHeader {
        file: u32,
        hunk: u32,
    },
    Line {
        file: u32,
        hunk: u32,
        line: u32,
    },
    SplitLine {
        file: u32,
        hunk: u32,
        left: Option<u32>,
        right: Option<u32>,
    },
}

/// Every file's rows in order, plus the row index of each file's header. A collapsed file keeps
/// only its header.
pub fn flatten_rows(
    files: &[FileDiff],
    mode: DiffMode,
    mut collapsed: impl FnMut(usize) -> bool,
) -> (Vec<DiffRow>, Vec<usize>) {
    let mut rows = Vec::new();
    let mut headers = Vec::with_capacity(files.len());
    for (ix, file) in files.iter().enumerate() {
        let f = ix as u32;
        headers.push(rows.len());
        rows.push(DiffRow::FileHeader { file: f });
        if collapsed(ix) {
            continue;
        }
        for notice in 0..file.notices.len() {
            rows.push(DiffRow::Notice {
                file: f,
                notice: notice as u32,
            });
        }
        for (h, hunk) in file.hunks.iter().enumerate() {
            let h = h as u32;
            rows.push(DiffRow::HunkHeader { file: f, hunk: h });
            match mode {
                DiffMode::Unified => {
                    rows.extend((0..hunk.lines.len() as u32).map(|line| DiffRow::Line {
                        file: f,
                        hunk: h,
                        line,
                    }))
                }
                DiffMode::Split => {
                    rows.extend(split_pairs(&hunk.lines).into_iter().map(|(left, right)| {
                        DiffRow::SplitLine {
                            file: f,
                            hunk: h,
                            left,
                            right,
                        }
                    }))
                }
            }
        }
    }
    (rows, headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(before: Option<&str>, after: Option<&str>) -> FileDiff {
        FileDiff::compute(&DiffFile::new(
            "src/lib.rs",
            before.map(str::to_owned),
            after.map(str::to_owned),
        ))
    }

    fn kinds(file: &FileDiff) -> Vec<(LineKind, &str)> {
        file.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .map(|l| (l.kind, l.text.as_str()))
            .collect()
    }

    #[test]
    fn additions_are_numbered_on_the_new_side() {
        let file = diff(Some("a\nb\n"), Some("a\nx\nb\n"));
        assert_eq!((file.additions, file.deletions), (1, 0));
        let added = &file.hunks[0].lines[1];
        assert_eq!(
            (added.kind, added.old_no, added.new_no, added.text.as_str()),
            (LineKind::Add, None, Some(2), "x")
        );
        assert_eq!(file.hunks[0].header, "@@ -1,2 +1,3 @@");
    }

    #[test]
    fn deletions_are_numbered_on_the_old_side() {
        let file = diff(Some("a\nb\nc\n"), Some("a\nc\n"));
        assert_eq!((file.additions, file.deletions), (0, 1));
        let gone = &file.hunks[0].lines[1];
        assert_eq!(
            (gone.kind, gone.old_no, gone.new_no),
            (LineKind::Del, Some(2), None)
        );
    }

    #[test]
    fn a_modification_is_a_delete_then_an_add() {
        let file = diff(Some("one\ntwo\nthree\n"), Some("one\n2\nthree\n"));
        assert_eq!(
            kinds(&file),
            [
                (LineKind::Context, "one"),
                (LineKind::Del, "two"),
                (LineKind::Add, "2"),
                (LineKind::Context, "three"),
            ]
        );
    }

    #[test]
    fn an_added_file_is_all_additions() {
        let file = diff(None, Some("a\nb\n"));
        assert_eq!(file.status, FileStatus::Added);
        assert_eq!((file.additions, file.deletions), (2, 0));
        assert_eq!(file.notices, ["New file"]);
        assert_eq!(file.hunks[0].header, "@@ -0,0 +1,2 @@");
    }

    #[test]
    fn a_deleted_file_is_all_deletions() {
        let file = diff(Some("a\nb\n"), None);
        assert_eq!(file.status, FileStatus::Deleted);
        assert_eq!((file.additions, file.deletions), (0, 2));
        assert_eq!(file.notices, ["Deleted file"]);
    }

    #[test]
    fn identical_text_has_no_hunks_and_says_so() {
        let file = diff(Some("same\n"), Some("same\n"));
        assert!(file.hunks.is_empty() && file.additions + file.deletions == 0);
        assert_eq!(file.notices, ["No changes"]);
    }

    #[test]
    fn distant_changes_make_separate_hunks_with_three_lines_of_context() {
        let before: String = (1..=30).map(|n| format!("l{n}\n")).collect();
        let after = before.replace("l2\n", "L2\n").replace("l29\n", "L29\n");
        let file = diff(Some(&before), Some(&after));
        assert_eq!(file.hunks.len(), 2);
        assert_eq!(file.hunks[0].lines.len(), 2 + 3 + 1); // l1, -l2, +L2, l3..l5
        assert_eq!(file.max_line, 30);
    }

    #[test]
    fn crlf_lines_lose_their_terminator_and_a_crlf_to_lf_change_is_reported() {
        let same = diff(Some("a\r\nb\r\n"), Some("a\r\nc\r\n"));
        assert_eq!(
            kinds(&same),
            [
                (LineKind::Context, "a"),
                (LineKind::Del, "b"),
                (LineKind::Add, "c")
            ]
        );
        let ends = diff(Some("a\r\nb\r\n"), Some("a\nb\n"));
        assert_eq!((ends.additions, ends.deletions), (2, 2));
        assert!(
            ends.notices
                .contains(&"Only line endings changed".to_owned())
        );
        assert!(ends.hunks[0].lines.iter().all(|l| !l.text.contains('\r')));
    }

    #[test]
    fn a_missing_final_newline_adds_a_marker_row() {
        let file = diff(Some("a\nb"), Some("a\nb\n"));
        assert_eq!(
            kinds(&file),
            [
                (LineKind::Context, "a"),
                (LineKind::Del, "b"),
                (LineKind::Meta, NO_NEWLINE_MARKER),
                (LineKind::Add, "b"),
            ]
        );
        let both = diff(Some("a\nb"), Some("a\nc"));
        let markers = kinds(&both)
            .iter()
            .filter(|(kind, _)| *kind == LineKind::Meta)
            .count();
        assert_eq!(markers, 2);
        assert_eq!(
            both.additions + both.deletions,
            2,
            "markers are not changes"
        );
    }

    fn expanded(text: &str) -> (String, usize) {
        let (mut column, mut out) = (0, String::new());
        push_expanded(text, &mut column, &mut out);
        (out, column)
    }

    #[test]
    fn display_columns_count_wide_characters_and_tab_stops() {
        assert_eq!(expanded("abc"), ("abc".to_owned(), 3));
        assert_eq!(expanded("日本").1, 4);
        assert_eq!(expanded("a\tb"), ("a   b".to_owned(), 5));
        assert_eq!(expanded("日\tx"), ("日  x".to_owned(), 5));
        assert_eq!(expanded("\t\ty").0, "        y");
        assert_eq!(expanded("e\u{301}").1, 1, "combining marks add no column");
    }

    #[test]
    fn binary_and_oversized_files_are_listed_but_not_diffed() {
        let binary = FileDiff::compute(&DiffFile::binary("a.png"));
        assert!(binary.hunks.is_empty());
        assert_eq!(binary.notices, ["Binary file, contents not shown"]);
        let huge = "x\n".repeat(MAX_DIFF_BYTES);
        let big = FileDiff::compute(&DiffFile::new("big.txt", Some(huge), Some(String::new())));
        assert_eq!(big.not_shown, Some(NotShown::TooLarge));
        assert!(big.hunks.is_empty());
    }

    fn line(kind: LineKind) -> DiffLine {
        DiffLine {
            kind,
            old_no: None,
            new_no: None,
            text: String::new(),
        }
    }

    #[test]
    fn split_pairs_line_up_a_change_block_and_leave_leftovers_one_sided() {
        use LineKind::*;
        let lines: Vec<_> = [Context, Del, Add, Add, Add, Context]
            .into_iter()
            .map(line)
            .collect();
        assert_eq!(
            split_pairs(&lines),
            [
                (Some(0), Some(0)),
                (Some(1), Some(2)),
                (None, Some(3)),
                (None, Some(4)),
                (Some(5), Some(5)),
            ]
        );
        let two_edits: Vec<_> = [Del, Add, Del, Add].into_iter().map(line).collect();
        assert_eq!(
            split_pairs(&two_edits),
            [(Some(0), Some(1)), (Some(2), Some(3))]
        );
    }

    #[test]
    fn flatten_keeps_only_the_header_of_a_collapsed_file() {
        let files = [
            diff(Some("a\n"), Some("b\n")),
            diff(Some("c\n"), Some("d\n")),
        ];
        let (rows, headers) = flatten_rows(&files, DiffMode::Unified, |ix| ix == 0);
        assert_eq!(headers, [0, 1]);
        assert_eq!(rows[0], DiffRow::FileHeader { file: 0 });
        assert_eq!(rows[1], DiffRow::FileHeader { file: 1 });
        let (split, _) = flatten_rows(&files, DiffMode::Split, |_| false);
        assert!(split.iter().any(|row| matches!(
            row,
            DiffRow::SplitLine {
                left: Some(_),
                right: Some(_),
                ..
            }
        )));
    }
}
