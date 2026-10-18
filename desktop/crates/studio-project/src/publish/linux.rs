//! Linux: the atomic no-replace rename is `renameat2(RENAME_NOREPLACE)`.
use std::{ffi::CStr, io, os::fd::RawFd};

/// Moves `from` in `from_dir` to `to` in `to_dir`; an existing `to` is `AlreadyExists`
/// and leaves both names untouched.
pub(super) fn rename_exclusive(
    from_dir: RawFd,
    from: &CStr,
    to_dir: RawFd,
    to: &CStr,
) -> io::Result<()> {
    // SAFETY: valid descriptors and NUL-terminated components; flag 1 is
    // RENAME_NOREPLACE.
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            from_dir,
            from.as_ptr(),
            to_dir,
            to.as_ptr(),
            1 as libc::c_uint,
        )
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
