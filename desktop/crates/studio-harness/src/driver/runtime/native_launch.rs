//! Launch helpers the native CLI wires share: the user flags a launch may carry and the
//! `--version` gate.

use parking_lot::Mutex;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant, SystemTime},
};

const VERSION_TIMEOUT: Duration = Duration::from_secs(2);

/// A release number, compared field by field.
pub(crate) type Version = (u64, u64, u64);

/// The launch command split into the program prefix (an npm entry script, if any) and the
/// allowed user flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserArgs {
    pub prefix: Vec<String>,
    pub flags: Vec<String>,
}

impl UserArgs {
    /// The value of a user flag, if it was given.
    pub(crate) fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .chunks(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
    }
}

/// Splits the resolved launch arguments: leading non-flag arguments are the program
/// prefix, the rest must be `allowed` `--flag value` / `--flag=value` pairs.
pub(crate) fn split_user_args(
    product: &str,
    allowed: &[&str],
    args: &[String],
) -> Result<UserArgs, String> {
    let start = args
        .iter()
        .position(|arg| arg.starts_with('-'))
        .unwrap_or(args.len());
    let prefix = args[..start].to_vec();
    let mut flags = Vec::new();
    let mut rest = args[start..].iter();
    while let Some(arg) = rest.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (arg.as_str(), None),
        };
        if !allowed.contains(&name) {
            return Err(format!(
                "the {product} protocol does not accept the argument `{name}`; only {} can be set",
                allowed.join(", ")
            ));
        }
        let value = match inline {
            Some(value) => value.to_owned(),
            None => rest
                .next()
                .filter(|value| !value.starts_with('-'))
                .ok_or_else(|| format!("`{name}` needs a value"))?
                .clone(),
        };
        if value.is_empty() {
            return Err(format!("`{name}` needs a value"));
        }
        flags.push(name.to_owned());
        flags.push(value);
    }
    Ok(UserArgs { prefix, flags })
}

/// Parses `2.1.296 (Claude Code)`, `codex-cli 0.162.1` or a bare `v2.1.296`.
pub(crate) fn parse_version(text: &str) -> Option<Version> {
    let word = text
        .split_whitespace()
        .map(|word| word.strip_prefix('v').unwrap_or(word))
        .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let mut parts = word.split('.');
    let mut next = || -> Option<u64> {
        let part = parts.next()?;
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    };
    Some((next()?, next()?, next()?))
}

pub(crate) fn version_label((a, b, c): Version) -> String {
    format!("{a}.{b}.{c}")
}

/// Refuses a release older than `min`, with the reason.
pub(crate) fn check_version(product: &str, min: Version, version: Version) -> Result<(), String> {
    if version < min {
        return Err(format!(
            "{product} {} is older than {}, the oldest release this protocol supports; update {product}",
            version_label(version),
            version_label(min)
        ));
    }
    Ok(())
}

/// The program and each argument of its prefix, with modification time and size for those
/// that are files: an interpreter stays the same when the script it runs is updated.
type VersionKey = Vec<(PathBuf, Option<SystemTime>, u64)>;
type VersionCache = Mutex<HashMap<VersionKey, Version>>;

fn version_cache() -> &'static VersionCache {
    static CACHE: OnceLock<VersionCache> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// `<program> --version`, bounded by [`VERSION_TIMEOUT`] and cached by the program, its
/// prefix arguments and the modification time and size of those that are files, then
/// checked against `min`.
pub(crate) fn version_gate(
    product: &str,
    min: Version,
    executable: &Path,
    prefix: &[String],
    env: &studio_bootstrap::ChildEnvironment,
) -> Result<Version, String> {
    let metadata = std::fs::metadata(executable)
        .map_err(|error| format!("{} cannot be read: {error}", executable.display()))?;
    let mut key = vec![(
        executable.to_owned(),
        metadata.modified().ok(),
        metadata.len(),
    )];
    key.extend(prefix.iter().map(|arg| {
        let metadata = std::fs::metadata(arg)
            .ok()
            .filter(std::fs::Metadata::is_file);
        (
            PathBuf::from(arg),
            metadata.as_ref().and_then(|m| m.modified().ok()),
            metadata.as_ref().map_or(0, std::fs::Metadata::len),
        )
    }));
    if let Some(version) = version_cache().lock().get(&key) {
        check_version(product, min, *version)?;
        return Ok(*version);
    }
    let text = run_version(executable, prefix, env)?;
    let version = parse_version(&text).ok_or_else(|| {
        format!(
            "`{} --version` did not report a version",
            executable.display()
        )
    })?;
    version_cache().lock().insert(key, version);
    check_version(product, min, version)?;
    Ok(version)
}

fn run_version(
    executable: &Path,
    prefix: &[String],
    env: &studio_bootstrap::ChildEnvironment,
) -> Result<String, String> {
    let mut command = Command::new(executable);
    env.apply_to_command(&mut command);
    command
        .args(prefix)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{} could not be started: {error}", executable.display()))?;
    let deadline = Instant::now() + VERSION_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "`{} --version` did not answer within {:?}",
                    executable.display(),
                    VERSION_TIMEOUT
                ));
            }
        }
    }
    let mut text = String::new();
    if let Some(stdout) = child.stdout.take() {
        let _ = stdout.take(4096).read_to_string(&mut text);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_of_both_clis_parse() {
        assert_eq!(parse_version("2.1.296 (Claude Code)"), Some((2, 1, 296)));
        assert_eq!(parse_version("codex-cli 0.162.1"), Some((0, 162, 1)));
        assert_eq!(parse_version("v2.2.0"), Some((2, 2, 0)));
        assert_eq!(parse_version("Claude Code"), None);
        assert!(check_version("X", (0, 162, 1), (0, 162, 1)).is_ok());
        assert!(
            check_version("X", (0, 162, 1), (0, 161, 9))
                .unwrap_err()
                .contains("0.162.1")
        );
    }

    #[test]
    fn user_flags_are_read_back_by_name() {
        let args: Vec<String> = ["entry.js", "--model", "m", "--effort=low"]
            .map(str::to_owned)
            .to_vec();
        let split = split_user_args("X", &["--model", "--effort"], &args).unwrap();
        assert_eq!(split.flag("--model"), Some("m"));
        assert_eq!(split.flag("--effort"), Some("low"));
        assert_eq!(split.flag("--other"), None);
    }
}
