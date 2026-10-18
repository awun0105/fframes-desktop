//! Classification of links found in agent output, and the policy for opening them.
//!
//! Agent text is untrusted. A link is only ever acted on in two ways: an http(s) URL is
//! opened in the browser after the user has seen the whole URL, and a file inside the
//! project is revealed in the file manager. A file is never opened with its default
//! verb, so a link to `tools/x.cmd` cannot run it.

use std::path::{Path, PathBuf};

use gpui::App;
use studio_project::ProjectPath;

/// What a link destination points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An http or https URL with a lowercase scheme, shown in full before opening.
    External {
        url: String,
    },
    /// A project-relative path that passes the project path rules. `line` is display-only.
    ProjectFile {
        path: ProjectPath,
        line: Option<u32>,
    },
    NotClickable(NotClickable),
}

/// Why a link is rendered as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotClickable {
    Empty,
    /// Lowercase scheme: `file`, `mailto`, `javascript`, `vscode`, ...
    Scheme(String),
    /// Malformed or credential-carrying http(s) URL.
    BadUrl,
    /// UNC (`\\server\share`, `//server`).
    Network,
    /// Absolute path or drive letter.
    Absolute,
    /// Not a valid contained relative path (`..`, backslash, reserved names, ...).
    OutsideProject,
    /// A bare `#anchor` or an unsupported fragment.
    Fragment,
    /// The file does not exist, or a path component is a symlink or reparse point.
    Unavailable,
}

impl NotClickable {
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "Empty link".into(),
            Self::Scheme(s) => format!("Links using {s}: are not opened"),
            Self::BadUrl => "Malformed or credential-carrying URL".into(),
            Self::Network => "Network paths are not opened".into(),
            Self::Absolute => "Absolute paths are not opened".into(),
            Self::OutsideProject => "Path is not inside the project".into(),
            Self::Fragment => "In-page anchors are not supported".into(),
            Self::Unavailable => "File is missing or is a link".into(),
        }
    }
}

/// Classify a markdown link destination. Pure: no filesystem access, so a project path
/// is only lexically valid here; `action_for` checks it on disk.
pub fn classify(destination: &str) -> LinkTarget {
    let dest = destination.trim();
    if dest.is_empty() {
        return LinkTarget::NotClickable(NotClickable::Empty);
    }
    if dest.chars().any(char::is_control) {
        return LinkTarget::NotClickable(NotClickable::OutsideProject);
    }
    if dest.starts_with("\\\\") || dest.starts_with("//") {
        return LinkTarget::NotClickable(NotClickable::Network);
    }
    if dest.starts_with('/') || dest.starts_with('\\') {
        return LinkTarget::NotClickable(NotClickable::Absolute);
    }
    if dest.starts_with('#') {
        return LinkTarget::NotClickable(NotClickable::Fragment);
    }
    // A web URL keeps every `:` (ports, `:12` in a path); only other text can end in a line.
    if let Some(scheme @ ("http" | "https")) = scheme_of(dest).as_deref() {
        return match normalize_url(dest, scheme) {
            Some(url) => LinkTarget::External { url },
            None => LinkTarget::NotClickable(NotClickable::BadUrl),
        };
    }
    // A `name:123` suffix is a line number, not a scheme.
    let (body, colon_line) = split_colon_line(dest);
    if let Some(scheme) = scheme_of(body) {
        return match scheme.as_str() {
            "http" | "https" => match normalize_url(body, &scheme) {
                Some(url) => LinkTarget::External { url },
                None => LinkTarget::NotClickable(NotClickable::BadUrl),
            },
            s if s.len() == 1 => LinkTarget::NotClickable(NotClickable::Absolute),
            _ => LinkTarget::NotClickable(NotClickable::Scheme(scheme)),
        };
    }
    classify_path(body, colon_line)
}

fn classify_path(body: &str, colon_line: Option<u32>) -> LinkTarget {
    let (raw, fragment) = match body.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (body, None),
    };
    let line = match fragment {
        None => colon_line,
        Some(f) => match parse_fragment_line(f) {
            Some(n) => Some(n),
            None => return LinkTarget::NotClickable(NotClickable::Fragment),
        },
    };
    let Some(decoded) = percent_decode(raw) else {
        return LinkTarget::NotClickable(NotClickable::OutsideProject);
    };
    // Decoding can reveal what the raw text hid.
    if decoded.starts_with(['/', '\\']) {
        let network = decoded.starts_with("//") || decoded.starts_with("\\\\");
        return LinkTarget::NotClickable(if network {
            NotClickable::Network
        } else {
            NotClickable::Absolute
        });
    }
    let relative = decoded.strip_prefix("./").unwrap_or(&decoded);
    match ProjectPath::try_from(relative.to_owned()) {
        Ok(path) => LinkTarget::ProjectFile { path, line },
        Err(_) => LinkTarget::NotClickable(NotClickable::OutsideProject),
    }
}

/// Lowercase scheme when `s` begins with `scheme:` (RFC 3986 scheme characters).
fn scheme_of(s: &str) -> Option<String> {
    let (head, _) = s.split_once(':')?;
    let mut chars = head.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then(|| head.to_ascii_lowercase())
}

/// Strip up to two trailing `:number` groups (`path:line:column`) and return the line.
/// A single character before the colon is a drive letter and is never split.
fn split_colon_line(s: &str) -> (&str, Option<u32>) {
    let mut body = s;
    let mut line = None;
    for _ in 0..2 {
        let Some((head, tail)) = body.rsplit_once(':') else {
            break;
        };
        let Ok(n) = tail.parse::<u32>() else { break };
        if head.chars().count() < 2 || head.ends_with(':') {
            break;
        }
        body = head;
        line = Some(n);
    }
    (body, line)
}

/// `path` with any trailing `:line[:column]` or `#fragment` removed.
pub fn without_location(path: &str) -> &str {
    let before_fragment = path.split_once('#').map_or(path, |(head, _)| head);
    split_colon_line(before_fragment).0
}

fn parse_fragment_line(f: &str) -> Option<u32> {
    let rest = f.strip_prefix('L')?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let (digits, tail) = rest.split_at(end);
    let tail_ok = tail.is_empty()
        || tail
            .strip_prefix('C')
            .or_else(|| tail.strip_prefix('-'))
            .is_some_and(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()));
    if tail_ok { digits.parse().ok() } else { None }
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Accept `scheme://host[:port][/path][?query][#fragment]` with an ASCII host and no
/// userinfo (`https://github.com@evil.example/` shows the wrong host). Returns the URL
/// with a lowercase scheme.
fn normalize_url(s: &str, scheme: &str) -> Option<String> {
    let rest = s.get(scheme.len()..)?.strip_prefix("://")?;
    // Characters RFC 3986 never allows unencoded; the Windows shell may read some of them.
    if s.chars().any(|c| {
        c.is_whitespace()
            || c.is_control()
            || !c.is_ascii()
            || matches!(c, '\\' | '"' | '<' | '>' | '^' | '|' | '`' | '{' | '}')
    }) {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = match authority.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => authority,
    };
    if authority.contains('@') || host.is_empty() {
        return None;
    }
    let host_ok = host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '[' | ']' | ':'));
    host_ok.then(|| format!("{scheme}://{rest}"))
}

/// What the renderer does when a link is clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    /// Show this full URL and ask; open it only after the user confirms.
    ConfirmExternal(String),
    /// Reveal this verified in-project path in the file manager.
    Reveal(PathBuf),
    None,
}

/// The effect that actually touches the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEffect {
    Reveal(PathBuf),
    OpenUrl(String),
}

/// Resolve a classified link to an action. This is the only step that reads the
/// filesystem: every component must exist as a real directory entry (no symlinks,
/// junctions or reparse points) and the canonical result must stay inside the project.
pub fn action_for(target: &LinkTarget, project_root: &Path) -> LinkAction {
    match target {
        LinkTarget::External { url } => LinkAction::ConfirmExternal(url.clone()),
        LinkTarget::ProjectFile { path, .. } => match verified_path(path, project_root) {
            Some(abs) => LinkAction::Reveal(abs),
            None => LinkAction::None,
        },
        LinkTarget::NotClickable(_) => LinkAction::None,
    }
}

/// Classify and resolve in one step. A project file that fails the on-disk check is
/// reported as `NotClickable::Unavailable` so the renderer can style it as plain text.
pub fn resolve(destination: &str, project_root: &Path) -> (LinkTarget, LinkAction) {
    let mut target = classify(destination);
    let action = action_for(&target, project_root);
    if matches!(target, LinkTarget::ProjectFile { .. }) && action == LinkAction::None {
        target = LinkTarget::NotClickable(NotClickable::Unavailable);
    }
    (target, action)
}

fn verified_path(path: &ProjectPath, project_root: &Path) -> Option<PathBuf> {
    path.resolve_existing(project_root).ok()?;
    let mut abs = project_root.to_path_buf();
    abs.extend(path.as_str().split('/'));
    Some(abs)
}

/// Turn an action into an effect. `confirmed` is true only after the user has been shown
/// the complete URL of a `ConfirmExternal` action and accepted it.
pub fn effect(action: &LinkAction, confirmed: bool) -> Option<LinkEffect> {
    match action {
        LinkAction::Reveal(p) => Some(LinkEffect::Reveal(p.clone())),
        LinkAction::ConfirmExternal(url) if confirmed => Some(LinkEffect::OpenUrl(url.clone())),
        LinkAction::ConfirmExternal(_) | LinkAction::None => None,
    }
}

/// gpui glue. Reveal selects the file in the file manager; it never launches it.
pub fn perform(effect: &LinkEffect, cx: &mut App) {
    match effect {
        LinkEffect::Reveal(path) => cx.reveal_path(path),
        LinkEffect::OpenUrl(url) => cx.open_url(url),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn not_clickable(dest: &str) -> NotClickable {
        match classify(dest) {
            LinkTarget::NotClickable(r) => r,
            other => panic!("{dest:?} classified as {other:?}"),
        }
    }

    #[test]
    fn http_and_https_are_external_with_normalized_scheme() {
        for (dest, url) in [
            ("https://example.com/a?b=1#c", "https://example.com/a?b=1#c"),
            ("HTTPS://Example.com/x", "https://Example.com/x"),
            ("Http://localhost:8080/", "http://localhost:8080/"),
            ("http://localhost:8080", "http://localhost:8080"),
            ("https://example.com:8443", "https://example.com:8443"),
            ("https://example.com/a:12", "https://example.com/a:12"),
        ] {
            assert_eq!(classify(dest), LinkTarget::External { url: url.into() });
        }
    }

    #[test]
    fn malformed_or_deceptive_urls_are_refused() {
        for dest in [
            "https://github.com@evil.example/",
            "https:/example.com",
            "https:example.com",
            "https://",
            "https:///x",
            "https://exa mple.com",
            "https://ex\u{430}mple.com",
            "https://example.com\\x",
            "https://example.com/\"&calc",
            "https://example.com/a|b",
            "https://example.com/<x>",
            "https://example.com/^x",
        ] {
            assert_eq!(not_clickable(dest), NotClickable::BadUrl, "{dest}");
        }
    }

    #[test]
    fn other_schemes_are_not_clickable_in_any_case() {
        for dest in [
            "file:///C:/Windows/system32/calc.exe",
            "File:///etc/passwd",
            "FILE:x.txt",
            "vscode://file/x",
            "javascript:alert(1)",
            "JavaScript:alert(1)",
            "data:text/html,<b>x</b>",
            "ms-settings:network",
            "mailto:a@b.c",
            "ftp://example.com/x",
            "ssh://host",
        ] {
            assert!(
                matches!(not_clickable(dest), NotClickable::Scheme(_)),
                "{dest}"
            );
        }
        assert_eq!(not_clickable("File:x"), NotClickable::Scheme("file".into()));
    }

    #[test]
    fn unc_absolute_and_drive_paths_are_not_clickable() {
        for dest in ["\\\\server\\share\\x", "//server/share", "///x"] {
            assert_eq!(not_clickable(dest), NotClickable::Network, "{dest}");
        }
        for dest in [
            "/etc/passwd",
            "\\Windows\\x.exe",
            "C:\\Windows\\x.exe",
            "c:/x",
            "C:x",
            "Z:",
        ] {
            assert_eq!(not_clickable(dest), NotClickable::Absolute, "{dest}");
        }
        assert_eq!(not_clickable("%2f%2fserver/x"), NotClickable::Network);
        assert_eq!(not_clickable("%5cWindows"), NotClickable::Absolute);
    }

    #[test]
    fn parent_escapes_and_backslashes_are_not_clickable() {
        for dest in [
            "../x",
            "a/../../x",
            "a/..",
            "%2e%2e/x",
            "%2E%2E%2Fx",
            "a/%2e%2e/%2e%2e/x",
            "a\\b.txt",
            "tools\\x.cmd",
            "a%5cb",
            "a/b%00",
            "a%3Ab",
            "CON",
            "bad%zz",
        ] {
            assert_eq!(not_clickable(dest), NotClickable::OutsideProject, "{dest}");
        }
    }

    #[test]
    fn anchors_alone_are_not_clickable() {
        assert_eq!(not_clickable("#top"), NotClickable::Fragment);
        assert_eq!(not_clickable("a.txt#top"), NotClickable::Fragment);
        assert_eq!(not_clickable(""), NotClickable::Empty);
    }

    #[test]
    fn project_paths_accept_line_suffixes() {
        let file = |dest: &str| match classify(dest) {
            LinkTarget::ProjectFile { path, line } => (path.as_str().to_owned(), line),
            other => panic!("{dest:?} classified as {other:?}"),
        };
        assert_eq!(file("src/lib.rs"), ("src/lib.rs".into(), None));
        assert_eq!(file("./src/lib.rs"), ("src/lib.rs".into(), None));
        assert_eq!(file("src/lib.rs:12"), ("src/lib.rs".into(), Some(12)));
        assert_eq!(file("src/lib.rs:12:5"), ("src/lib.rs".into(), Some(12)));
        assert_eq!(file("src/lib.rs#L12"), ("src/lib.rs".into(), Some(12)));
        assert_eq!(file("src/lib.rs#L12-L20"), ("src/lib.rs".into(), Some(12)));
        assert_eq!(file("src/lib.rs#L12C3"), ("src/lib.rs".into(), Some(12)));
        assert_eq!(file("my%20file.txt"), ("my file.txt".into(), None));
        assert_eq!(file("x.cmd:7"), ("x.cmd".into(), Some(7)));
    }

    #[test]
    fn external_action_needs_confirmation_before_any_effect() {
        let root = Path::new("proj");
        let action = action_for(&classify("https://example.com/"), root);
        assert_eq!(
            action,
            LinkAction::ConfirmExternal("https://example.com/".into())
        );
        assert_eq!(effect(&action, false), None);
        assert_eq!(
            effect(&action, true),
            Some(LinkEffect::OpenUrl("https://example.com/".into()))
        );
    }

    #[test]
    fn not_clickable_has_no_action() {
        let action = action_for(&classify("file:///x"), Path::new("proj"));
        assert_eq!(action, LinkAction::None);
        assert_eq!(effect(&action, true), None);
    }

    #[test]
    fn existing_script_is_revealed_never_opened() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("tools")).unwrap();
        for name in ["x.cmd", "x.exe", "x.bat", "x.ps1", "x.lnk", "x.html"] {
            std::fs::write(root.join("tools").join(name), "echo hi").unwrap();
            let dest = format!("tools/{name}");
            let target = classify(&dest);
            assert!(matches!(target, LinkTarget::ProjectFile { .. }), "{dest}");
            let action = action_for(&target, root);
            let LinkAction::Reveal(abs) = &action else {
                panic!("{dest}: {action:?}");
            };
            assert_eq!(abs, &root.join("tools").join(name));
            for confirmed in [false, true] {
                assert_eq!(
                    effect(&action, confirmed),
                    Some(LinkEffect::Reveal(abs.clone()))
                );
            }
        }
    }

    #[test]
    fn missing_file_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let (target, action) = resolve("nope/x.cmd", dir.path());
        assert_eq!(target, LinkTarget::NotClickable(NotClickable::Unavailable));
        assert_eq!(action, LinkAction::None);
    }

    #[test]
    fn symlink_pointing_outside_the_project_is_not_clickable() {
        let outside = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.cmd");
        std::fs::write(&secret, "x").unwrap();
        let link_file = project.path().join("link.cmd");
        let link_dir = project.path().join("linkdir");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&secret, &link_file)
            .and_then(|()| std::os::unix::fs::symlink(outside.path(), &link_dir));
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&secret, &link_file)
            .and_then(|()| std::os::windows::fs::symlink_dir(outside.path(), &link_dir));
        if let Err(e) = made {
            eprintln!("skipping symlink check: cannot create symlinks here ({e})");
            return;
        }
        for dest in ["link.cmd", "linkdir/secret.cmd"] {
            let (target, action) = resolve(dest, project.path());
            assert_eq!(
                target,
                LinkTarget::NotClickable(NotClickable::Unavailable),
                "{dest}"
            );
            assert_eq!(action, LinkAction::None, "{dest}");
        }
    }
}
