//! Reading and replacing `timeline.json` (or another run-time document, see [`DocFile`])
//! without a build, a journal record or a lost outside write.
//!
//! The publication primitives have no single-step replace, so a write publishes the new
//! bytes under a private name in `.fframes/cache/timing/` (an excluded tree: neither the
//! source revision nor the watcher sees it), checks that the live file is still the one
//! the session last loaded or wrote, displaces it into the cache and publishes the new
//! file in its place. Both moves are atomic no-replace renames, so a file an editor or an
//! agent put there in between is never overwritten. A crash between the two moves leaves
//! the new bytes staged in the cache; [`recover`] adopts them when the project opens.
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

use studio_project::{
    publish::{Dir, FileId, NoClobber, fstat, hard_link_refusal},
    revision::TIMING_FILE,
};

use super::doc::TimingDoc;

/// Where staged and displaced copies live, relative to the project root.
pub const TIMING_CACHE: &str = ".fframes/cache/timing";
/// Displaced copies kept for manual recovery; older ones are removed.
pub const KEPT_COPIES: usize = 8;
const STAGED: &str = ".staged.json";
const DISPLACED: &str = ".previous.json";

/// A document at the project root that Studio writes this way.
#[derive(Debug, Clone, Copy)]
pub struct DocFile {
    /// Its name at the project root.
    pub name: &'static str,
    /// Its cache directory below the project root, in an excluded tree.
    pub cache: &'static str,
    /// Larger files are not read as a document.
    pub max_bytes: usize,
    /// What a conflict says was reloaded ("the timeline").
    pub view: &'static str,
    /// Whether staged bytes are a document [`recover_doc`] may publish.
    pub valid: fn(&str) -> bool,
}

/// `timeline.json`.
pub const TIMING: DocFile = DocFile {
    name: TIMING_FILE,
    cache: TIMING_CACHE,
    max_bytes: fframes_studio_protocol::MAX_TIMING_DOC_BYTES,
    view: "the timeline",
    valid: |json| TimingDoc::from_json(json).is_ok(),
};

/// The live document as the session last loaded or wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFile {
    pub id: FileId,
    pub sha256: String,
}

/// The live file: its bytes (as text when they are UTF-8) and identity.
pub struct LiveFile {
    pub text: Result<String, String>,
    pub known: KnownFile,
}

/// Why a write did not replace the file.
#[derive(Debug)]
pub enum WriteError {
    /// The live file is not the one the session knows: something else wrote it. Nothing
    /// was replaced; the session reloads it.
    Conflict(String),
    Io(io::Error),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict(reason) => f.write_str(reason),
            // `write_doc` names the file in the error.
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl From<io::Error> for WriteError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// The points of a write a test can interrupt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStep {
    /// The new bytes are staged; the live file is untouched.
    Staged,
    /// The live file was moved into the cache; nothing is published yet.
    Displaced,
}

/// Reads the live `timeline.json`; `None` when the project has none.
pub fn read(root: &Path) -> io::Result<Option<LiveFile>> {
    read_doc(root, &TIMING)
}

/// Reads the live `doc`; `None` when the project has none.
pub fn read_doc(root: &Path, doc: &DocFile) -> io::Result<Option<LiveFile>> {
    let name = doc.name;
    let root = Dir::open_root(root)?;
    if root.stat(name)?.is_none() {
        return Ok(None);
    }
    let (file, stat) = match root.open_regular(name) {
        Ok(opened) => opened,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let limit = doc.max_bytes as u64 + 1;
    let mut bytes = Vec::new();
    (&file).take(limit).read_to_end(&mut bytes)?;
    let after = fstat(&file)?;
    if after.id != stat.id || after.modified_ns != stat.modified_ns || after.size != stat.size {
        return Err(io::Error::other(format!(
            "{name} changed while it was being read"
        )));
    }
    let text = if bytes.len() as u64 >= limit {
        Err(format!("{name} exceeds {} bytes", doc.max_bytes))
    } else {
        String::from_utf8(bytes.clone()).map_err(|_| format!("{name} is not UTF-8"))
    };
    let sha256 = if bytes.len() as u64 >= limit {
        // Too large to be a document: identify it by its full contents all the same.
        root.hash_regular(name)?.1
    } else {
        sha256(&bytes)
    };
    Ok(Some(LiveFile {
        text,
        known: KnownFile {
            id: stat.id,
            sha256,
        },
    }))
}

pub fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!("{:x}", sha2::Sha256::digest(bytes))
}

/// Opens (creating as needed) the cache directory below the held root.
fn cache_dir(root: &Dir, cache: &str) -> io::Result<Dir> {
    let mut dir = root.try_clone()?;
    for part in cache.split('/') {
        dir = match dir.open_child(part) {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match dir.mkdir(part, 0o755) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error),
                }
                dir.open_child(part)?
            }
            Err(error) => return Err(error),
        };
    }
    Ok(dir)
}

/// A name that sorts by time of creation.
fn stamp() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!(
        "{nanos:024}-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

/// Replaces the live `timeline.json` with `json` when it is still `expected` (`None`: the
/// project has no timing document). Returns the identity of the published file.
pub fn write(
    root_path: &Path,
    json: &str,
    expected: Option<&KnownFile>,
    observe: &mut dyn FnMut(WriteStep) -> io::Result<()>,
) -> Result<KnownFile, WriteError> {
    write_doc(root_path, &TIMING, json, expected, observe)
}

/// Replaces the live `doc` with `json` when it is still `expected` (`None`: the project
/// has none). Returns the identity of the published file.
pub fn write_doc(
    root_path: &Path,
    doc: &DocFile,
    json: &str,
    expected: Option<&KnownFile>,
    observe: &mut dyn FnMut(WriteStep) -> io::Result<()>,
) -> Result<KnownFile, WriteError> {
    replace(root_path, doc, json, expected, observe).map_err(|error| match error {
        WriteError::Io(error) => WriteError::Io(io::Error::new(
            error.kind(),
            format!("{} could not be written: {error}", doc.name),
        )),
        conflict => conflict,
    })
}

fn replace(
    root_path: &Path,
    doc: &DocFile,
    json: &str,
    expected: Option<&KnownFile>,
    observe: &mut dyn FnMut(WriteStep) -> io::Result<()>,
) -> Result<KnownFile, WriteError> {
    let name = doc.name;
    let root = Dir::open_root(root_path)?;
    let cache = cache_dir(&root, doc.cache)?;
    let stamp = stamp();
    let staged = format!("{stamp}{STAGED}");
    let mut file = cache.create_new(&staged, 0o644)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    drop(file);
    cache.sync()?;
    observe(WriteStep::Staged)?;

    let conflict = || {
        WriteError::Conflict(format!(
            "{name} changed outside the studio; the outside version was kept and {} reloaded",
            doc.view
        ))
    };
    match (root.stat(name)?, expected) {
        (None, None) => {}
        (Some(_), Some(expected)) => {
            let (stat, hash) = root.hash_regular(name)?;
            if let Some(reason) = hard_link_refusal(&stat) {
                return Err(WriteError::Conflict(format!("{name} {reason}")));
            }
            if stat.id != expected.id || hash != expected.sha256 {
                return Err(conflict());
            }
            let displaced = format!("{stamp}{DISPLACED}");
            root.rename_noreplace(NoClobber::Renameat2, name, &cache, &displaced)?;
            // The rename moved whatever the name held at that instant: put back a file
            // that is not the one just verified.
            let (moved, moved_hash) = cache.hash_regular(&displaced)?;
            if moved.id != expected.id || moved_hash != expected.sha256 {
                let _ = cache.rename_noreplace(NoClobber::Renameat2, &displaced, &root, name);
                return Err(conflict());
            }
            observe(WriteStep::Displaced)?;
        }
        _ => return Err(conflict()),
    }
    match cache.rename_noreplace(NoClobber::Renameat2, &staged, &root, name) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Err(conflict()),
        Err(error) => return Err(error.into()),
    }
    root.sync()?;
    cache.sync()?;
    let (stat, hash) = root.hash_regular(name)?;
    if hash != sha256(json.as_bytes()) {
        return Err(conflict());
    }
    // Housekeeping only: a copy that cannot be removed now is removed next time.
    let _ = prune(&cache, &root_path.join(doc.cache));
    Ok(KnownFile {
        id: stat.id,
        sha256: hash,
    })
}

/// Names in the cache directory with `suffix`, oldest first.
fn copies(cache: &Path, suffix: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(cache)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(suffix))
        .collect();
    names.sort();
    names
}

/// Keeps the newest [`KEPT_COPIES`] displaced copies.
fn prune(cache: &Dir, cache_path: &Path) -> io::Result<()> {
    let displaced = copies(cache_path, DISPLACED);
    for name in &displaced[..displaced.len().saturating_sub(KEPT_COPIES)] {
        cache.unlink(name)?;
    }
    Ok(())
}

/// Completes a write that a crash interrupted: when the project has no `timeline.json`
/// but the cache holds the staged bytes of an unfinished write, the newest valid ones
/// are published. Staged leftovers next to a live file lost to it and are removed.
/// Returns a note when it recovered a document.
pub fn recover(root_path: &Path) -> io::Result<Option<String>> {
    recover_doc(root_path, &TIMING)
}

/// [`recover`] for `doc`.
pub fn recover_doc(root_path: &Path, doc: &DocFile) -> io::Result<Option<String>> {
    let (name, cache_name) = (doc.name, doc.cache);
    let cache_path = root_path.join(cache_name);
    if !cache_path.is_dir() {
        return Ok(None);
    }
    let root = Dir::open_root(root_path)?;
    let cache = cache_dir(&root, cache_name)?;
    let staged = copies(&cache_path, STAGED);
    if staged.is_empty() {
        return Ok(None);
    }
    let mut note = None;
    if root.stat(name)?.is_none() {
        // Newest first: the staged copy of the interrupted write, then displaced
        // copies, which are older documents.
        let mut candidates: Vec<String> = staged
            .iter()
            .chain(copies(&cache_path, DISPLACED).iter())
            .cloned()
            .collect();
        candidates.sort();
        for copy in candidates.iter().rev() {
            let valid = fs::read_to_string(cache_path.join(copy))
                .ok()
                .is_some_and(|json| (doc.valid)(&json));
            if !valid {
                continue;
            }
            match cache.rename_noreplace(NoClobber::Renameat2, copy, &root, name) {
                Ok(()) => {
                    root.sync()?;
                    note = Some(format!(
                        "{name} was restored from an interrupted save ({cache_name}/{copy})"
                    ));
                }
                // Something else created it meanwhile; that file stands.
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
            break;
        }
    }
    for copy in copies(&cache_path, STAGED) {
        cache.unlink(&copy)?;
    }
    cache.sync()?;
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "{\"version\":1,\"clips\":[]}\n";
    const B: &str = "{\"version\":1,\"clips\":[{\"key\":\"a\",\"trim_start_frames\":1,\"trim_end_frames\":0}]}\n";

    fn none(_: WriteStep) -> io::Result<()> {
        Ok(())
    }

    #[test]
    fn writes_create_then_replace_and_keep_copies_out_of_the_source() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("studio.json"), "{}").unwrap();
        let before = studio_project::SourceInventory::scan(root).unwrap();
        let first = write(root, A, None, &mut none).unwrap();
        assert_eq!(fs::read_to_string(root.join(TIMING_FILE)).unwrap(), A);
        let second = write(root, B, Some(&first), &mut none).unwrap();
        assert_eq!(fs::read_to_string(root.join(TIMING_FILE)).unwrap(), B);
        assert_eq!(read(root).unwrap().unwrap().known, second);
        let after = studio_project::SourceInventory::scan(root).unwrap();
        assert_eq!(
            after.files.len(),
            before.files.len() + 1,
            "only timeline.json is new"
        );
        assert_eq!(copies(&root.join(TIMING_CACHE), DISPLACED).len(), 1);
        assert!(copies(&root.join(TIMING_CACHE), STAGED).is_empty());
    }

    #[test]
    fn an_outside_write_is_never_replaced() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let known = write(root, A, None, &mut none).unwrap();
        fs::write(root.join(TIMING_FILE), "outside").unwrap();
        assert!(matches!(
            write(root, B, Some(&known), &mut none),
            Err(WriteError::Conflict(_))
        ));
        assert_eq!(
            fs::read_to_string(root.join(TIMING_FILE)).unwrap(),
            "outside"
        );
        // Created by someone else although the session saw none.
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join(TIMING_FILE), "outside").unwrap();
        assert!(matches!(
            write(other.path(), B, None, &mut none),
            Err(WriteError::Conflict(_))
        ));
    }

    #[test]
    fn a_crash_between_displace_and_publish_is_recovered_on_open() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let known = write(root, A, None, &mut none).unwrap();
        let crashed = write(root, B, Some(&known), &mut |step| match step {
            WriteStep::Displaced => Err(io::Error::other("crash")),
            WriteStep::Staged => Ok(()),
        });
        assert!(matches!(crashed, Err(WriteError::Io(_))));
        assert!(!root.join(TIMING_FILE).exists());
        let note = recover(root).unwrap().unwrap();
        assert!(note.contains("interrupted save"), "{note}");
        assert_eq!(fs::read_to_string(root.join(TIMING_FILE)).unwrap(), B);
        assert!(copies(&root.join(TIMING_CACHE), STAGED).is_empty());
        assert_eq!(recover(root).unwrap(), None);
    }

    #[test]
    fn staged_leftovers_lose_to_a_live_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let known = write(root, A, None, &mut none).unwrap();
        let crashed = write(root, B, Some(&known), &mut |step| match step {
            WriteStep::Staged => Err(io::Error::other("crash")),
            WriteStep::Displaced => Ok(()),
        });
        assert!(crashed.is_err());
        assert_eq!(recover(root).unwrap(), None);
        assert_eq!(fs::read_to_string(root.join(TIMING_FILE)).unwrap(), A);
        assert!(copies(&root.join(TIMING_CACHE), STAGED).is_empty());
    }

    #[test]
    fn displaced_copies_are_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut known = write(root, A, None, &mut none).unwrap();
        for n in 0..KEPT_COPIES + 3 {
            let json = format!(
                "{{\"version\":1,\"clips\":[{{\"key\":\"a\",\"trim_start_frames\":{n},\"trim_end_frames\":0}}]}}\n"
            );
            known = write(root, &json, Some(&known), &mut none).unwrap();
        }
        assert_eq!(
            copies(&root.join(TIMING_CACHE), DISPLACED).len(),
            KEPT_COPIES
        );
    }
}
