//! App-owned PNG artifacts with ids, hashes, expiry and size limits.
use super::*;

struct StoredArtifact {
    path: PathBuf,
    bytes: u64,
    expires: Instant,
    task: AgentTaskId,
}

/// App-owned artifact directory: ids, expiry and size limits. The only way a caller gets
/// a path is through an [`ArtifactRef`] it was handed.
pub struct ArtifactStore {
    root: PathBuf,
    ttl: Duration,
    max_total: u64,
    entries: Mutex<HashMap<String, StoredArtifact>>,
}

impl ArtifactStore {
    pub fn new(root: &Path, ttl: Duration) -> Result<Self, ToolError> {
        Self::with_limit(root, ttl, MAX_ARTIFACT_STORE_BYTES)
    }

    pub fn with_limit(root: &Path, ttl: Duration, max_total: u64) -> Result<Self, ToolError> {
        let internal = |e: std::io::Error| tool_error(ToolErrorCode::Internal, e.to_string());
        // Artifacts are disposable: whatever a previous app run left is removed.
        if std::fs::symlink_metadata(root).is_ok_and(|m| m.is_symlink()) {
            return Err(tool_error(
                ToolErrorCode::Internal,
                "artifact root must not be a symlink",
            ));
        }
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(root).map_err(internal)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
                .map_err(internal)?;
        }
        Ok(Self {
            root: std::fs::canonicalize(root).map_err(internal)?,
            ttl,
            max_total,
            entries: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn sweep(&self, entries: &mut HashMap<String, StoredArtifact>) {
        let now = Instant::now();
        entries.retain(|_, a| {
            let keep = a.expires > now;
            if !keep {
                let _ = std::fs::remove_file(&a.path);
            }
            keep
        });
    }

    /// Store one PNG. Larger than 8 MiB is refused.
    pub fn put_png(
        &self,
        task: &AgentTaskId,
        bytes: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ArtifactRef, ToolError> {
        if bytes.len() > MAX_IMAGE_ARTIFACT_BYTES {
            return Err(tool_error(
                ToolErrorCode::TooLarge,
                format!(
                    "the rendered image is {} bytes; the limit is {MAX_IMAGE_ARTIFACT_BYTES}. Request a smaller scale or fewer frames",
                    bytes.len()
                ),
            ));
        }
        let mut entries = self.entries.lock();
        self.sweep(&mut entries);
        let mut total: u64 = entries.values().map(|a| a.bytes).sum();
        while total + bytes.len() as u64 > self.max_total {
            let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, a)| a.expires)
                .map(|(id, _)| id.clone())
            else {
                break;
            };
            if let Some(gone) = entries.remove(&oldest) {
                total -= gone.bytes;
                let _ = std::fs::remove_file(gone.path);
            }
        }
        let id = format!("art-{}", uuid::Uuid::new_v4().simple());
        let path = self.root.join(format!("{id}.png"));
        write_private(&path, bytes)
            .map_err(|e| tool_error(ToolErrorCode::Internal, e.to_string()))?;
        let expires = Instant::now() + self.ttl;
        entries.insert(
            id.clone(),
            StoredArtifact {
                path: path.clone(),
                bytes: bytes.len() as u64,
                expires,
                task: task.clone(),
            },
        );
        Ok(ArtifactRef {
            id,
            media_type: "image/png".into(),
            path: path.to_string_lossy().into_owned(),
            bytes: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            width,
            height,
            expires_at_unix: SystemTime::now()
                .checked_add(self.ttl)
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs()),
        })
    }

    /// Resolve an id to its path: well-formed id, known, unexpired, a regular file inside
    /// the store with no symlink component.
    pub fn resolve(&self, id: &str) -> Result<PathBuf, ToolError> {
        let well_formed = id
            .strip_prefix("art-")
            .is_some_and(|hex| hex.len() == 32 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
        if !well_formed {
            return Err(tool_error(
                ToolErrorCode::InvalidParams,
                "malformed artifact id",
            ));
        }
        let mut entries = self.entries.lock();
        self.sweep(&mut entries);
        let entry = entries
            .get(id)
            .ok_or_else(|| tool_error(ToolErrorCode::NotFound, "unknown or expired artifact"))?;
        let meta = std::fs::symlink_metadata(&entry.path)
            .map_err(|_| tool_error(ToolErrorCode::NotFound, "artifact file is gone"))?;
        let canonical = std::fs::canonicalize(&entry.path)
            .map_err(|_| tool_error(ToolErrorCode::NotFound, "artifact file is gone"))?;
        if meta.is_symlink() || !meta.is_file() || !canonical.starts_with(&self.root) {
            return Err(tool_error(
                ToolErrorCode::Unauthorized,
                "artifact path escapes the app store",
            ));
        }
        Ok(canonical)
    }

    /// Reads a bounded PNG only when it belongs to `task`. This is used to attach
    /// revision-bound evidence to an ACP prompt without exposing its app-private path.
    pub fn read_png_for_task(
        &self,
        task: &AgentTaskId,
        id: &str,
        max_bytes: usize,
    ) -> Result<Vec<u8>, ToolError> {
        let well_formed = id
            .strip_prefix("art-")
            .is_some_and(|hex| hex.len() == 32 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
        if !well_formed {
            return Err(ToolError::new(
                ToolErrorCode::InvalidParams,
                "malformed artifact id",
            ));
        }
        let mut entries = self.entries.lock();
        self.sweep(&mut entries);
        let entry = entries.get(id).ok_or_else(|| {
            ToolError::new(ToolErrorCode::NotFound, "unknown or expired artifact")
        })?;
        if &entry.task != task {
            return Err(ToolError::new(
                ToolErrorCode::Unauthorized,
                "artifact belongs to another task",
            ));
        }
        if entry.bytes > max_bytes as u64 {
            return Err(ToolError::new(
                ToolErrorCode::TooLarge,
                "artifact exceeds the prompt image limit",
            ));
        }
        let meta = std::fs::symlink_metadata(&entry.path)
            .map_err(|_| ToolError::new(ToolErrorCode::NotFound, "artifact file is gone"))?;
        let canonical = std::fs::canonicalize(&entry.path)
            .map_err(|_| ToolError::new(ToolErrorCode::NotFound, "artifact file is gone"))?;
        if meta.is_symlink() || !meta.is_file() || !canonical.starts_with(&self.root) {
            return Err(ToolError::new(
                ToolErrorCode::Unauthorized,
                "artifact path escapes the app store",
            ));
        }
        let bytes = std::fs::read(canonical)
            .map_err(|_| ToolError::new(ToolErrorCode::NotFound, "artifact file is gone"))?;
        if bytes.len() as u64 != entry.bytes || bytes.len() > max_bytes {
            return Err(ToolError::new(
                ToolErrorCode::TooLarge,
                "artifact size changed or exceeds the prompt image limit",
            ));
        }
        Ok(bytes)
    }

    pub fn remove_task(&self, task: &AgentTaskId) {
        let mut entries = self.entries.lock();
        entries.retain(|_, a| {
            let keep = &a.task != task;
            if !keep {
                let _ = std::fs::remove_file(&a.path);
            }
            keep
        });
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock();
        for (_, a) in entries.drain() {
            let _ = std::fs::remove_file(a.path);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
