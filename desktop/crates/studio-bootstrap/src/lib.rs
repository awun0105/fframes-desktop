pub mod process;

use std::path::{Path, PathBuf};

pub use process::{
    ChildEnvironment, EscapeScan, GroupMembership, ProcessError, ProcessTreeManager,
    ScopeObservation, ScopeTermination, SpawnOptions, TerminationReport, TrackedChild,
    WriterOwnership, spawn_tracked,
};

/// `path` in the form other programs expect. `std::fs::canonicalize` returns Windows paths
/// with the verbatim `\\?\` prefix, which toolchains and Node.js tools (npm's `realpath`
/// recurses without end on one) do not handle; elsewhere the path is returned unchanged.
pub fn external_path(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(normalize_windows_verbatim_prefix(&path.to_string_lossy()).into_owned())
    } else {
        path.to_owned()
    }
}

/// Drops the verbatim prefix of a drive (`\\?\C:\x` is `C:\x`) or share
/// (`\\?\UNC\server\share` is `\\server\share`) path; any other path is unchanged.
pub fn normalize_windows_verbatim_prefix(path: &str) -> std::borrow::Cow<'_, str> {
    if let Some(unc_path) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc_path}").into();
    }
    if let Some(drive_path) = path.strip_prefix(r"\\?\")
        && drive_path.as_bytes().get(1) == Some(&b':')
    {
        return drive_path.into();
    }
    path.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_verbatim_paths_are_normalized_for_external_tools() {
        assert_eq!(
            normalize_windows_verbatim_prefix(r"\\?\C:\sdk\ffmpeg"),
            r"C:\sdk\ffmpeg"
        );
        assert_eq!(
            normalize_windows_verbatim_prefix(r"\\?\UNC\server\share\sdk"),
            r"\\server\share\sdk"
        );
        assert_eq!(
            normalize_windows_verbatim_prefix(r"C:\sdk\ffmpeg"),
            r"C:\sdk\ffmpeg"
        );
    }
}
