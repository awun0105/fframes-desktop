//! File links written as inline code.
//!
//! Agents name files in code spans (`SOURCES.md`, `src/main.rs:12`) that the block parser has no
//! link for. A span whose text names an existing project file is rewritten into the link it
//! stands for, shown under its file name; spans with nothing behind them keep the inline-code
//! look. Where two different paths in one block end in the same name, each shows as many
//! trailing components as it takes to tell them apart. A directory span is not a link, but it is
//! the context a later bare name in the same block resolves against.
//!
//! Nothing here touches the filesystem while a row renders. [`link_block`] only asks a lookup
//! whether a project path is known to be a file, a directory or missing, and reports which
//! paths it asked about; [`Probes`] holds the answers, and [`kind_on_disk`] finds them on a
//! background task. Until an answer arrives the span stays plain code. Every candidate goes
//! through the link policy in [`super::links`] (`classify`, then `action_for`), so a span can
//! only link to what a markdown link to the same path could, and a click takes the same route.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use studio_markdown::{Block, InlineRun};

use super::links::{LinkAction, LinkTarget, action_for, classify, without_location};

/// What a project path is, as far as the disk showed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
    Missing,
}

/// Code spans resolved per block; the rest stay plain.
const MAX_SPANS: usize = 64;
/// Context directories kept per block.
const MAX_DIRS: usize = 8;
/// Longest span considered a path.
const MAX_SPAN_LEN: usize = 512;
/// Answers kept per row; later ones are dropped and their spans stay plain code.
const MAX_KNOWN: usize = 512;

/// What a lookup answered for a project path: `None` while no probe has finished.
pub type Seen = Vec<(String, Option<Kind>)>;

/// A block with its resolved spans rewritten, and every path that was asked about.
pub struct Linked {
    /// `None` when no span resolved (the caller keeps the original block).
    pub block: Option<Block>,
    pub seen: Seen,
}

impl Linked {
    /// The paths no probe has answered yet.
    pub fn unknown(&self) -> impl Iterator<Item = &str> {
        self.seen
            .iter()
            .filter(|(_, kind)| kind.is_none())
            .map(|(key, _)| key.as_str())
    }
}

/// The probe answers of one row for one project root.
#[derive(Default)]
pub struct Probes {
    known: HashMap<String, Kind>,
    in_flight: HashSet<String>,
    generation: u64,
}

impl Probes {
    pub fn get(&self, key: &str) -> Option<Kind> {
        self.known.get(key).copied()
    }

    /// Forgets every answer (the project changed); results of earlier probes are ignored.
    pub fn clear(&mut self) {
        self.known.clear();
        self.in_flight.clear();
        self.generation += 1;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The keys among `wanted` that nobody is probing yet; they count as in flight from now on.
    /// Nothing is handed out once answers and probes in flight fill the cap, so the disk work of
    /// a row is bounded too.
    pub fn claim<'a>(&mut self, wanted: impl Iterator<Item = &'a str>) -> Vec<String> {
        let mut fresh = Vec::new();
        for key in wanted {
            if self.known.len() + self.in_flight.len() >= MAX_KNOWN {
                break;
            }
            if !self.known.contains_key(key) && self.in_flight.insert(key.to_owned()) {
                fresh.push(key.to_owned());
            }
        }
        fresh
    }

    /// Stores finished probes. Returns false when they belong to an earlier project.
    pub fn accept(&mut self, generation: u64, results: Vec<(String, Kind)>) -> bool {
        if generation != self.generation {
            return false;
        }
        for (key, kind) in results {
            self.in_flight.remove(&key);
            if self.known.len() < MAX_KNOWN {
                self.known.insert(key, kind);
            }
        }
        true
    }
}

/// The destination of a markdown link to `path` (a decoded project path) at `line`.
/// `path` as the text of a link destination: the characters the link policy decodes or splits
/// on are escaped.
fn encode(path: &str) -> String {
    path.replace('%', "%25").replace('#', "%23")
}

/// The destination of a markdown link to `path` (a decoded project path) at `line`.
fn destination(path: &str, line: Option<u32>) -> String {
    let mut dest = encode(path);
    if let Some(line) = line {
        dest.push_str(&format!(":{line}"));
    }
    dest
}

/// The project path (and line) a link to `dest` names under the link policy.
fn project_path(dest: &str) -> Option<(String, Option<u32>)> {
    match classify(dest) {
        LinkTarget::ProjectFile { path, line } => Some((path.as_str().to_owned(), line)),
        _ => None,
    }
}

/// What `key` is on disk under `root`, by the link policy's own on-disk check. Blocking: run it
/// off the render path.
pub fn kind_on_disk(key: &str, root: &Path) -> Kind {
    let target = classify(&destination(key, None));
    match action_for(&target, root) {
        LinkAction::Reveal(abs) => match std::fs::metadata(abs) {
            Ok(meta) if meta.is_file() => Kind::File,
            Ok(meta) if meta.is_dir() => Kind::Dir,
            _ => Kind::Missing,
        },
        _ => Kind::Missing,
    }
}

/// What one code span stands for once resolved.
enum Resolved {
    File { dest: String },
    Dir(String),
}

struct Resolver<'a> {
    lookup: &'a dyn Fn(&str) -> Option<Kind>,
    seen: Seen,
    dirs: Vec<String>,
    budget: usize,
}

impl Resolver<'_> {
    fn kind(&mut self, key: &str) -> Option<Kind> {
        let answer = (self.lookup)(key);
        if !self.seen.iter().any(|(seen, _)| seen == key) {
            self.seen.push((key.to_owned(), answer));
        }
        answer
    }

    fn resolve(&mut self, text: &str) -> Option<Resolved> {
        if text.is_empty()
            || text.len() > MAX_SPAN_LEN
            || text.trim() != text
            || text.contains(['\n', '\r'])
            || text.contains("://")
            || text.starts_with(['~', '/', '\\'])
        {
            return None;
        }
        // The trailing slash of a directory the text introduces is presentation.
        let body = text.trim_end_matches('/');
        let head = without_location(body);
        let (path, line) = project_path(&format!("{}{}", encode(head), &body[head.len()..]))?;
        if self.budget == 0 {
            return None;
        }
        self.budget -= 1;
        match self.kind(&path) {
            Some(Kind::File) => {
                return Some(Resolved::File {
                    dest: destination(&path, line),
                });
            }
            Some(Kind::Dir) => return Some(Resolved::Dir(path)),
            _ => {}
        }
        for dir in self.dirs.clone() {
            let Some((joined, _)) = project_path(&destination(&format!("{dir}/{path}"), None))
            else {
                continue;
            };
            if self.kind(&joined) == Some(Kind::File) {
                return Some(Resolved::File {
                    dest: destination(&joined, line),
                });
            }
        }
        None
    }
}

/// `block` with every code span that names an existing project file rewritten into a link.
pub fn link_block(block: &Block, lookup: &dyn Fn(&str) -> Option<Kind>) -> Linked {
    let mut spans = Vec::new();
    visit(block, &mut |runs| {
        spans.extend(plain_code_spans(runs).map(|(_, text)| text.to_owned()));
    });
    let labels = span_labels(&spans);
    let mut resolver = Resolver {
        lookup,
        seen: Vec::new(),
        dirs: Vec::new(),
        budget: MAX_SPANS,
    };
    let block = rewrite(block, &mut |runs| {
        let mut linked: Option<Vec<InlineRun>> = None;
        for (ix, text) in plain_code_spans(runs) {
            match resolver.resolve(text) {
                Some(Resolved::File { dest }) => {
                    let out = linked.get_or_insert_with(|| runs.to_vec());
                    out[ix].style.code = false;
                    out[ix].style.link = Some(dest);
                    out[ix].text = labels
                        .get(text)
                        .map_or_else(|| trailing(text, 1), String::as_str)
                        .to_owned();
                }
                Some(Resolved::Dir(dir))
                    if resolver.dirs.len() < MAX_DIRS && !resolver.dirs.contains(&dir) =>
                {
                    resolver.dirs.push(dir);
                }
                Some(Resolved::Dir(_)) | None => {}
            }
        }
        linked
    });
    Linked {
        block,
        seen: resolver.seen,
    }
}

/// Only a plain code span stands for a file: an author's link, image or task marker already
/// means something else.
fn plain_code_spans(runs: &[InlineRun]) -> impl Iterator<Item = (usize, &str)> {
    runs.iter().enumerate().filter_map(|(ix, run)| {
        let style = &run.style;
        (style.code && style.link.is_none() && style.image.is_none() && style.task.is_none())
            .then_some((ix, run.text.as_str()))
    })
}

/// Calls `f` on every inline run list of `block`, in document order.
fn visit(block: &Block, f: &mut dyn FnMut(&[InlineRun])) {
    match block {
        Block::Paragraph { runs } | Block::Heading { runs, .. } => f(runs),
        Block::BlockQuote { children } => children.iter().for_each(|child| visit(child, f)),
        Block::List { items, .. } => items.iter().flatten().for_each(|child| visit(child, f)),
        Block::Table { header, rows, .. } => {
            header.iter().for_each(|cell| f(cell));
            rows.iter().flatten().for_each(|cell| f(cell));
        }
        Block::CodeBlock { .. } | Block::Rule => {}
    }
}

type RunsFn<'a> = dyn FnMut(&[InlineRun]) -> Option<Vec<InlineRun>> + 'a;

/// `block` with every run list `f` returned a replacement for; `None` when `f` changed nothing.
fn rewrite(block: &Block, f: &mut RunsFn<'_>) -> Option<Block> {
    let mut changed = false;
    let out = match block {
        Block::Paragraph { runs } => Block::Paragraph {
            runs: rewrite_runs(runs, f, &mut changed),
        },
        Block::Heading { level, runs } => Block::Heading {
            level: *level,
            runs: rewrite_runs(runs, f, &mut changed),
        },
        Block::BlockQuote { children } => Block::BlockQuote {
            children: rewrite_all(children, f, &mut changed),
        },
        Block::List {
            ordered_start,
            items,
        } => Block::List {
            ordered_start: *ordered_start,
            items: items
                .iter()
                .map(|item| rewrite_all(item, f, &mut changed))
                .collect(),
        },
        Block::Table {
            header,
            rows,
            align,
        } => Block::Table {
            header: header
                .iter()
                .map(|cell| rewrite_runs(cell, f, &mut changed))
                .collect(),
            rows: rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| rewrite_runs(cell, f, &mut changed))
                        .collect()
                })
                .collect(),
            align: align.clone(),
        },
        Block::CodeBlock { .. } | Block::Rule => return None,
    };
    changed.then_some(out)
}

fn rewrite_runs(runs: &[InlineRun], f: &mut RunsFn<'_>, changed: &mut bool) -> Vec<InlineRun> {
    match f(runs) {
        Some(linked) => {
            *changed = true;
            linked
        }
        None => runs.to_vec(),
    }
}

fn rewrite_all(blocks: &[Block], f: &mut RunsFn<'_>, changed: &mut bool) -> Vec<Block> {
    blocks
        .iter()
        .map(|block| match rewrite(block, f) {
            Some(linked) => {
                *changed = true;
                linked
            }
            None => block.clone(),
        })
        .collect()
}

/// The last `components` path components of `span` (all of it when it has fewer).
fn trailing(span: &str, components: usize) -> &str {
    span.rmatch_indices('/')
        .nth(components - 1)
        .map_or(span, |(at, _)| &span[at + 1..])
}

/// Labels for spans whose file name a different path in the same block shares: each grows
/// trailing components until no other path ends the same way (`ui/src/lib.rs` beside
/// `engine/src/lib.rs`), keeping its `:line` suffix. Every other span shows its file name.
fn span_labels(spans: &[String]) -> HashMap<&str, String> {
    let mut by_name: HashMap<&str, Vec<&str>> = HashMap::new();
    for span in spans {
        let path = without_location(span);
        let same = by_name.entry(trailing(path, 1)).or_default();
        if !same.contains(&path) {
            same.push(path);
        }
    }
    let mut labels = HashMap::new();
    for span in spans {
        let path = without_location(span);
        let same = &by_name[trailing(path, 1)];
        if same.len() < 2 {
            continue;
        }
        let shared = |tail: &str| {
            same.iter().any(|&other| {
                other != path
                    && other
                        .strip_suffix(tail)
                        .is_some_and(|rest| rest.is_empty() || rest.ends_with('/'))
            })
        };
        let components = (1..)
            .find(|&n| {
                let tail = trailing(path, n);
                tail == path || !shared(tail)
            })
            .unwrap_or(1);
        // The location suffix holds no `/`, so the span's own tail keeps it.
        labels.insert(span.as_str(), trailing(span, components).to_owned());
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_markdown::parse_full;

    /// A lookup over a fixed listing: files and directories by project path.
    fn listing<'a>(
        files: &'a [&'a str],
        dirs: &'a [&'a str],
    ) -> impl Fn(&str) -> Option<Kind> + 'a {
        move |key| {
            Some(if files.contains(&key) {
                Kind::File
            } else if dirs.contains(&key) {
                Kind::Dir
            } else {
                Kind::Missing
            })
        }
    }

    /// Every run of the first block, as (text, link).
    fn linked_runs(
        source: &str,
        lookup: &dyn Fn(&str) -> Option<Kind>,
    ) -> Vec<(String, Option<String>)> {
        let tree = parse_full(source);
        let linked = link_block(&tree.blocks[0].block, lookup);
        let block = linked.block.unwrap_or_else(|| tree.blocks[0].block.clone());
        let mut out = Vec::new();
        visit(&block, &mut |runs| {
            out.extend(
                runs.iter()
                    .map(|run| (run.text.clone(), run.style.link.clone())),
            )
        });
        out
    }

    fn link_of(runs: &[(String, Option<String>)], label: &str) -> Option<String> {
        runs.iter()
            .find(|(text, _)| text == label)
            .and_then(|(_, link)| link.clone())
    }

    #[test]
    fn an_existing_file_links_under_its_file_name_and_the_rest_stays_code() {
        let lookup = listing(&["src/main.rs", "SOURCES.md"], &[]);
        let runs = linked_runs("see `src/main.rs:12`, `SOURCES.md` and `nope.rs`", &lookup);
        assert_eq!(
            link_of(&runs, "main.rs:12").as_deref(),
            Some("src/main.rs:12")
        );
        assert_eq!(link_of(&runs, "SOURCES.md").as_deref(), Some("SOURCES.md"));
        assert_eq!(link_of(&runs, "nope.rs"), None);
    }

    #[test]
    fn a_directory_span_is_context_for_the_bare_names_after_it() {
        let lookup = listing(&["notes/day/SOURCES.md"], &["notes/day"]);
        let runs = linked_runs("under `notes/day/`: `SOURCES.md`, `other.md`", &lookup);
        assert_eq!(
            link_of(&runs, "SOURCES.md").as_deref(),
            Some("notes/day/SOURCES.md")
        );
        assert_eq!(
            link_of(&runs, "notes/day/"),
            None,
            "a directory is not a link"
        );
        assert_eq!(link_of(&runs, "other.md"), None);
        // Before the directory span, the bare name has no context.
        let runs = linked_runs("`SOURCES.md` then `notes/day/`", &lookup);
        assert_eq!(link_of(&runs, "SOURCES.md"), None);
    }

    #[test]
    fn paths_sharing_a_file_name_show_enough_of_the_path_to_differ() {
        let lookup = listing(&["ui/src/lib.rs", "engine/src/lib.rs", "other/mod.rs"], &[]);
        let runs = linked_runs(
            "`ui/src/lib.rs` `engine/src/lib.rs:3` `other/mod.rs`",
            &lookup,
        );
        assert_eq!(
            link_of(&runs, "ui/src/lib.rs").as_deref(),
            Some("ui/src/lib.rs")
        );
        assert_eq!(
            link_of(&runs, "engine/src/lib.rs:3").as_deref(),
            Some("engine/src/lib.rs:3")
        );
        assert_eq!(link_of(&runs, "mod.rs").as_deref(), Some("other/mod.rs"));
    }

    #[test]
    fn labels_grow_only_as_far_as_a_collision_needs() {
        let spans: Vec<String> = [
            "a/x/lib.rs",
            "b/x/lib.rs",
            "c/y/lib.rs",
            "a/x/lib.rs:9",
            "z.rs",
        ]
        .map(String::from)
        .into();
        let labels = span_labels(&spans);
        assert_eq!(labels["a/x/lib.rs"], "a/x/lib.rs");
        assert_eq!(labels["b/x/lib.rs"], "b/x/lib.rs");
        assert_eq!(labels["c/y/lib.rs"], "y/lib.rs");
        // The same path at another line does not collide with itself.
        assert_eq!(labels["a/x/lib.rs:9"], "a/x/lib.rs:9");
        assert!(!labels.contains_key("z.rs"));
    }

    #[test]
    fn spans_the_link_policy_refuses_are_never_looked_up() {
        let asked = std::cell::RefCell::new(Vec::new());
        let lookup = |key: &str| {
            asked.borrow_mut().push(key.to_owned());
            Some(Kind::File)
        };
        for span in [
            "../escape.md",
            "a/../../x",
            "/etc/passwd",
            "C:\\x.exe",
            "c:/x",
            "\\\\server\\x",
            "~/x",
            "https://example.com/a.md",
            "javascript:alert(1)",
            "a\\b.txt",
            "CON",
        ] {
            let runs = linked_runs(&format!("`{span}`"), &lookup);
            assert!(runs.iter().all(|(_, link)| link.is_none()), "{span}");
        }
        assert!(asked.borrow().is_empty(), "{:?}", asked.borrow());
    }

    #[test]
    fn unanswered_paths_are_reported_and_leave_the_span_plain() {
        let tree = parse_full("`a.md` and `dir/` then `b.md`");
        let linked = link_block(&tree.blocks[0].block, &|_| None);
        assert!(linked.block.is_none());
        assert_eq!(
            linked.unknown().collect::<Vec<_>>(),
            ["a.md", "dir", "b.md"]
        );
    }

    #[test]
    fn spans_in_lists_quotes_and_tables_link_too() {
        let lookup = listing(&["a.md"], &[]);
        for source in [
            "- `a.md`",
            "> `a.md`",
            "| h |\n|---|\n| `a.md` |",
            "# `a.md`",
        ] {
            let runs = linked_runs(source, &lookup);
            assert_eq!(link_of(&runs, "a.md").as_deref(), Some("a.md"), "{source}");
        }
    }

    #[test]
    fn a_name_with_percent_or_hash_round_trips_through_the_link_policy() {
        let lookup = listing(&["100%.txt"], &[]);
        let runs = linked_runs("`100%.txt`", &lookup);
        let dest = link_of(&runs, "100%.txt").expect("linked");
        assert_eq!(dest, "100%25.txt");
        assert_eq!(project_path(&dest), Some(("100%.txt".to_owned(), None)));
    }

    #[test]
    fn probes_keep_what_they_were_given_and_ignore_an_earlier_project() {
        let mut probes = Probes::default();
        assert_eq!(probes.claim(["a", "b", "a"].into_iter()), ["a", "b"]);
        assert!(
            probes.claim(["a"].into_iter()).is_empty(),
            "already in flight"
        );
        let generation = probes.generation();
        assert!(probes.accept(generation, vec![("a".into(), Kind::File)]));
        assert_eq!(probes.get("a"), Some(Kind::File));
        assert!(probes.claim(["a"].into_iter()).is_empty(), "answered");
        probes.clear();
        assert!(!probes.accept(generation, vec![("b".into(), Kind::File)]));
        assert_eq!(probes.get("a"), None);
        assert_eq!(probes.get("b"), None);
        assert_eq!(probes.claim(["a"].into_iter()), ["a"]);
    }

    #[test]
    fn a_full_row_hands_out_no_more_probes() {
        let mut probes = Probes::default();
        let keys: Vec<String> = (0..MAX_KNOWN + 10).map(|n| format!("f{n}")).collect();
        let claimed = probes.claim(keys.iter().map(String::as_str));
        assert_eq!(claimed.len(), MAX_KNOWN);
        let generation = probes.generation();
        let answers = claimed
            .into_iter()
            .map(|key| (key, Kind::Missing))
            .collect();
        assert!(probes.accept(generation, answers));
        // The refused keys are not probed now or on a later pass.
        assert!(probes.claim(keys.iter().map(String::as_str)).is_empty());
        assert!(probes.claim(keys.iter().map(String::as_str)).is_empty());
        assert_eq!(probes.get("f0"), Some(Kind::Missing));
        assert_eq!(probes.get(&keys[MAX_KNOWN]), None);
    }

    #[test]
    fn the_disk_check_confines_to_the_project() {
        let outside = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let root = project.path();
        std::fs::create_dir(root.join("tools")).unwrap();
        std::fs::write(root.join("tools").join("x.cmd"), "x").unwrap();
        std::fs::write(outside.path().join("secret.md"), "x").unwrap();
        assert_eq!(kind_on_disk("tools/x.cmd", root), Kind::File);
        assert_eq!(kind_on_disk("tools", root), Kind::Dir);
        assert_eq!(kind_on_disk("tools/none.cmd", root), Kind::Missing);
        // Paths the policy refuses never reach the disk.
        for key in ["../secret.md", "..", "/etc/passwd", "C:\\x", ""] {
            assert_eq!(kind_on_disk(key, root), Kind::Missing, "{key}");
        }
        let absolute = outside.path().join("secret.md");
        assert_eq!(
            kind_on_disk(&absolute.to_string_lossy(), root),
            Kind::Missing
        );
        // A link that points outside is not a file of the project.
        let link = root.join("link.md");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(outside.path().join("secret.md"), &link);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(outside.path().join("secret.md"), &link);
        match made {
            Ok(()) => assert_eq!(kind_on_disk("link.md", root), Kind::Missing),
            Err(e) => eprintln!("skipping symlink check: cannot create symlinks here ({e})"),
        }
    }
}
