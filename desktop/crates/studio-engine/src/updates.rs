//! Whether a newer fframes Studio has been released. Asks GitHub for the latest release
//! with the system's `curl` (shipped with Windows 10 and later, macOS and most Linux
//! distributions), so the app needs no HTTP client. Only reports; never downloads.
use std::process::Command;

use serde::Deserialize;

/// The repository whose releases are the app's (the one `install.ps1` installs from).
pub const RELEASES_REPOSITORY: &str = "khanhthanhdev/fframes-desktop";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    /// The released version, without the tag's `v`.
    pub version: String,
    /// The release page, where the installer and notes are.
    pub url: String,
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
}

/// The latest release when it is newer than `current`. Blocks for up to 15 seconds; run it
/// off the UI thread.
pub fn check_for_update(current: &str) -> Result<Option<AvailableUpdate>, String> {
    let mut command = Command::new("curl");
    command.args([
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--max-time",
        "15",
        "--header",
        "Accept: application/vnd.github+json",
        &format!("https://api.github.com/repos/{RELEASES_REPOSITORY}/releases/latest"),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .map_err(|e| format!("Cannot check for updates: curl is unavailable ({e})"))?;
    if !output.status.success() {
        return Err(format!(
            "Cannot check for updates: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    newer_release(current, &output.stdout)
}

/// The release described by GitHub's `releases/latest` answer `json`, when it is newer
/// than `current`.
fn newer_release(current: &str, json: &[u8]) -> Result<Option<AvailableUpdate>, String> {
    let release: LatestRelease = serde_json::from_slice(json)
        .map_err(|e| format!("Cannot check for updates: unexpected answer ({e})"))?;
    let version = release.tag_name.trim_start_matches('v');
    let parse = |v: &str| {
        semver::Version::parse(v).map_err(|e| format!("Cannot check for updates: version {v}: {e}"))
    };
    Ok(
        (parse(version)? > parse(current)?).then(|| AvailableUpdate {
            version: version.to_owned(),
            url: release.html_url,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(tag: &str) -> Vec<u8> {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://example.invalid/{tag}","draft":false}}"#
        )
        .into_bytes()
    }

    #[test]
    fn only_a_newer_release_is_an_update() {
        let update = newer_release("0.1.4", &answer("v0.1.10")).unwrap().unwrap();
        assert_eq!(
            update.version, "0.1.10",
            "compared as versions, not as text"
        );
        assert_eq!(update.url, "https://example.invalid/v0.1.10");
        assert_eq!(newer_release("0.1.4", &answer("v0.1.4")).unwrap(), None);
        assert_eq!(newer_release("0.2.0", &answer("v0.1.9")).unwrap(), None);
        assert_eq!(
            newer_release("0.2.0-rc.1", &answer("0.2.0"))
                .unwrap()
                .unwrap()
                .version,
            "0.2.0"
        );
        assert!(newer_release("0.1.4", br#"{"message":"Not Found"}"#).is_err());
        assert!(newer_release("0.1.4", &answer("nightly")).is_err());
    }
}
