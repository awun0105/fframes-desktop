//! Adapter readiness reports remembered per executable identity.
//!
//! A probe launches the adapter, so automatic readers (the re-check after settings
//! changed mid-probe, setup snapshots) go through this cache and never launch it twice for
//! the same file. The key is the resolved executable's canonical path, modification time
//! and size plus everything else that shapes the launch; replacing the file re-probes.
//! Failures are cached like successes. Sign-in state is not part of the key, so an explicit
//! check always probes and replaces the entry.

use parking_lot::Mutex;
use std::{collections::HashMap, ffi::OsString, path::PathBuf, sync::OnceLock, time::SystemTime};
use studio_harness::{AdapterConfig, DiscoveryReport, ExecutableSearch, resolve_executable};

/// The launch identity a report describes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProbeKey {
    executable: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    args: Vec<String>,
    auth_env_names: Vec<String>,
    auth_method: Option<String>,
    node: Option<PathBuf>,
    managed_dirs: Vec<PathBuf>,
    gui_path: Option<OsString>,
}

impl ProbeKey {
    /// `None` when the executable does not resolve to a readable file: such a probe only
    /// reports the missing executable, which is cheap and never cached.
    pub fn of(
        adapter: &AdapterConfig,
        search: &ExecutableSearch,
        auth_method: Option<&str>,
    ) -> Option<Self> {
        let resolved = resolve_executable(&adapter.executable, search).ok()?;
        let executable = resolved.canonicalize().ok()?;
        let metadata = executable.metadata().ok()?;
        Some(Self {
            executable,
            modified: metadata.modified().ok(),
            len: metadata.len(),
            args: adapter.args.clone(),
            auth_env_names: adapter.auth_env_names.clone(),
            auth_method: auth_method.map(str::to_owned),
            node: search.node.clone(),
            managed_dirs: search.managed_dirs.clone(),
            gui_path: search.gui_path.clone(),
        })
    }
}

#[derive(Default)]
pub struct AdapterProbeCache {
    entries: Mutex<HashMap<ProbeKey, DiscoveryReport>>,
}

impl AdapterProbeCache {
    /// The process-wide cache every workflow shares.
    pub fn global() -> &'static Self {
        static CACHE: OnceLock<AdapterProbeCache> = OnceLock::new();
        CACHE.get_or_init(Self::default)
    }

    /// The remembered report for `key`; never probes.
    pub fn peek(&self, key: &ProbeKey) -> Option<DiscoveryReport> {
        self.entries.lock().get(key).cloned()
    }

    /// The remembered report, or a fresh probe that is then remembered. The lock is not
    /// held while probing; two concurrent first probes of one file both run.
    pub fn get_or_probe(
        &self,
        key: Option<ProbeKey>,
        probe: impl FnOnce() -> DiscoveryReport,
    ) -> DiscoveryReport {
        if let Some(report) = key.as_ref().and_then(|key| self.peek(key)) {
            return report;
        }
        self.refresh(key, probe)
    }

    /// Always probes, and replaces whatever was remembered for this executable.
    pub fn refresh(
        &self,
        key: Option<ProbeKey>,
        probe: impl FnOnce() -> DiscoveryReport,
    ) -> DiscoveryReport {
        let report = probe();
        if let Some(key) = key {
            let mut entries = self.entries.lock();
            // One entry per executable: an older identity of the same file is stale.
            entries.retain(|old, _| old.executable != key.executable);
            entries.insert(key, report.clone());
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, fs, path::Path};
    use studio_harness::AdapterStatus;

    fn adapter(path: &Path) -> AdapterConfig {
        AdapterConfig {
            executable: path.to_string_lossy().into_owned(),
            args: vec!["--acp".into()],
            auth_env_names: vec![],
        }
    }

    fn write_adapter(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    fn report(detail: &str) -> DiscoveryReport {
        DiscoveryReport {
            status: AdapterStatus::MissingRuntime {
                detail: detail.into(),
            },
            executable: None,
            initialized: None,
        }
    }

    fn key(path: &Path) -> Option<ProbeKey> {
        ProbeKey::of(&adapter(path), &ExecutableSearch::default(), None)
    }

    #[test]
    fn an_unchanged_adapter_is_probed_once_and_its_failure_is_remembered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adapter.cmd");
        write_adapter(&path, "echo one");
        let cache = AdapterProbeCache::default();
        let probes = Cell::new(0);
        let probe = || {
            probes.set(probes.get() + 1);
            report("node missing")
        };
        let first = cache.get_or_probe(key(&path), probe);
        let second = cache.get_or_probe(key(&path), probe);
        assert_eq!(probes.get(), 1);
        assert_eq!(first, second);
        assert_eq!(cache.peek(&key(&path).unwrap()), Some(first));
    }

    #[test]
    fn replacing_the_adapter_file_probes_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adapter.cmd");
        write_adapter(&path, "echo one");
        let cache = AdapterProbeCache::default();
        let probes = Cell::new(0);
        let probe = || {
            probes.set(probes.get() + 1);
            report(&format!("probe {}", probes.get()))
        };
        cache.get_or_probe(key(&path), probe);
        write_adapter(&path, "echo a longer replacement");
        let after = cache.get_or_probe(key(&path), probe);
        assert_eq!(probes.get(), 2);
        assert_eq!(after, report("probe 2"));
        // The old identity of the same file is gone.
        assert_eq!(cache.entries.lock().len(), 1);
    }

    #[test]
    fn launch_settings_are_part_of_the_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adapter.cmd");
        write_adapter(&path, "echo one");
        let search = ExecutableSearch::default();
        let plain = ProbeKey::of(&adapter(&path), &search, None);
        let with_method = ProbeKey::of(&adapter(&path), &search, Some("api-key"));
        let mut other_args = adapter(&path);
        other_args.args.push("--verbose".into());
        let other_args = ProbeKey::of(&other_args, &search, None);
        assert_ne!(plain, with_method);
        assert_ne!(plain, other_args);
    }

    #[test]
    fn an_explicit_check_always_probes_and_replaces_the_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adapter.cmd");
        write_adapter(&path, "echo one");
        let cache = AdapterProbeCache::default();
        cache.get_or_probe(key(&path), || report("signed out"));
        let refreshed = cache.refresh(key(&path), || report("signed in"));
        assert_eq!(refreshed, report("signed in"));
        assert_eq!(
            cache.get_or_probe(key(&path), || unreachable!("cached")),
            report("signed in")
        );
    }

    #[test]
    fn a_missing_executable_is_not_cached() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("absent.cmd");
        assert_eq!(key(&path), None);
        let cache = AdapterProbeCache::default();
        let probes = Cell::new(0);
        let probe = || {
            probes.set(probes.get() + 1);
            report("missing")
        };
        cache.get_or_probe(key(&path), probe);
        cache.get_or_probe(key(&path), probe);
        assert_eq!(probes.get(), 2);
        assert!(cache.entries.lock().is_empty());
    }
}
