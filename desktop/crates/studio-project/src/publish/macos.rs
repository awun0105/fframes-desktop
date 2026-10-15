//! macOS: the atomic no-replace rename is `renameatx_np(RENAME_EXCL)`.
use std::{ffi::CStr, io, os::fd::RawFd};

/// Moves `from` in `from_dir` to `to` in `to_dir`; an existing `to` is `AlreadyExists`
/// and leaves both names untouched.
pub(super) fn rename_exclusive(
    from_dir: RawFd,
    from: &CStr,
    to_dir: RawFd,
    to: &CStr,
) -> io::Result<()> {
    // SAFETY: valid descriptors and NUL-terminated components.
    let result = unsafe {
        libc::renameatx_np(
            from_dir,
            from.as_ptr(),
            to_dir,
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
