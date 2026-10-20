//! Which permission requests Studio answers itself under the user's [`PermissionPolicy`].
//!
//! Only what the policy clearly covers is answered, always with the agent's own
//! `allow_once` option (never `allow_always`, which would change the agent's stored rules).
//! Anything unclear (no kind, no locations, a path that leaves the working copy, no
//! `allow_once` option) is left to the user. Questions never come here.

use std::path::{Component, Path};

use studio_harness::driver::PermissionPrompt;
use studio_proto::PermissionPolicy;

/// Kinds "Allow in the working copy" covers when every location is inside the draft.
const WORKING_COPY_KINDS: [&str; 5] = ["read", "search", "edit", "delete", "move"];

/// The option Studio answers `prompt` with under `policy`, or `None` to ask the user.
/// `draft` is the task's working copy, the agent's working directory (relative locations
/// are resolved against it).
pub(crate) fn auto_answer(
    policy: PermissionPolicy,
    prompt: &PermissionPrompt,
    draft: &Path,
) -> Option<String> {
    let covered = match policy {
        PermissionPolicy::AskEveryTime => false,
        PermissionPolicy::AllowEverything => true,
        PermissionPolicy::AllowInWorkingCopy => match prompt.kind.as_deref() {
            Some("think") => true,
            Some(kind) if WORKING_COPY_KINDS.contains(&kind) => {
                !prompt.locations.is_empty()
                    && draft.canonicalize().is_ok_and(|root| {
                        prompt
                            .locations
                            .iter()
                            .all(|location| resolves_inside(&root, location))
                    })
            }
            _ => false,
        },
    };
    if !covered {
        return None;
    }
    prompt
        .options
        .iter()
        .find(|option| option.kind == "allow_once")
        .map(|option| option.option_id.clone())
}

/// Whether `location` (absolute, or relative to `root`) is `root` or inside it once links
/// are followed. A path that does not exist yet is judged by its nearest existing ancestor,
/// and its missing part must be plain names (no `..`).
fn resolves_inside(root: &Path, location: &Path) -> bool {
    let path = root.join(location);
    let mut existing = path.as_path();
    let mut missing_plain = true;
    loop {
        match existing.canonicalize() {
            Ok(real) => return missing_plain && real.starts_with(root),
            Err(_) => {
                missing_plain &= matches!(
                    existing.components().next_back(),
                    Some(Component::Normal(_))
                );
                let Some(parent) = existing.parent() else {
                    return false;
                };
                existing = parent;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use studio_harness::driver::{PermissionChoice, PermissionId};

    fn prompt(kind: Option<&str>, locations: &[&Path], options: &[&str]) -> PermissionPrompt {
        PermissionPrompt {
            id: PermissionId(1),
            turn: 1,
            tool_call_id: "call".into(),
            title: "t".into(),
            options: options
                .iter()
                .map(|kind| PermissionChoice {
                    option_id: format!("id-{kind}"),
                    name: (*kind).into(),
                    kind: (*kind).into(),
                })
                .collect(),
            kind: kind.map(str::to_owned),
            locations: locations.iter().map(|p| p.to_path_buf()).collect(),
        }
    }

    const OFFERED: [&str; 3] = ["allow_always", "allow_once", "reject_once"];

    #[test]
    fn working_copy_allows_edits_inside_the_draft_once_and_asks_for_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let draft = dir.path().join("draft");
        std::fs::create_dir_all(draft.join("src")).unwrap();
        std::fs::write(draft.join("src/lib.rs"), "").unwrap();
        let policy = PermissionPolicy::AllowInWorkingCopy;
        let answer = |kind, locations: &[&Path]| {
            auto_answer(policy, &prompt(Some(kind), locations, &OFFERED), &draft)
        };
        let once = Some("id-allow_once".to_owned());
        // Existing file, relative and absolute; a new file in a new folder.
        assert_eq!(answer("edit", &[Path::new("src/lib.rs")]), once);
        assert_eq!(answer("edit", &[&draft.join("src/lib.rs")]), once);
        assert_eq!(answer("edit", &[Path::new("src/new/scene.rs")]), once);
        assert_eq!(answer("read", &[Path::new(".")]), once);
        assert_eq!(answer("think", &[]), once);
        // Leaving the draft, lexically or by a missing `..`, or not saying where.
        assert_eq!(answer("edit", &[Path::new("../outside.rs")]), None);
        assert_eq!(answer("edit", &[Path::new("src/new/../../../x.rs")]), None);
        assert_eq!(answer("edit", &[&dir.path().join("other.rs")]), None);
        assert_eq!(
            answer("edit", &[Path::new("src/lib.rs"), &dir.path().join("o")]),
            None
        );
        assert_eq!(answer("edit", &[]), None);
        // Kinds the policy never covers.
        assert_eq!(answer("execute", &[Path::new("src/lib.rs")]), None);
        assert_eq!(answer("fetch", &[]), None);
        assert_eq!(answer("other", &[Path::new("src/lib.rs")]), None);
        let unknown = prompt(None, &[Path::new("src/lib.rs")], &OFFERED);
        assert_eq!(auto_answer(policy, &unknown, &draft), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_draft_counts_as_outside() {
        let dir = tempfile::tempdir().unwrap();
        let draft = dir.path().join("draft");
        std::fs::create_dir_all(&draft).unwrap();
        std::os::unix::fs::symlink(dir.path(), draft.join("escape")).unwrap();
        let request = prompt(Some("edit"), &[Path::new("escape/x.rs")], &OFFERED);
        assert_eq!(
            auto_answer(PermissionPolicy::AllowInWorkingCopy, &request, &draft),
            None
        );
    }

    #[test]
    fn the_other_policies_and_missing_options() {
        let dir = tempfile::tempdir().unwrap();
        let draft = PathBuf::from(dir.path());
        let command = prompt(Some("execute"), &[], &OFFERED);
        assert_eq!(
            auto_answer(PermissionPolicy::AskEveryTime, &command, &draft),
            None
        );
        assert_eq!(
            auto_answer(PermissionPolicy::AllowEverything, &command, &draft),
            Some("id-allow_once".to_owned())
        );
        let unknown = prompt(None, &[], &OFFERED);
        assert!(auto_answer(PermissionPolicy::AllowEverything, &unknown, &draft).is_some());
        let edit = prompt(Some("edit"), &[Path::new("a.rs")], &OFFERED);
        assert_eq!(
            auto_answer(PermissionPolicy::AskEveryTime, &edit, &draft),
            None
        );
        // Never `allow_always`, and nothing when the agent offers no single allow.
        let always_only = prompt(
            Some("edit"),
            &[Path::new("a.rs")],
            &["allow_always", "reject_once"],
        );
        assert_eq!(
            auto_answer(PermissionPolicy::AllowEverything, &always_only, &draft),
            None
        );
    }
}
