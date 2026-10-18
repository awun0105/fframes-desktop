//! Writer containment: who may call an adapter's writer processes contained.
//!
//! A settings value never qualifies anything. In production the driver reports the
//! containment the process supervisor verified for the adapter it just spawned (a job
//! object it checked the adapter is in on Windows, a process group with an escape scan on
//! Linux and macOS); observations during the task can only demote it. A test harness may
//! inject an ownership instead, and that is labelled as injected wherever it is shown.
//!
//! The launch identity below binds a resumable provider session to the exact program it
//! was started with.

use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};
use studio_bootstrap::WriterOwnership;
use studio_harness::{AdapterLaunch, OwnershipSource};

/// What an adapter launch runs, as bytes. Paths are hashed, never recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchIdentity {
    pub executable_sha256: String,
    pub args: Vec<String>,
    pub auth_env_names: Vec<String>,
    /// `(argument index, sha256)` of every argument that names an existing absolute file
    /// (an interpreter's script, a bundle): changing it changes the identity.
    pub arg_files: Vec<(usize, String)>,
}

impl LaunchIdentity {
    /// The binding digest: SHA-256 over the NUL-joined parts `fframes-launch/1`,
    /// executable digest, argument count + arguments, variable name count + sorted names,
    /// file-argument count + `index:digest` entries.
    pub fn digest(&self) -> String {
        let mut names = self.auth_env_names.clone();
        names.sort();
        let mut parts: Vec<String> = vec![
            "fframes-launch/1".into(),
            self.executable_sha256.clone(),
            self.args.len().to_string(),
        ];
        parts.extend(self.args.iter().cloned());
        parts.push(names.len().to_string());
        parts.extend(names);
        parts.push(self.arg_files.len().to_string());
        parts.extend(self.arg_files.iter().map(|(i, d)| format!("{i}:{d}")));
        let mut hasher = Sha256::new();
        for (index, part) in parts.iter().enumerate() {
            if index > 0 {
                hasher.update([0u8]);
            }
            hasher.update(part.as_bytes());
        }
        hex(&hasher.finalize())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

/// The identity of the program a launch actually runs: the resolved executable (for an
/// npm shim, the interpreter with its script among the arguments) and its arguments.
/// `Err` when the executable cannot be read. Hashes files: call it off the UI thread.
pub fn launch_identity(launch: &AdapterLaunch) -> Result<LaunchIdentity, String> {
    let executable_sha256 = hash_file(&launch.executable)
        .map_err(|e| format!("the adapter executable cannot be read: {e}"))?;
    let mut arg_files = Vec::new();
    for (index, arg) in launch.args.iter().enumerate() {
        let candidate = Path::new(arg);
        if candidate.is_absolute()
            && std::fs::metadata(candidate).is_ok_and(|m| m.is_file())
            && let Ok(digest) = hash_file(candidate)
        {
            arg_files.push((index, digest));
        }
    }
    Ok(LaunchIdentity {
        executable_sha256,
        args: launch.args.clone(),
        auth_env_names: launch.auth_names.clone(),
        arg_files,
    })
}

/// Who may say a writer is contained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipPolicy {
    /// Production: the containment the supervisor verified at each launch.
    Validated,
    /// Test-only injection for fixtures (the studio's observation entry): the given
    /// ownership is used as is and labelled as injected everywhere it is shown.
    TestInjected(WriterOwnership),
}

impl OwnershipPolicy {
    /// What the driver uses as the writer's ownership at launch.
    pub fn source(&self) -> OwnershipSource {
        match self {
            Self::Validated => OwnershipSource::Verified,
            Self::TestInjected(ownership) => OwnershipSource::Injected(ownership.clone()),
        }
    }

    /// One line for the Setup tab, before or between launches.
    pub fn summary(&self) -> String {
        match self {
            Self::Validated => "Writer containment is not a setting: it is verified every time the adapter starts (a job object on Windows, a process group with an escape scan on Linux and macOS). Candidates are captured only from a launch whose containment held.".into(),
            Self::TestInjected(ownership) => format!(
                "Writer containment: injected by the test harness ({}); this is not a verification.",
                ownership.label()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "819ef19d8b35aa9874b2025b70414b64535eed3d11429bfaf1f342e5fabbdb65";

    fn identity() -> LaunchIdentity {
        LaunchIdentity {
            executable_sha256: "aa".repeat(32),
            args: vec!["--acp".into(), "x y".into()],
            auth_env_names: vec!["B".into(), "A".into()],
            arg_files: vec![(0, "bb".repeat(32))],
        }
    }

    #[test]
    fn the_launch_digest_matches_its_pinned_vector_and_binds_every_part() {
        // A pinned vector, so any change to how the digest is computed is deliberate.
        assert_eq!(identity().digest(), DIGEST);
        let changed = |f: &dyn Fn(&mut LaunchIdentity)| {
            let mut id = identity();
            f(&mut id);
            assert_ne!(id.digest(), DIGEST);
        };
        changed(&|id| id.executable_sha256 = "cc".repeat(32));
        changed(&|id| id.args.push("--more".into()));
        changed(&|id| id.args[1] = "x  y".into());
        changed(&|id| id.auth_env_names.push("C".into()));
        changed(&|id| id.arg_files[0].1 = "dd".repeat(32));
        changed(&|id| id.arg_files.clear());
        // Name order is not part of the identity.
        let mut reordered = identity();
        reordered.auth_env_names.reverse();
        assert_eq!(reordered.digest(), DIGEST);
        // Arguments cannot masquerade as the neighbouring list.
        let mut shifted = identity();
        shifted.args = vec!["--acp".into(), "x y".into(), "B".into()];
        shifted.auth_env_names = vec!["A".into()];
        assert_ne!(shifted.digest(), DIGEST);
    }

    #[test]
    fn the_identity_hashes_the_program_that_is_launched_and_its_file_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("node");
        let script = dir.path().join("adapter.js");
        std::fs::write(&program, b"interpreter").unwrap();
        std::fs::write(&script, b"one").unwrap();
        let launch = AdapterLaunch {
            executable: program.clone(),
            args: vec![script.to_string_lossy().into_owned(), "--acp".into()],
            env: Default::default(),
            secrets: vec![],
            auth_names: vec!["KEY".into()],
            auth_method: None,
        };
        let first = launch_identity(&launch).unwrap();
        assert_eq!(first.arg_files.len(), 1);
        std::fs::write(&script, b"two").unwrap();
        assert_ne!(launch_identity(&launch).unwrap().digest(), first.digest());
        std::fs::remove_file(&program).unwrap();
        assert!(launch_identity(&launch).is_err());
    }
}
