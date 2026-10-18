//! POSIX primitives: `*at` calls relative to a held directory descriptor, `O_NOFOLLOW`
//! on every open and `AT_SYMLINK_NOFOLLOW` on every stat. The atomic no-replace rename
//! itself is per OS (`linux.rs`, `macos.rs`).
use std::{
    ffi::CString,
    fs::File,
    io,
    os::fd::{AsRawFd, FromRawFd},
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{EntryKind, EntryStat, FileId, NoClobber, check_component};

fn component(name: &str) -> io::Result<CString> {
    check_component(name)?;
    CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

fn check(result: libc::c_int) -> io::Result<libc::c_int> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

// `libc::stat` field widths differ per platform (e.g. `st_nlink` is `u16` on macOS), so
// these casts are no-ops on some targets and required on others.
#[allow(clippy::unnecessary_cast)]
fn stat_of(raw: &libc::stat) -> EntryStat {
    let kind = match raw.st_mode & libc::S_IFMT {
        libc::S_IFREG => EntryKind::Regular,
        libc::S_IFDIR => EntryKind::Directory,
        libc::S_IFLNK => EntryKind::Symlink,
        _ => EntryKind::Other,
    };
    EntryStat {
        id: FileId {
            volume: raw.st_dev as u64,
            file: u128::from(raw.st_ino as u64),
        },
        kind,
        size: raw.st_size as u64,
        mode: raw.st_mode as u32 & 0o7777,
        nlink: raw.st_nlink as u64,
        modified_ns: i128::from(raw.st_mtime) * 1_000_000_000 + i128::from(raw.st_mtime_nsec),
        changed_ns: i128::from(raw.st_ctime) * 1_000_000_000 + i128::from(raw.st_ctime_nsec),
        reparse_tag: 0,
    }
}

/// `fstat` of an open descriptor.
pub fn fstat(file: &File) -> io::Result<EntryStat> {
    // SAFETY: zeroed `stat` is a valid out-parameter and the descriptor is open.
    let mut raw: libc::stat = unsafe { std::mem::zeroed() };
    check(unsafe { libc::fstat(file.as_raw_fd(), &mut raw) })?;
    Ok(stat_of(&raw))
}

/// The identity chain of a root pathname: the `(device, inode)` of every directory from
/// `/` down to the root, found by a no-follow walk. Re-walking it later shows whether
/// the pathname still leads to the very directory that was opened (a root renamed away
/// and replaced by a link, or any ancestor swapped, changes the chain).
#[derive(Debug)]
struct RootFence {
    path: PathBuf,
    chain: Vec<FileId>,
}

/// Walks `path` (absolute, already canonical) from `/` without following any link and
/// returns the final directory plus the id of every directory on the way.
fn walk_no_follow(path: &Path) -> io::Result<(File, Vec<FileId>)> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Component;
    let mut current = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open("/")?;
    let mut chain = vec![fstat(&current)?.id];
    for part in path.components() {
        let name = match part {
            Component::RootDir | Component::CurDir => continue,
            Component::Normal(name) => name,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{} is not a canonical absolute path", path.display()),
                ));
            }
        };
        let name = CString::new(std::os::unix::ffi::OsStrExt::as_bytes(name))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        let fd = check(unsafe {
            libc::openat(
                current.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        })?;
        // SAFETY: openat returned a new descriptor we own.
        current = unsafe { File::from_raw_fd(fd) };
        chain.push(fstat(&current)?.id);
    }
    Ok((current, chain))
}

/// A directory held open by descriptor. All names are single components resolved
/// relative to it, so a path swap above it cannot redirect an operation, and every
/// open refuses symlinks.
#[derive(Debug)]
pub struct Dir {
    file: File,
    id: FileId,
    /// Set on a root handle (and its clones): the pathname binding to re-verify.
    fence: Option<Arc<RootFence>>,
}

impl Dir {
    /// Opens `path` as the root handle. The path is canonicalized once and then walked
    /// from `/` one no-follow component at a time, recording the identity of every
    /// directory on the way (see [`Self::check_root_binding`]).
    pub fn open_root(path: &Path) -> io::Result<Self> {
        let path = std::fs::canonicalize(path)?;
        let (file, chain) = walk_no_follow(&path)?;
        let id = fstat(&file)?.id;
        Ok(Self {
            file,
            id,
            fence: Some(Arc::new(RootFence { path, chain })),
        })
    }

    /// Whether the pathname this root handle was opened by still resolves, without
    /// following any link, to the same directory through the same ancestors. `Err`
    /// describes what changed. A handle that is not a root has nothing to check.
    pub fn check_root_binding(&self) -> Result<(), String> {
        let Some(fence) = &self.fence else {
            return Ok(());
        };
        match walk_no_follow(&fence.path) {
            Ok((_, chain)) if chain == fence.chain => Ok(()),
            Ok(_) => Err(format!(
                "{} no longer leads to the opened directory (it or an ancestor was replaced)",
                fence.path.display()
            )),
            Err(e) => Err(format!(
                "{} can no longer be resolved without following a link ({e}); it was moved, removed or replaced by a link",
                fence.path.display()
            )),
        }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            file: self.file.try_clone()?,
            id: self.id,
            fence: self.fence.clone(),
        })
    }

    pub fn open_child(&self, name: &str) -> io::Result<Self> {
        let name = component(name)?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        let fd = check(unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        })?;
        // SAFETY: openat returned a new descriptor we own.
        let file = unsafe { File::from_raw_fd(fd) };
        if !self.lists_exactly(&name)? {
            return Err(super::alias_refusal(
                name.to_str().unwrap_or_default(),
                None,
            ));
        }
        let id = fstat(&file)?.id;
        Ok(Self {
            file,
            id,
            fence: None,
        })
    }

    /// Whether this directory lists an entry with exactly `name`. A case-insensitive or
    /// normalising directory cannot hold two names that compare equal, so when the exact
    /// name is listed the open reached that entry and no alias of it; the comparison
    /// is by name only, because `d_ino` and `st_ino` differ on overlayfs and mount points.
    fn lists_exactly(&self, name: &std::ffi::CStr) -> io::Result<bool> {
        // A fresh descriptor for the listing: `fdopendir` takes ownership of the one it
        // is given and `closedir` closes it.
        // SAFETY: valid directory descriptor and NUL-terminated literal.
        let fd = check(unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        })?;
        // SAFETY: `fd` is a directory descriptor we own; on success the stream owns it.
        let stream = unsafe { libc::fdopendir(fd) };
        if stream.is_null() {
            let error = io::Error::last_os_error();
            // SAFETY: `fdopendir` failed, so `fd` is still ours.
            unsafe { libc::close(fd) };
            return Err(error);
        }
        let mut found = false;
        loop {
            // SAFETY: `stream` is open; the entry is valid until the next call.
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break;
            }
            // SAFETY: `d_name` is NUL-terminated.
            if unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) } == name {
                found = true;
                break;
            }
        }
        // SAFETY: `stream` is open and closed exactly once.
        unsafe { libc::closedir(stream) };
        Ok(found)
    }

    pub fn id(&self) -> FileId {
        self.id
    }

    /// `fstat` of the directory itself (its permission bits, for example).
    pub fn stat_self(&self) -> io::Result<EntryStat> {
        fstat(&self.file)
    }

    /// `fchmod` of the directory itself (so the umask never changes a restored mode).
    pub fn chmod(&self, mode: u32) -> io::Result<()> {
        // SAFETY: the descriptor is open.
        check(unsafe { libc::fchmod(self.file.as_raw_fd(), mode as libc::mode_t) }).map(drop)
    }

    /// `lstat` of a name; `None` if it does not exist.
    pub fn stat(&self, name: &str) -> io::Result<Option<EntryStat>> {
        let name = component(name)?;
        // SAFETY: zeroed `stat` is a valid out-parameter.
        let mut raw: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: valid directory descriptor and NUL-terminated component.
        let result = unsafe {
            libc::fstatat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                &mut raw,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(error)
            };
        }
        Ok(Some(stat_of(&raw)))
    }

    /// Opens a regular file read-only without following links. The returned stat is the
    /// descriptor's own (`fstat`), so it describes exactly the inode that was opened.
    pub fn open_regular(&self, name: &str) -> io::Result<(File, EntryStat)> {
        let name = component(name)?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        let fd = check(unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        })?;
        // SAFETY: openat returned a new descriptor we own.
        let file = unsafe { File::from_raw_fd(fd) };
        let stat = fstat(&file)?;
        if !stat.is_regular() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name:?} is not a regular file"),
            ));
        }
        Ok((file, stat))
    }

    /// Creates a new regular file (`O_EXCL`, no link following) with `mode` before any
    /// bytes are written. The mode is re-applied with `fchmod` so the umask never
    /// changes it.
    pub fn create_new(&self, name: &str, mode: u32) -> io::Result<File> {
        let cname = component(name)?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        let fd = check(unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                cname.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600 as libc::c_uint,
            )
        })?;
        // SAFETY: openat returned a new descriptor we own.
        let file = unsafe { File::from_raw_fd(fd) };
        // SAFETY: the descriptor is open.
        check(unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) })?;
        Ok(file)
    }

    pub fn mkdir(&self, name: &str, mode: u32) -> io::Result<()> {
        let name = component(name)?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        check(unsafe { libc::mkdirat(self.file.as_raw_fd(), name.as_ptr(), mode as libc::mode_t) })
            .map(drop)
    }

    /// Removes an empty directory.
    pub fn rmdir(&self, name: &str) -> io::Result<()> {
        let name = component(name)?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        check(unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) })
            .map(drop)
    }

    pub fn unlink(&self, name: &str) -> io::Result<()> {
        let name = component(name)?;
        // SAFETY: valid directory descriptor and NUL-terminated component.
        check(unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0) }).map(drop)
    }

    pub fn sync(&self) -> io::Result<()> {
        self.file.sync_all()
    }

    /// [`Self::rename_noreplace`] with a seam for the window between the two syscalls of
    /// the link-based mechanism: `between` runs after `linkat` and before `unlinkat`
    /// (both names exist). The atomic rename has no such window and never calls it.
    pub fn rename_noreplace_observed(
        &self,
        mechanism: NoClobber,
        from: &str,
        target: &Dir,
        to: &str,
        between: impl FnOnce(),
    ) -> io::Result<()> {
        let from_c = component(from)?;
        let to_c = component(to)?;
        match mechanism {
            NoClobber::Renameat2 => super::os::rename_exclusive(
                self.file.as_raw_fd(),
                &from_c,
                target.file.as_raw_fd(),
                &to_c,
            ),
            NoClobber::Link => {
                self.link_noreplace(from, target, to)?;
                between();
                // SAFETY: valid descriptor and NUL-terminated component.
                check(unsafe { libc::unlinkat(self.file.as_raw_fd(), from_c.as_ptr(), 0) })
                    .map(drop)
            }
        }
    }

    /// `linkat` without following a symlink source; an existing `to` is
    /// `ErrorKind::AlreadyExists`. The first half of the link-based move.
    pub fn link_noreplace(&self, from: &str, target: &Dir, to: &str) -> io::Result<()> {
        let from = component(from)?;
        let to = component(to)?;
        // SAFETY: valid descriptors and NUL-terminated components. Without
        // AT_SYMLINK_FOLLOW a symlink source is linked as itself.
        check(unsafe {
            libc::linkat(
                self.file.as_raw_fd(),
                from.as_ptr(),
                target.file.as_raw_fd(),
                to.as_ptr(),
                0,
            )
        })
        .map(drop)
    }
}
