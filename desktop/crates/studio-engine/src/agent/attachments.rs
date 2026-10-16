//! Composer attachments: what may be attached to a message, enforced here and never
//! trusted from the UI.
//!
//! * A file inside the project travels as a link to the file in the agent's own working
//!   copy (never to the real project root): the engine checks the file in the project when
//!   it is attached and the link is resolved inside the working copy when the message is
//!   sent. The agent opens that file itself, later, so what it reads is whatever the
//!   working copy holds at that moment.
//! * A file outside the project needs two steps. [`stage`] checks the file, shows the
//!   canonical path and the file identity, and parks a one-shot token in the engine. Only
//!   a request that redeems an unused, unexpired token whose identity still matches the
//!   file reaches the agent, and the agent gets the file's content (read once, through the
//!   handle that passed the checks, as an embedded resource) rather than a path to open.
//! * An image is decoded and always re-encoded as PNG (after applying its orientation), so
//!   metadata and anything appended to the file never reach the agent.
//!
//! Every file must be a regular file with a single directory entry (no hard links, links
//! or reparse points) under a size cap. A deny list (environment files, private keys,
//! `.git`, `.ssh`) is applied to the spelled path and again to the canonical path
//! (components compared after trailing dots, spaces and invisible format characters are
//! dropped, so `.env `, `a.pem.` and 8.3 names cannot slip past); on Windows any `:` in a
//! name (alternate data streams) is refused. It cannot be confirmed past.
//!
//! What this does and does not guarantee: the path is resolved and the handle is opened
//! without following a link at the final component, and the identity of the opened file
//! (volume and file index, or device and inode) is checked against the path before and
//! after. A directory on the way can still be swapped by someone with write access to it
//! between the check and a later read of a project file; that is why only file content
//! read through the checked handle is ever sent for outside files. Nothing here is
//! persisted: the conversation log and the session store only see an attachment's label.

use super::WorkflowError;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, metadata::Orientation};
use parking_lot::Mutex;
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
use studio_project::ProjectPath;

/// Largest image the composer takes in, before decoding.
const MAX_IMAGE_INPUT_BYTES: usize = 16 * 1024 * 1024;
/// Largest PNG one attachment may become.
pub(crate) const MAX_ATTACHED_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_IMAGE_SIDE: u32 = 8192;
const MAX_DECODE_ALLOC: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ATTACHED_IMAGES: usize = 3;
pub(crate) const MAX_ATTACHED_FILES: usize = 16;
/// Largest file from outside the project: its content travels in the message.
pub(crate) const MAX_EMBEDDED_FILE_BYTES: u64 = 1024 * 1024;
/// Largest project file that may be linked.
const MAX_LINKED_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Longest label shown for an attachment.
const MAX_LABEL_CHARS: usize = 120;
/// How long a staged outside file waits for its confirmation.
const STAGE_TTL: Duration = Duration::from_secs(5 * 60);
/// Staged files kept at once; the oldest is dropped beyond this.
const MAX_STAGED: usize = 32;

/// Identity of an open file: the volume serial number and file index on Windows, device
/// and inode elsewhere. Two paths that name the same file have the same identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume: u64,
    pub index: u64,
}

/// Opaque one-shot confirmation for an outside file. Redeemed by
/// [`AttachmentRequest::Staged`]; the engine holds everything it stands for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StageToken(String);

/// An outside file that passed the checks and waits for the user's confirmation.
#[derive(Debug, Clone)]
pub struct StagedAttachment {
    pub token: StageToken,
    /// The canonical path: this is what the user is asked to confirm.
    pub path: PathBuf,
    /// The file name as the agent will see it.
    pub label: String,
    pub size: u64,
    pub identity: FileIdentity,
}

/// What the UI asks to attach.
#[derive(Debug, Clone)]
pub enum AttachmentRequest {
    /// A file inside the project: a project path (relative to the project root, or
    /// absolute inside it). A path that lands outside is refused; stage it first.
    File { path: PathBuf },
    /// An outside file the user confirmed after [`stage`] showed it.
    Staged { token: StageToken },
    /// An image the user pasted or dropped, in any format the app can decode.
    Image { label: String, bytes: Vec<u8> },
}

enum Inner {
    /// A project file, linked inside the agent's working copy when it is sent.
    ProjectFile {
        label: String,
        relative: String,
    },
    /// An outside file, sent as content.
    Embedded {
        label: String,
        bytes: Vec<u8>,
    },
    Image {
        label: String,
        png: Vec<u8>,
    },
}

/// An attachment that passed the engine's checks. Only this crate can construct one, so
/// the send operations cannot be handed an unchecked file or image.
pub struct PreparedAttachment(Inner);

impl std::fmt::Debug for PreparedAttachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Inner::ProjectFile { label, .. } => write!(f, "PreparedAttachment::File({label})"),
            Inner::Embedded { label, bytes } => {
                write!(
                    f,
                    "PreparedAttachment::Embedded({label}, {} bytes)",
                    bytes.len()
                )
            }
            Inner::Image { label, png } => {
                write!(f, "PreparedAttachment::Image({label}, {} bytes)", png.len())
            }
        }
    }
}

impl PreparedAttachment {
    /// The name shown on the chip and in the conversation.
    pub fn label(&self) -> &str {
        match &self.0 {
            Inner::ProjectFile { label, .. }
            | Inner::Embedded { label, .. }
            | Inner::Image { label, .. } => label,
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self.0, Inner::Image { .. })
    }

    /// Whether the file's content travels in the message (it needs an agent that takes
    /// embedded context).
    pub fn is_embedded(&self) -> bool {
        matches!(self.0, Inner::Embedded { .. })
    }

    /// Splits into the files and PNG images the driver takes. A project file becomes a
    /// link inside `draft`, the agent's working copy, which must hold it as a plain file.
    pub(crate) fn split(
        attachments: Vec<Self>,
        draft: &Path,
    ) -> Result<(Vec<studio_harness::driver::PromptFile>, Vec<Vec<u8>>), String> {
        use studio_harness::driver::PromptFile;
        let mut files = Vec::new();
        let mut images = Vec::new();
        for attachment in attachments {
            match attachment.0 {
                Inner::ProjectFile { label, relative } => {
                    let path = ProjectPath::try_from(relative)
                        .and_then(|relative| relative.resolve_existing(draft))
                        .map_err(|_| {
                            format!("`{label}` is not a plain file in the agent's working copy.")
                        })?;
                    files.push(PromptFile::new(label, path));
                }
                Inner::Embedded { label, bytes } => files.push(PromptFile::embedded(label, bytes)),
                Inner::Image { png, .. } => images.push(png),
            }
        }
        Ok((files, images))
    }
}

fn refused(message: impl Into<String>) -> WorkflowError {
    WorkflowError::Refused(message.into())
}

// ---- labels -------------------------------------------------------------------------------------------

/// Whether `c` is a format character (bidirectional controls, zero-width characters, the
/// byte order mark, tags): invisible, so it could disguise what a label says.
fn is_format_char(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

/// A label that is safe to show: no control or format characters, bounded, never empty.
pub(crate) fn clean_label(label: &str, fallback: &str) -> String {
    let cleaned: String = label
        .chars()
        .filter(|c| !c.is_control() && !is_format_char(*c))
        .collect();
    let cleaned: String = cleaned.trim().chars().take(MAX_LABEL_CHARS).collect();
    if cleaned.is_empty() {
        fallback.to_owned()
    } else {
        cleaned
    }
}

// ---- the deny list ------------------------------------------------------------------------------------

/// One path component as a filesystem that ignores trailing dots and spaces and a reader
/// who cannot see format characters would compare it.
fn folded(name: &str) -> String {
    let visible: String = name.chars().filter(|c| !is_format_char(*c)).collect();
    visible.trim_end_matches(['.', ' ']).to_lowercase()
}

/// Why `name` may never be attached, if it may not.
fn denied_component(name: &str) -> Option<&'static str> {
    let lower = folded(name);
    if lower == ".env" || lower.starts_with(".env.") {
        Some("environment files hold secrets")
    } else if lower.ends_with(".pem") || lower.ends_with(".key") {
        Some("key files hold secrets")
    } else if lower == ".git" {
        Some("the .git folder is repository internals")
    } else if lower == ".ssh" {
        Some("the .ssh folder holds private keys")
    } else {
        None
    }
}

/// The deny-list verdict for every component of `path`.
fn check_denied(path: &Path) -> Result<(), WorkflowError> {
    for component in path.components() {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy();
            if let Some(reason) = denied_component(&name) {
                return Err(refused(format!(
                    "`{}` cannot be attached: {reason}. This is never allowed, even when confirmed.",
                    clean_label(&name, "that name")
                )));
            }
        }
    }
    Ok(())
}

/// Refuses spellings that name something other than a plain file by its path: alternate
/// data streams (`name:stream`), device paths and drive-relative paths on Windows.
fn check_spelling(path: &Path) -> Result<(), WorkflowError> {
    #[cfg(windows)]
    {
        use std::path::Prefix;
        let mut components = path.components().peekable();
        if let Some(Component::Prefix(prefix)) = components.peek() {
            match prefix.kind() {
                Prefix::Disk(_)
                | Prefix::VerbatimDisk(_)
                | Prefix::UNC(..)
                | Prefix::VerbatimUNC(..) => {}
                _ => return Err(refused("that path is a device path, not a file path")),
            }
            components.next();
            if !matches!(components.peek(), Some(Component::RootDir)) {
                return Err(refused(
                    "give the full path, including the drive and the root",
                ));
            }
        }
        for component in components {
            if let Component::Normal(name) = component
                && name.to_string_lossy().contains(':')
            {
                return Err(refused(
                    "a file name with `:` (an alternate data stream) cannot be attached",
                ));
            }
        }
    }
    check_denied(path)
}

// ---- opening and checking ---------------------------------------------------------------------------

fn is_link(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// Identity and directory-entry count of an open file.
#[cfg(unix)]
fn handle_facts(file: &File) -> std::io::Result<(FileIdentity, u64)> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    Ok((
        FileIdentity {
            volume: meta.dev(),
            index: meta.ino(),
        },
        meta.nlink(),
    ))
}

#[cfg(windows)]
fn handle_facts(file: &File) -> std::io::Result<(FileIdentity, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    // SAFETY: the structure is plain data that the call fills in; the handle stays open
    // for the duration of the call because `file` is borrowed.
    let info = unsafe {
        let mut info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
        if GetFileInformationByHandle(file.as_raw_handle(), &mut info) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        info
    };
    Ok((
        FileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        u64::from(info.nNumberOfLinks),
    ))
}

/// What an open file must be: a regular file with one directory entry, under `cap`.
/// Returns its identity and size.
fn check_open_file(
    file: &File,
    shown: &Path,
    cap: u64,
) -> Result<(FileIdentity, u64), WorkflowError> {
    let unreadable =
        |error: std::io::Error| refused(format!("{} cannot be read: {error}", shown.display()));
    let meta = file.metadata().map_err(unreadable)?;
    if !meta.is_file() || is_link(&meta) {
        return Err(refused(format!(
            "{} is not a regular file.",
            shown.display()
        )));
    }
    let (identity, links) = handle_facts(file).map_err(unreadable)?;
    if links != 1 {
        return Err(refused(format!(
            "{} has more than one name on disk (a hard link) and cannot be attached.",
            shown.display()
        )));
    }
    if meta.len() > cap {
        return Err(refused(format!(
            "{} is larger than {} KiB.",
            shown.display(),
            cap / 1024
        )));
    }
    Ok((identity, meta.len()))
}

/// Opens `path` without following a link at its last component.
fn open_no_follow(path: &Path) -> Result<File, WorkflowError> {
    let unreadable =
        |error: std::io::Error| refused(format!("{} cannot be read: {error}", path.display()));
    let meta = fs::symlink_metadata(path).map_err(unreadable)?;
    if is_link(&meta) {
        return Err(refused(format!(
            "{} is a link; attach the file it points to instead.",
            path.display()
        )));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT: open a link itself, never its target.
        options.custom_flags(0x0020_0000);
    }
    options.open(path).map_err(unreadable)
}

/// Where a requested file really is.
struct Located {
    canonical: PathBuf,
    /// The portable path inside the project, when the file is inside.
    inside: Option<String>,
}

fn locate(root: &Path, requested: &Path) -> Result<Located, WorkflowError> {
    check_spelling(requested)?;
    let absolute = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    let unreadable = |path: &Path, error: std::io::Error| {
        refused(format!("{} cannot be read: {error}", path.display()))
    };
    let meta = fs::symlink_metadata(&absolute).map_err(|error| unreadable(&absolute, error))?;
    if is_link(&meta) {
        return Err(refused(format!(
            "{} is a link; attach the file it points to instead.",
            absolute.display()
        )));
    }
    // Everything on the way is resolved: a link or junction in a directory is judged by
    // where it really leads, so one that leads into `.ssh` is refused and one that leads
    // out of the project is an outside file.
    let canonical = fs::canonicalize(&absolute).map_err(|error| unreadable(&absolute, error))?;
    let root_canonical = fs::canonicalize(root).map_err(|error| unreadable(root, error))?;
    match canonical.strip_prefix(&root_canonical) {
        Ok(relative) => {
            check_denied(relative)?;
            let portable = relative
                .components()
                .map(|component| match component {
                    Component::Normal(name) => name
                        .to_str()
                        .map(str::to_owned)
                        .ok_or_else(|| refused("that file name is not valid text")),
                    _ => Err(refused("that path is not a valid project path")),
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("/");
            if studio_project::paths::protected_tree(&portable).is_some() {
                return Err(refused(format!(
                    "`{}` is not project source and cannot be attached.",
                    clean_label(&portable, "that path")
                )));
            }
            Ok(Located {
                canonical,
                inside: Some(portable),
            })
        }
        Err(_) => {
            check_denied(&canonical)?;
            Ok(Located {
                canonical,
                inside: None,
            })
        }
    }
}

// ---- staging outside files ----------------------------------------------------------------------------

struct StagedEntry {
    token: StageToken,
    path: PathBuf,
    identity: FileIdentity,
    expires: Instant,
}

/// The confirmations the engine holds for outside files. A token is redeemed once and
/// expires; nothing about it is trusted from the request that carries it.
pub(crate) struct StageStore {
    ttl: Duration,
    entries: Mutex<Vec<StagedEntry>>,
}

impl Default for StageStore {
    fn default() -> Self {
        Self::with_ttl(STAGE_TTL)
    }
}

impl StageStore {
    pub(crate) fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(Vec::new()),
        }
    }

    fn insert(&self, path: PathBuf, identity: FileIdentity) -> StageToken {
        let token = StageToken(uuid::Uuid::new_v4().simple().to_string());
        let mut entries = self.entries.lock();
        let now = Instant::now();
        entries.retain(|entry| entry.expires > now);
        while entries.len() >= MAX_STAGED {
            entries.remove(0);
        }
        entries.push(StagedEntry {
            token: token.clone(),
            path,
            identity,
            expires: now + self.ttl,
        });
        token
    }

    /// Takes the entry for `token` out: a token works once, and only before it expires.
    fn redeem(&self, token: &StageToken) -> Option<(PathBuf, FileIdentity)> {
        let mut entries = self.entries.lock();
        let position = entries.iter().position(|entry| &entry.token == token)?;
        let entry = entries.remove(position);
        (entry.expires > Instant::now()).then_some((entry.path, entry.identity))
    }
}

/// Checks an outside file and parks a confirmation token for it. The user confirms the
/// returned canonical path; [`AttachmentRequest::Staged`] with the token then attaches it.
pub(crate) fn stage(
    root: &Path,
    store: &StageStore,
    requested: &Path,
) -> Result<StagedAttachment, WorkflowError> {
    if !requested.is_absolute() {
        return Err(refused(
            "a path outside the project must be given as an absolute path",
        ));
    }
    let located = locate(root, requested)?;
    if located.inside.is_some() {
        return Err(refused(
            "that file is inside the project; attach it by its project path",
        ));
    }
    let file = open_no_follow(&located.canonical)?;
    let (identity, size) = check_open_file(&file, &located.canonical, MAX_EMBEDDED_FILE_BYTES)?;
    // The path must still lead to the file that was opened.
    confirm_same_file(&located.canonical, identity)?;
    let label = located
        .canonical
        .file_name()
        .map(|name| clean_label(&name.to_string_lossy(), "file"))
        .unwrap_or_else(|| "file".to_owned());
    let token = store.insert(located.canonical.clone(), identity);
    Ok(StagedAttachment {
        token,
        path: located.canonical,
        label,
        size,
        identity,
    })
}

/// `path` canonicalises to itself and names the file with `identity`.
fn confirm_same_file(path: &Path, identity: FileIdentity) -> Result<(), WorkflowError> {
    let changed = || {
        refused(format!(
            "{} changed while it was being checked; choose it again.",
            path.display()
        ))
    };
    if fs::canonicalize(path).map_err(|_| changed())? != path {
        return Err(changed());
    }
    let again = open_no_follow(path)?;
    let (now, _) = handle_facts(&again).map_err(|_| changed())?;
    if now != identity {
        return Err(changed());
    }
    Ok(())
}

fn redeem_staged(
    store: &StageStore,
    token: &StageToken,
) -> Result<PreparedAttachment, WorkflowError> {
    let (path, identity) = store.redeem(token).ok_or_else(|| {
        refused("That confirmation expired or was already used; choose the file again.")
    })?;
    // The checks run again on what is on disk now.
    check_spelling(&path)?;
    check_denied(&path)?;
    let mut file = open_no_follow(&path)?;
    let (opened, _) = check_open_file(&file, &path, MAX_EMBEDDED_FILE_BYTES)?;
    if opened != identity {
        return Err(refused(format!(
            "{} is not the file that was confirmed; choose it again.",
            path.display()
        )));
    }
    confirm_same_file(&path, identity)?;
    // The content comes through the handle that passed the checks, bounded again in case
    // the file grew.
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_EMBEDDED_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| refused(format!("{} cannot be read: {error}", path.display())))?;
    if bytes.len() as u64 > MAX_EMBEDDED_FILE_BYTES {
        return Err(refused(format!(
            "{} is larger than {} KiB.",
            path.display(),
            MAX_EMBEDDED_FILE_BYTES / 1024
        )));
    }
    let label = path
        .file_name()
        .map(|name| clean_label(&name.to_string_lossy(), "file"))
        .unwrap_or_else(|| "file".to_owned());
    Ok(PreparedAttachment(Inner::Embedded { label, bytes }))
}

// ---- project files ------------------------------------------------------------------------------------

fn prepare_project_file(
    root: &Path,
    requested: &Path,
) -> Result<PreparedAttachment, WorkflowError> {
    let located = locate(root, requested)?;
    let Some(portable) = located.inside else {
        return Err(refused(format!(
            "{} is outside the project. Stage it and confirm attaching this file to send it to the agent.",
            requested.display()
        )));
    };
    let project_path = ProjectPath::try_from(portable.clone())
        .map_err(|error| refused(format!("not a valid project path: {error}")))?;
    let label = clean_label(&portable, "file");
    let file = project_path
        .open_file(root)
        .map_err(|error| refused(format!("`{label}` cannot be attached: {error}")))?;
    check_open_file(&file, &located.canonical, MAX_LINKED_FILE_BYTES)?;
    Ok(PreparedAttachment(Inner::ProjectFile {
        label,
        relative: portable,
    }))
}

// ---- images -------------------------------------------------------------------------------------------

/// Decodes at most one image at a time: decoding is the heavy part of attaching.
static DECODE_GATE: Mutex<()> = Mutex::new(());

fn prepare_image(label: &str, bytes: &[u8]) -> Result<PreparedAttachment, WorkflowError> {
    let label = clean_label(label, "image");
    if bytes.is_empty() {
        return Err(refused("the image is empty"));
    }
    if bytes.len() > MAX_IMAGE_INPUT_BYTES {
        return Err(refused(format!(
            "`{label}` is larger than {} MiB",
            MAX_IMAGE_INPUT_BYTES / (1024 * 1024)
        )));
    }
    let not_readable = || refused(format!("`{label}` is not an image the app can read"));
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| not_readable())?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);
    reader.format().ok_or_else(not_readable)?;
    let _gate = DECODE_GATE.lock();
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| refused(format!("`{label}` cannot be used as an image: {error}")))?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut decoded = DynamicImage::from_decoder(decoder)
        .map_err(|error| refused(format!("`{label}` cannot be used as an image: {error}")))?;
    decoded.apply_orientation(orientation);
    // Always re-encoded, a PNG included: only pixels survive, never metadata or bytes
    // appended after the image.
    let mut out = Cursor::new(Vec::new());
    decoded
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|error| refused(format!("`{label}` could not be converted to PNG: {error}")))?;
    let png = out.into_inner();
    if png.len() > MAX_ATTACHED_IMAGE_BYTES {
        return Err(refused(format!(
            "`{label}` is larger than {} MiB as a PNG",
            MAX_ATTACHED_IMAGE_BYTES / (1024 * 1024)
        )));
    }
    Ok(PreparedAttachment(Inner::Image { label, png }))
}

pub(crate) fn prepare(
    root: &Path,
    store: &StageStore,
    request: AttachmentRequest,
) -> Result<PreparedAttachment, WorkflowError> {
    match request {
        AttachmentRequest::File { path } => prepare_project_file(root, &path),
        AttachmentRequest::Staged { token } => redeem_staged(store, &token),
        AttachmentRequest::Image { label, bytes } => prepare_image(&label, &bytes),
    }
}

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod tests;
