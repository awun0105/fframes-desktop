//! Read-only detection of installed agent CLIs and ACP adapters.
//!
//! Detection only checks that files exist; it never launches anything. Launching still goes
//! through [`crate::resolve_executable`] with an absolute path the user confirmed.

use crate::discovery::{candidates, is_executable_file};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

/// One agent to look for: its own CLI and the ACP adapter that fronts it. For agents that
/// speak ACP natively both names may be the same executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionSpec {
    pub id: String,
    pub cli_executable: String,
    pub adapter_executable: String,
    /// Extra absolute directories to search for this agent only.
    pub extra_dirs: Vec<PathBuf>,
}

/// The environment detection reads. Injected so tests never touch the real one.
#[derive(Debug, Clone, Default)]
pub struct DetectionEnv {
    /// Directories owned by the app that hold installed adapters (searched first).
    pub managed_dirs: Vec<PathBuf>,
    /// The process `PATH` (and, on Unix, a login-shell `PATH` appended by the caller).
    pub path: Option<OsString>,
    pub home: Option<PathBuf>,
    /// `%APPDATA%` on Windows.
    pub app_data: Option<PathBuf>,
    /// `%LOCALAPPDATA%` on Windows.
    pub local_app_data: Option<PathBuf>,
}

impl DetectionEnv {
    /// Reads the real process environment.
    pub fn from_process(managed_dirs: Vec<PathBuf>) -> Self {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        Self {
            managed_dirs,
            path: std::env::var_os("PATH"),
            home: var("HOME").or_else(|| var("USERPROFILE")),
            app_data: var("APPDATA"),
            local_app_data: var("LOCALAPPDATA"),
        }
    }

    /// Search roots in priority order, absolute and de-duplicated.
    fn roots(&self, extra: &[PathBuf]) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = self.managed_dirs.clone();
        if let Some(path) = &self.path {
            roots.extend(std::env::split_paths(path));
        }
        roots.extend(extra.iter().cloned());
        if let Some(home) = &self.home {
            roots.push(home.join(".local").join("bin"));
            roots.push(home.join(".npm-global").join("bin"));
            roots.push(home.join(".bun").join("bin"));
            roots.push(home.join(".volta").join("bin"));
            roots.push(home.join(".local").join("share").join("pnpm"));
            roots.extend(version_dirs(
                &home.join(".nvm").join("versions").join("node"),
                &["bin"],
            ));
            for fnm in [
                home.join(".fnm").join("node-versions"),
                home.join(".local")
                    .join("share")
                    .join("fnm")
                    .join("node-versions"),
            ] {
                roots.extend(version_dirs(&fnm, &["installation", "bin"]));
            }
        }
        if let Some(app_data) = &self.app_data {
            roots.push(app_data.join("npm"));
        }
        if let Some(local) = &self.local_app_data {
            roots.push(local.join("pnpm"));
            roots.push(local.join("Volta").join("bin"));
        }
        roots.push(PathBuf::from("/opt/homebrew/bin"));
        roots.push(PathBuf::from("/usr/local/bin"));
        let mut seen = std::collections::HashSet::new();
        roots
            .into_iter()
            // Relative entries would resolve against an arbitrary directory, and an empty
            // Windows entry names nothing.
            .filter(|dir| dir.is_absolute())
            .filter(|dir| seen.insert(root_key(dir)))
            .collect()
    }
}

/// The identity of a search root. Windows paths compare case-insensitively and accept either
/// separator, so `C:\Node` and `c:/node` are one root.
fn root_key(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(dir.to_string_lossy().replace('/', "\\").to_lowercase())
    } else {
        dir.to_owned()
    }
}

/// `<base>/<version>/<tail...>` for every version directory that exists, newest name first.
fn version_dirs(base: &Path, tail: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(base) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| tail.iter().fold(entry.path(), |dir, part| dir.join(part)))
        .filter(|dir| dir.is_dir())
        .collect();
    dirs.sort_by(|a, b| b.cmp(a));
    dirs
}

/// What detection found for one agent. Paths are absolute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub id: String,
    pub cli: Option<PathBuf>,
    pub adapter: Option<PathBuf>,
}

/// The first match, as the path the user's search root names it. Found paths are saved, so
/// symlinks are not resolved: a version manager that repoints its link keeps working.
fn find(roots: &[PathBuf], name: &str) -> Option<PathBuf> {
    roots
        .iter()
        .flat_map(|dir| candidates(dir, name))
        .find(|candidate| is_executable_file(candidate))
}

/// Finds one executable (for example `node` or `npm`) the same way detection does.
pub fn find_executable(name: &str, env: &DetectionEnv) -> Option<PathBuf> {
    find(&env.roots(&[]), name)
}

/// Looks for every spec's CLI and adapter. Existence checks only; no process is spawned.
pub fn detect_agents(specs: &[DetectionSpec], env: &DetectionEnv) -> Vec<Detected> {
    specs
        .iter()
        .map(|spec| {
            let roots = env.roots(&spec.extra_dirs);
            Detected {
                id: spec.id.clone(),
                cli: find(&roots, &spec.cli_executable),
                adapter: find(&roots, &spec.adapter_executable),
            }
        })
        .collect()
}

/// How an adapter file starts, given that the launch `PATH` never contains `node`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeEntry {
    /// Not JavaScript: start the file itself.
    Native,
    /// A `.js` file, or the entry an npm shim runs: start `node <entry>`.
    Script(PathBuf),
    /// A `.cmd`/`.bat` launcher whose script entry could not be read from it. Start the
    /// launcher itself with Node's folder first on its `PATH`, so a `node` call inside it
    /// finds the runtime.
    Shim,
}

fn is_script(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ["js", "mjs", "cjs"].contains(&ext.to_ascii_lowercase().as_str()))
}

/// Classifies an adapter: a JavaScript file, an npm `.cmd` shim (resolved to its package
/// entry), another batch launcher, or a native binary. Fails only when a batch launcher
/// cannot be read, so an unreadable shim is never mistaken for a native binary.
pub fn node_entry(adapter: &Path) -> Result<NodeEntry, String> {
    if is_script(adapter) {
        return Ok(NodeEntry::Script(adapter.to_owned()));
    }
    let batch = adapter
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));
    if !batch {
        return Ok(NodeEntry::Native);
    }
    let unreadable = |error: std::io::Error| {
        format!(
            "could not read the adapter launcher '{}': {error}",
            adapter.display()
        )
    };
    let mut head = Vec::new();
    std::fs::File::open(adapter)
        .map_err(unreadable)?
        .take(16 * 1024)
        .read_to_end(&mut head)
        .map_err(unreadable)?;
    let text = String::from_utf8_lossy(&head);
    let Some(parent) = adapter.parent() else {
        return Ok(NodeEntry::Shim);
    };
    // npm's shim runs `"%_prog%" "%dp0%\<relative entry>" %*` (older versions write
    // `%~dp0\`); earlier references name the optional `node.exe` beside the shim, which the
    // script filter skips.
    let entry = ["%dp0%\\", "%~dp0\\"]
        .iter()
        .flat_map(|marker| text.split(marker).skip(1))
        .filter_map(|rest| rest.split('"').next())
        .filter(|relative| !relative.split(['\\', '/']).any(|part| part == ".."))
        .map(|relative| parent.join(relative.replace('\\', "/")))
        .find(|entry| is_script(entry) && entry.is_file());
    Ok(entry.map_or(NodeEntry::Shim, NodeEntry::Script))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe_name(name: &str) -> String {
        if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        }
    }

    fn touch(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(exe_name(name));
        std::fs::write(&path, b"x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn spec() -> DetectionSpec {
        DetectionSpec {
            id: "claude".into(),
            cli_executable: "claude".into(),
            adapter_executable: "claude-agent-acp".into(),
            extra_dirs: Vec::new(),
        }
    }

    fn env_with_path(dirs: &[&Path]) -> DetectionEnv {
        DetectionEnv {
            path: Some(std::env::join_paths(dirs).unwrap()),
            ..DetectionEnv::default()
        }
    }

    #[test]
    fn cli_without_adapter_is_reported_as_such() {
        let dir = tempfile::tempdir().unwrap();
        let cli = touch(dir.path(), "claude");
        let found = detect_agents(&[spec()], &env_with_path(&[dir.path()]));
        assert_eq!(found[0].cli, Some(cli));
        assert_eq!(found[0].adapter, None);
    }

    #[test]
    fn managed_dir_wins_over_path() {
        let managed = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let managed_exe = touch(managed.path(), "claude-agent-acp");
        touch(other.path(), "claude-agent-acp");
        let mut env = env_with_path(&[other.path()]);
        env.managed_dirs = vec![managed.path().to_owned()];
        assert_eq!(detect_agents(&[spec()], &env)[0].adapter, Some(managed_exe));
    }

    #[test]
    fn duplicate_roots_and_relative_roots_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let mut path = std::env::join_paths([dir.path(), dir.path()]).unwrap();
        path.push(if cfg!(windows) {
            ";relative\\bin;;"
        } else {
            ":relative/bin:"
        });
        let env = DetectionEnv {
            path: Some(path),
            ..DetectionEnv::default()
        };
        let roots = env.roots(&[]);
        assert_eq!(roots.iter().filter(|r| *r == dir.path()).count(), 1);
        assert!(roots.iter().all(|r| r.is_absolute()));
    }

    #[test]
    fn home_and_version_manager_bins_are_searched() {
        let home = tempfile::tempdir().unwrap();
        let local = touch(&home.path().join(".local").join("bin"), "claude");
        let nvm = touch(
            &home.path().join(".nvm/versions/node/v22.1.0/bin"),
            "claude-agent-acp",
        );
        let env = DetectionEnv {
            home: Some(home.path().to_owned()),
            ..DetectionEnv::default()
        };
        let found = &detect_agents(&[spec()], &env)[0];
        assert_eq!(found.cli, Some(local));
        assert_eq!(found.adapter, Some(nvm));
    }

    #[test]
    fn nothing_found_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        let found = &detect_agents(&[spec()], &env_with_path(&[dir.path()]))[0];
        assert_eq!((found.cli.clone(), found.adapter.clone()), (None, None));
    }

    #[test]
    fn npm_cmd_shim_resolves_to_its_package_entry() {
        let dir = tempfile::tempdir().unwrap();
        let entry = dir
            .path()
            .join("modules")
            .join("pkg")
            .join("dist")
            .join("index.js");
        std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
        std::fs::write(&entry, b"//").unwrap();
        // The layout npm writes: the optional `node.exe` beside the shim is named first.
        let shim = dir.path().join("pkg-acp.cmd");
        std::fs::write(
            &shim,
            "@ECHO off\r\nSET dp0=%~dp0\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  \
             SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n)\r\n\
             \"%_prog%\"  \"%dp0%\\modules\\pkg\\dist\\index.js\" %*\r\n",
        )
        .unwrap();
        assert_eq!(node_entry(&shim), Ok(NodeEntry::Script(entry.clone())));
        assert_eq!(node_entry(&entry), Ok(NodeEntry::Script(entry)));
    }

    #[test]
    fn older_npm_shim_template_resolves_to_its_package_entry() {
        let dir = tempfile::tempdir().unwrap();
        let entry = dir.path().join("node_modules/pkg/bin/cli.js");
        std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
        std::fs::write(&entry, b"//").unwrap();
        // The layout older npm versions write, with `%~dp0\` and `node.exe` named first.
        let shim = dir.path().join("pkg-acp.cmd");
        std::fs::write(
            &shim,
            "@IF EXIST \"%~dp0\\node.exe\" (\r\n  \
             \"%~dp0\\node.exe\"  \"%~dp0\\node_modules\\pkg\\bin\\cli.js\" %*\r\n\
             ) ELSE (\r\n  @SETLOCAL\r\n  @SET PATHEXT=%PATHEXT:;.JS;=;%\r\n  \
             node  \"%~dp0\\node_modules\\pkg\\bin\\cli.js\" %*\r\n)\r\n",
        )
        .unwrap();
        assert_eq!(node_entry(&shim), Ok(NodeEntry::Script(entry)));
    }

    #[test]
    fn native_binaries_run_directly_and_unresolved_shims_run_as_shims() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("agent.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        assert_eq!(node_entry(&exe), Ok(NodeEntry::Native));
        // An entry outside the shim's folder is never returned.
        std::fs::write(dir.path().join("evil.js"), b"//").unwrap();
        let inner = dir.path().join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        let escaping = inner.join("bad.cmd");
        std::fs::write(&escaping, "\"%_prog%\" \"%dp0%\\..\\evil.js\" %*").unwrap();
        assert_eq!(node_entry(&escaping), Ok(NodeEntry::Shim));
        let wrapper = dir.path().join("wrapper.bat");
        std::fs::write(&wrapper, "@echo off\r\nagent.exe %*\r\n").unwrap();
        assert_eq!(node_entry(&wrapper), Ok(NodeEntry::Shim));
    }

    #[test]
    fn an_unreadable_shim_is_an_error_not_a_native_binary() {
        let dir = tempfile::tempdir().unwrap();
        let shim = dir.path().join("shim.cmd");
        std::fs::create_dir_all(&shim).unwrap();
        let error = node_entry(&shim).unwrap_err();
        assert!(error.contains("shim.cmd"), "{error}");
    }

    #[test]
    fn found_paths_keep_the_search_root_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let exe = touch(&dir.path().join("bin"), "node");
        // A root spelled with a `.` component is returned as spelled, not canonicalized.
        let spelled = dir.path().join(".").join("bin");
        let env = env_with_path(&[&spelled]);
        let found = find_executable("node", &env).unwrap();
        assert_eq!(found, spelled.join(exe.file_name().unwrap()));
        assert!(
            found
                .to_string_lossy()
                .contains(&format!(".{}bin", std::path::MAIN_SEPARATOR))
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_roots_are_deduplicated_ignoring_case_and_separators() {
        let env = DetectionEnv {
            path: Some(OsString::from(r"C:\Node\bin;c:/node/BIN\;C:\Other")),
            ..DetectionEnv::default()
        };
        let roots = env.roots(&[]);
        assert_eq!(roots[0], PathBuf::from(r"C:\Node\bin"));
        assert_eq!(roots[1], PathBuf::from(r"C:\Other"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_symlinked_node_keeps_the_link_path() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("v22");
        touch(&real, "node");
        let link = dir.path().join("nodejs");
        // Creating a directory symlink needs a privilege a test host may lack.
        if std::os::windows::fs::symlink_dir(&real, &link).is_err() {
            return;
        }
        let found = find_executable("node", &env_with_path(&[&link])).unwrap();
        assert_eq!(found, link.join("node.exe"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_cmd_shim_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("claude-agent-acp.cmd"), b"@echo off").unwrap();
        let found = &detect_agents(&[spec()], &env_with_path(&[dir.path()]))[0];
        assert!(
            found
                .adapter
                .as_ref()
                .unwrap()
                .ends_with("claude-agent-acp.cmd")
        );
    }
}
