use parking_lot::Mutex;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use studio_bootstrap::{
    ChildEnvironment, ProcessError, ProcessTreeManager, SpawnOptions, TrackedChild,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("process error: {0}")]
    Process(#[from] ProcessError),
}

/// Starts adapter children inside the shared process tree, so they are owned and stopped
/// with it.
pub struct AgentSupervisor {
    process_tree: ProcessTreeManager,
}

impl AgentSupervisor {
    pub fn new(process_tree: ProcessTreeManager) -> Self {
        Self { process_tree }
    }

    /// Spawns an agent adapter child in an isolated environment with process group ownership.
    pub fn spawn_adapter(
        &self,
        executable: &Path,
        args: &[String],
        working_dir: &Path,
        custom_env: Option<ChildEnvironment>,
    ) -> Result<Arc<Mutex<TrackedChild>>, SupervisorError> {
        let env = custom_env.unwrap_or_else(ChildEnvironment::default_allowlist);
        let mut opts = SpawnOptions::new(executable);
        opts.args(args);
        opts.current_dir(working_dir);
        opts.env = env;
        opts.stdin = Stdio::piped();
        opts.stdout = Stdio::piped();
        opts.stderr = Stdio::piped();

        Ok(self.process_tree.spawn(opts)?)
    }
}
