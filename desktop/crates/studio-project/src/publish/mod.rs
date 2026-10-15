//! Publication primitives for durable, no-clobber source mutation.
//!
//! Every operation is relative to a held directory handle ([`Dir`]) and takes a single
//! path component, so a path swap above the handle cannot redirect it, and no operation
//! follows a link. Moving a name that an editor controls (displacing an original, taking
//! a published file down again) uses one atomic no-replace rename only:
//!
//! - Linux: `renameat2(RENAME_NOREPLACE)`;
//! - macOS: `renameatx_np(RENAME_EXCL)`;
//! - Windows: `NtSetInformationFile(FileRenameInformationEx)` with `RootDirectory` set to
//!   the target directory and no `FILE_RENAME_REPLACE_IF_EXISTS`.
//!
//! A destination or a source that an uncooperative editor replaced is therefore never
//! overwritten or unlinked by mistake.
//!
//! [`NoClobber::Link`] (`linkat` then `unlinkat`) is **not** an equivalent. Between the
//! two calls an editor can save by renaming a new file over the source name, and the
//! `unlinkat` then deletes the editor's file while the link keeps the old inode alive,
//! so every later verification of the moved bytes still passes (see
//! `tests/publish.rs::link_then_unlink_loses_an_editor_save_between_the_two_syscalls`).
//! It is only safe where the *source* is a private, immutable name nobody else knows,
//! and the transaction layer never uses it to move a live source name:
//! [`probe_no_clobber`] qualifies the atomic rename alone and callers keep their Apply
//! gate blocked where it is unavailable. Windows does not offer the pair at all.
//!
//! [`Dir::open_root`] binds the root *pathname* (not only its identity) with a no-follow
//! walk from the filesystem root; [`Dir::check_root_binding`] re-walks it so a root
//! renamed away and replaced by a link is detected around each mutation and the final
//! verification.
//!
//! These primitives cannot exclude a malicious writer that holds an open handle; the
//! transaction layer detects such races by re-verifying identity and hash around each
//! step and retains every observed variant.
//!
//! # Windows differences
//!
//! - The rename opens the source first and renames *the opened file*, not the name. The
//!   source handle is opened without `FILE_SHARE_DELETE`, so between the open and the
//!   rename nobody can rename, replace or delete the source name.
//! - Links are refused by a check, not a flag: `FILE_OPEN_REPARSE_POINT` opens a reparse
//!   point itself, and every opened handle's attribute tag is then classified. Symlinks
//!   and junctions are [`EntryKind::Symlink`]; Cloud Files placeholders (OneDrive) are
//!   [`EntryKind::CloudPlaceholder`] and refused with a reason that asks the user to move
//!   the project to a local folder; every other reparse point is [`EntryKind::Other`].
//! - NTFS refuses to rename a directory while a handle below it is open, so an ancestor
//!   of a held root cannot be swapped at all.
//! - There are no permission bits. [`EntryStat::mode`] is synthesised from the read-only
//!   attribute (`0o444` read-only, `0o644` otherwise, `0o755` for directories), so the
//!   transaction layer's exact-mode checks compare the one bit Windows has; a file is
//!   never executable. Directory modes cannot be set ([`Dir::chmod`] does nothing).
//! - Network shares refuse the handle-relative rename; the probe fails there and Apply
//!   stays blocked.
use std::{
    fmt,
    io::{self, Read},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{Dir, fstat};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{Dir, fstat};

/// A volume identifier plus a file identifier: `(st_dev, st_ino)` on Unix,
/// `FILE_ID_INFORMATION` (`VolumeSerialNumber`, the 128-bit `FILE_ID_128`) on Windows.
///
/// Journals written before the identifier was widened name the fields `dev` and `ino`;
/// they still decode, and on Unix compare equal to a fresh `fstat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId {
    #[serde(alias = "dev")]
    pub volume: u64,
    #[serde(alias = "ino", with = "file_number")]
    pub file: u128,
}

impl fmt::Display for FileId {
    /// `volume:file` in decimal, the form journalled in progress details. Records written
    /// before the identifier was widened (`dev:ino`) read back unchanged.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.volume, self.file)
    }
}

impl std::str::FromStr for FileId {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (volume, file) = text.split_once(':').ok_or(())?;
        Ok(Self {
            volume: volume.parse().map_err(drop)?,
            file: file.parse().map_err(drop)?,
        })
    }
}

/// The 128-bit file number as a JSON number when it fits in 64 bits (every Unix inode,
/// and the shape older journals carry) and as a decimal string otherwise, so no reader
/// depends on 128-bit integer support in the JSON layer.
mod file_number {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        match u64::try_from(*value) {
            Ok(small) => serializer.serialize_u64(small),
            Err(_) => serializer.serialize_str(&value.to_string()),
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Number {
        Small(u64),
        Text(String),
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        match Number::deserialize(deserializer)? {
            Number::Small(small) => Ok(u128::from(small)),
            Number::Text(text) => text.parse().map_err(D::Error::custom),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Regular,
    Directory,
    /// A symlink, or on Windows a symlink or junction (mount point) reparse point.
    Symlink,
    /// A Cloud Files placeholder (OneDrive Files On-Demand and other providers).
    CloudPlaceholder,
    /// Anything else: FIFO, socket, device, or a reparse point of another kind.
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryStat {
    pub id: FileId,
    pub kind: EntryKind,
    pub size: u64,
    /// Permission bits including setuid/setgid/sticky (`st_mode & 0o7777`); synthesised
    /// from the read-only attribute on Windows (see the module docs).
    pub mode: u32,
    pub nlink: u64,
    /// Last data modification (`st_mtim`, `LastWriteTime`), nanoseconds since the
    /// platform's epoch.
    pub modified_ns: i128,
    /// Last metadata change (`st_ctim`, `ChangeTime`: writes, mode changes, renames),
    /// nanoseconds since the platform's epoch.
    pub changed_ns: i128,
    /// The raw reparse tag on Windows (0 when the entry is no reparse point); 0 on Unix.
    pub reparse_tag: u32,
}

impl EntryStat {
    pub fn is_regular(&self) -> bool {
        self.kind == EntryKind::Regular
    }
    pub fn executable(&self) -> bool {
        self.mode & 0o111 != 0
    }
}

/// How a staged file is published without replacing an existing destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoClobber {
    /// The platform's atomic no-replace rename (the serialized name predates the other
    /// platforms). The only mechanism that may move a name an editor controls.
    Renameat2,
    /// `linkat` then `unlinkat`: never replaces the destination, but is two syscalls: an
    /// editor saving by rename between them has its file unlinked. Only for sources that
    /// are private immutable names (never a live project file). Unsupported on Windows.
    Link,
}

impl NoClobber {
    pub fn label(self) -> &'static str {
        match self {
            Self::Renameat2 if cfg!(windows) => {
                "NtSetInformationFile(FileRenameInformationEx) without replace"
            }
            Self::Renameat2 if cfg!(target_os = "macos") => "renameatx_np(RENAME_EXCL)",
            Self::Renameat2 => "renameat2(RENAME_NOREPLACE)",
            Self::Link => "linkat+unlinkat",
        }
    }
}

/// Validates a single path component: no separators, no `.`/`..`, no NUL. On Windows also
/// no `:` (alternate data streams), no character Win32 cannot name, and no trailing dot or
/// space (NT would create a name Win32 tools cannot open).
pub(crate) fn check_component(name: &str) -> io::Result<()> {
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\0'])
        || (cfg!(windows)
            && (name.contains(['\\', ':', '*', '?', '"', '<', '>', '|'])
                || name.chars().any(|c| u32::from(c) < 0x20)
                || name.ends_with(['.', ' '])));
    if invalid {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name:?} is not a single path component"),
        ))
    } else {
        Ok(())
    }
}

/// Why an entry cannot be opened or moved as `wanted`, or `None` when it is that kind.
#[cfg(windows)]
pub(crate) fn refusal(name: &str, stat: &EntryStat, wanted: EntryKind) -> Option<io::Error> {
    if stat.kind == wanted {
        return None;
    }
    let what = match stat.kind {
        EntryKind::Symlink => "a link (symlink or junction); links are never followed".to_owned(),
        EntryKind::CloudPlaceholder => format!(
            "a cloud-files placeholder (reparse tag {:#010x}, e.g. OneDrive Files On-Demand); project files are never changed inside a cloud-synced folder because the sync provider can replace or dehydrate them underneath the transaction. Move the project to a local folder",
            stat.reparse_tag
        ),
        EntryKind::Other if stat.reparse_tag != 0 => format!(
            "a reparse point of an unsupported kind (tag {:#010x})",
            stat.reparse_tag
        ),
        EntryKind::Other => "a special file".to_owned(),
        EntryKind::Regular => "a regular file".to_owned(),
        EntryKind::Directory => "a directory".to_owned(),
    };
    let kind = match stat.kind {
        EntryKind::Directory => io::ErrorKind::IsADirectory,
        _ if wanted == EntryKind::Directory => io::ErrorKind::NotADirectory,
        _ => io::ErrorKind::InvalidInput,
    };
    Some(io::Error::new(kind, format!("{name:?} is {what}")))
}

/// Hard links make a file reachable under names the transaction does not control: a
/// write through another name would change bytes the transaction verified, and moving
/// one name aside leaves the bytes live elsewhere. Such files are never displaced or
/// published over.
pub fn hard_link_refusal(stat: &EntryStat) -> Option<String> {
    (stat.kind == EntryKind::Regular && stat.nlink > 1).then(|| {
        format!(
            "has {} hard links; a file reachable under other names is never replaced or removed",
            stat.nlink
        )
    })
}

/// SHA-256 and length of everything readable from `reader`.
pub fn hash_reader(reader: &mut impl Read) -> io::Result<(String, u64)> {
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        size += count as u64;
        digest.update(&buffer[..count]);
    }
    Ok((format!("{:x}", digest.finalize()), size))
}

/// Operations that are the same on every platform, built on the per-OS primitives.
impl Dir {
    /// Descends `relative` (slash separated, empty = this directory) one no-follow
    /// component at a time.
    pub fn open_relative(&self, relative: &str) -> io::Result<Self> {
        let mut current = self.try_clone()?;
        if relative.is_empty() {
            return Ok(current);
        }
        for part in relative.split('/') {
            current = current.open_child(part)?;
        }
        Ok(current)
    }

    /// Whether `relative` below `root` still resolves (without following links) to this
    /// very directory. A swapped directory or link makes this false.
    pub fn is_still_at(&self, root: &Dir, relative: &str) -> bool {
        root.open_relative(relative)
            .is_ok_and(|current| current.id() == self.id())
    }

    /// Hash of a regular file's current bytes together with its identity and mode.
    ///
    /// The handle is stat'ed before and after the read: a concurrent in-place write
    /// (even one that keeps the length), a mode change or a replaced file makes the
    /// result an error instead of a hash that describes no single state of the file.
    pub fn hash_regular(&self, name: &str) -> io::Result<(EntryStat, String)> {
        self.hash_regular_observed(name, || {})
    }

    /// [`Self::hash_regular`] with a seam: `after_read` runs once all bytes were read and
    /// before the final stat, so a test can interleave a concurrent writer exactly there.
    pub fn hash_regular_observed(
        &self,
        name: &str,
        after_read: impl FnOnce(),
    ) -> io::Result<(EntryStat, String)> {
        let (mut file, stat) = self.open_regular(name)?;
        let (hash, size) = hash_reader(&mut file)?;
        after_read();
        let after = fstat(&file)?;
        if size != after.size
            || after.id != stat.id
            || after.mode != stat.mode
            || after.modified_ns != stat.modified_ns
            || after.changed_ns != stat.changed_ns
        {
            return Err(io::Error::other("file changed while it was being read"));
        }
        Ok((EntryStat { size, ..stat }, hash))
    }

    /// Moves `from` (in this directory) to `to` in `target` without replacing an
    /// existing `to`. An existing destination is `ErrorKind::AlreadyExists` and leaves
    /// both names untouched.
    ///
    /// `NoClobber::Link` must only be used when `from` is a private name no editor can
    /// touch: see the module documentation.
    pub fn rename_noreplace(
        &self,
        mechanism: NoClobber,
        from: &str,
        target: &Dir,
        to: &str,
    ) -> io::Result<()> {
        self.rename_noreplace_observed(mechanism, from, target, to, || {})
    }
}

/// Proves on the filesystem of `dir` that moving a name an editor controls never
/// replaces an existing name: an atomic no-replace rename onto an existing file must
/// fail with `AlreadyExists` leaving both files byte-identical, and onto a free name it
/// must move the file. Only the atomic rename qualifies: the link-based pair is two
/// syscalls and can unlink an editor's replacement (see the module documentation), so
/// there is no fallback; the caller keeps Apply blocked on `Err`. The probe files carry
/// exact app-generated names and are removed again.
pub fn probe_no_clobber(dir: &Dir) -> Result<NoClobber, String> {
    match probe_mechanism(dir, NoClobber::Renameat2) {
        Ok(()) => Ok(NoClobber::Renameat2),
        Err(reason) => Err(format!(
            "this filesystem cannot move project files with an atomic no-replace rename ({reason}); a link-then-unlink fallback could delete an editor's replacement saved between its two calls and is never used, so Apply stays blocked"
        )),
    }
}

/// Proves one specific mechanism on the filesystem of `dir` (see [`probe_no_clobber`]).
pub fn probe_mechanism(dir: &Dir, mechanism: NoClobber) -> Result<(), String> {
    let tag = uuid::Uuid::new_v4().simple().to_string();
    let names: Vec<String> = (0..3)
        .map(|i| format!("{}{tag}-{i}.stage", crate::paths::TX_PREFIX))
        .collect();
    let result = probe_with(dir, mechanism, &names);
    for name in &names {
        let _ = dir.unlink(name);
    }
    let _ = dir.sync();
    result.map_err(|reason| format!("{}: {reason}", mechanism.label()))
}

/// Whether an error means the filesystem does not offer the primitive at all.
fn is_unsupported(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::Unsupported {
        return true;
    }
    #[cfg(unix)]
    // macOS answers `RENAME_EXCL` on volumes without it (SMB, some FUSE) with ENOTSUP,
    // which differs from EOPNOTSUPP there; on Linux they are the same number.
    let codes = [
        libc::ENOSYS,
        libc::EINVAL,
        libc::EOPNOTSUPP,
        libc::ENOTSUP,
        libc::EPERM,
    ];
    // ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED, ERROR_BAD_LENGTH and
    // ERROR_INVALID_PARAMETER: SMB answers the handle-relative rename with the last two.
    #[cfg(windows)]
    let codes = [1, 50, 24, 87];
    error
        .raw_os_error()
        .is_some_and(|code| codes.contains(&code))
}

fn probe_with(dir: &Dir, mechanism: NoClobber, names: &[String]) -> Result<(), String> {
    use std::io::Write;
    let describe = |step: &str, error: io::Error| format!("{step}: {error}");
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let mut file = dir
            .create_new(name, 0o600)
            .map_err(|e| describe("create probe", e))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| describe("write probe", e))
    };
    write(&names[0], b"staged")?;
    write(&names[1], b"existing")?;
    let staged_id = dir
        .stat(&names[0])
        .map_err(|e| describe("stat probe", e))?
        .ok_or("probe file vanished")?
        .id;
    match dir.rename_noreplace(mechanism, &names[0], dir, &names[1]) {
        Ok(()) => return Err("an existing destination was replaced".into()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) if is_unsupported(&e) => return Err(format!("not supported ({e})")),
        Err(e) => return Err(describe("refusing to replace", e)),
    }
    let read = |name: &str| -> Result<Vec<u8>, String> {
        let (mut file, _) = dir.open_regular(name).map_err(|e| describe("reopen", e))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| describe("reread", e))?;
        Ok(bytes)
    };
    if read(&names[0])? != b"staged" || read(&names[1])? != b"existing" {
        return Err("a refused publication still changed a file".into());
    }
    dir.rename_noreplace(mechanism, &names[0], dir, &names[2])
        .map_err(|e| describe("publish onto a free name", e))?;
    let moved = dir
        .stat(&names[2])
        .map_err(|e| describe("stat published", e))?
        .ok_or("published file is missing")?;
    if dir
        .stat(&names[0])
        .map_err(|e| describe("stat source", e))?
        .is_some()
        || moved.id != staged_id
        || read(&names[2])? != b"staged"
    {
        return Err("publication onto a free name did not move the staged inode".into());
    }
    Ok(())
}

/// Whether a Windows reparse tag belongs to the Cloud Files API (`IO_REPARSE_TAG_CLOUD`
/// and `IO_REPARSE_TAG_CLOUD_1` .. `IO_REPARSE_TAG_CLOUD_F`, which differ only in bits
/// 12..15).
pub fn is_cloud_reparse_tag(tag: u32) -> bool {
    const IO_REPARSE_TAG_CLOUD: u32 = 0x9000_001A;
    const CLOUD_MASK: u32 = 0x0000_F000;
    tag & !CLOUD_MASK == IO_REPARSE_TAG_CLOUD
}

/// Classifies a Windows reparse point by its tag.
pub fn classify_reparse_tag(tag: u32) -> EntryKind {
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
    if is_cloud_reparse_tag(tag) {
        EntryKind::CloudPlaceholder
    } else if tag == IO_REPARSE_TAG_SYMLINK || tag == IO_REPARSE_TAG_MOUNT_POINT {
        EntryKind::Symlink
    } else {
        EntryKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_ids_round_trip_wide_values_and_read_the_older_dev_ino_shape() {
        let wide = FileId {
            volume: 0x8c64_df92_64df_7e00,
            file: 0x0003_0000_0014_2a7f_0000_0000_0000_0001,
        };
        let json = serde_json::to_string(&wide).unwrap();
        assert_eq!(serde_json::from_str::<FileId>(&json).unwrap(), wide);
        let small = FileId {
            volume: 2049,
            file: 1234,
        };
        assert_eq!(
            serde_json::to_string(&small).unwrap(),
            r#"{"volume":2049,"file":1234}"#
        );
        let older: FileId = serde_json::from_str(r#"{"dev":2049,"ino":1234}"#).unwrap();
        assert_eq!(older, small);
        // The journalled text form: older `dev:ino` details parse into the same id.
        assert_eq!("2049:1234".parse::<FileId>(), Ok(small));
        assert_eq!(wide.to_string().parse::<FileId>(), Ok(wide));
        assert!("2049".parse::<FileId>().is_err());
    }

    #[test]
    fn cloud_tags_are_classified_as_placeholders_and_nothing_else_is() {
        // IO_REPARSE_TAG_CLOUD and CLOUD_1 .. CLOUD_F.
        for n in 0..=0xFu32 {
            let tag = 0x9000_001A | (n << 12);
            assert!(is_cloud_reparse_tag(tag), "{tag:#x}");
            assert_eq!(classify_reparse_tag(tag), EntryKind::CloudPlaceholder);
        }
        assert_eq!(classify_reparse_tag(0xA000_0003), EntryKind::Symlink); // mount point
        assert_eq!(classify_reparse_tag(0xA000_000C), EntryKind::Symlink); // symlink
        for other in [
            0x8000_0013u32, // dedup
            0x8000_0017,    // WOF (compressed system files)
            0x8000_001B,    // AppExecLink
            0x8000_0023,    // AF_UNIX socket
            0x9000_101B,    // near miss: not the cloud low word
            0x1000_001A,    // near miss: other high bits
            0,
        ] {
            assert!(!is_cloud_reparse_tag(other), "{other:#x}");
            assert_eq!(classify_reparse_tag(other), EntryKind::Other, "{other:#x}");
        }
    }

    #[test]
    fn components_must_be_single_plain_names() {
        for good in ["a", "a.txt", ".fframes-tx-1.stage", "con-tent"] {
            check_component(good).unwrap();
        }
        let mut bad = vec!["", ".", "..", "a/b", "a\0b"];
        if cfg!(windows) {
            bad.extend(["a\\b", "file:stream", "a.", "a ", "x*", "c:"]);
        }
        for name in bad {
            assert!(check_component(name).is_err(), "{name:?}");
        }
    }
}
