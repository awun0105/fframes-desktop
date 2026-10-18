//! Capturing a draft into an immutable candidate.

use super::*;

#[test]
fn capture_names_identities_hashes_and_changed_paths_and_registers_with_the_task() {
    let f = fixture();
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let context = start(&mut controller);
    fs::write(context.draft.join("src/lib.rs"), "// edited by the agent").unwrap();
    fs::write(context.draft.join("media/new.txt"), "new asset").unwrap();
    let t = ticket(&mut controller, &context);
    let store = checkpoints(&f, &controller);
    let captured = capture_candidate(&t, &store).unwrap();
    assert_eq!(captured.identity(), &context.identity);
    assert_eq!(captured.source_base(), &context.source_base);
    assert_ne!(
        captured.candidate().revision(),
        context.source_base.revision()
    );
    let manifest_bytes = fs::read(store.manifest_path(captured.candidate().revision())).unwrap();
    assert_eq!(
        captured.manifest_sha256(),
        format!("{:x}", sha2_digest(&manifest_bytes))
    );
    assert!(captured.objects().iter().any(|o| o.path == "studio.json"));
    let changes = captured.changes();
    assert_eq!(changes.total, 2);
    assert!(changes.entries.iter().any(|c| c.path == "src/lib.rs"
        && c.change == ChangeKind::Modified
        && c.kind == studio_project::revision::FileKind::Rust));
    assert!(
        changes
            .entries
            .iter()
            .any(|c| c.path == "media/new.txt" && c.change == ChangeKind::Added)
    );
    // The candidate registers as a distinct type and starts validation.
    controller.agent_record_candidate(t, &captured).unwrap();
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::Validating
    );
    // Source promotion is not part of capture: the live project is untouched.
    assert_ne!(
        fs::read_to_string(f.root.join("src/lib.rs")).unwrap(),
        "// edited by the agent"
    );
}

fn sha2_digest(bytes: &[u8]) -> impl std::fmt::LowerHex {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
}

#[test]
fn unchanged_candidate_has_the_base_revision_and_no_changes() {
    let f = fixture();
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let context = start(&mut controller);
    let t = ticket(&mut controller, &context);
    let captured = capture_candidate(&t, &checkpoints(&f, &controller)).unwrap();
    assert_eq!(
        captured.candidate().revision(),
        captured.source_base().revision()
    );
    assert!(captured.changes().is_empty());
}

#[test]
fn a_draft_that_changes_during_capture_fails_at_either_verification() {
    for phase in [CapturePhase::Scanned, CapturePhase::Captured] {
        let f = fixture();
        let mut controller = Controller::open(&f.root, &f.paths).unwrap();
        let context = start(&mut controller);
        let t = ticket(&mut controller, &context);
        let store = checkpoints(&f, &controller);
        let draft = context.draft.clone();
        let error = capture_candidate_observed(&t, &store, &mut |p| {
            if p == phase {
                fs::write(draft.join("src/lib.rs"), format!("// moved during {p:?}")).unwrap();
            }
        })
        .unwrap_err();
        assert_eq!(error, CandidateError::DraftChanged, "{phase:?}");
        // Nothing was registered: the task still waits for a stable capture.
        assert_eq!(
            controller.agent_task().unwrap().state(),
            TaskState::Quiescing
        );
        assert!(controller.agent_task().unwrap().candidate().is_none());
    }
}

#[test]
fn invalid_drafts_and_foreign_project_ids_are_rejected_before_validation() {
    let f = fixture();
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let context = start(&mut controller);
    let manifest = fs::read_to_string(context.draft.join("studio.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    value["project_id"] = serde_json::json!("another-project-identity");
    fs::write(
        context.draft.join("studio.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let t = ticket(&mut controller, &context);
    let store = checkpoints(&f, &controller);
    assert!(matches!(
        capture_candidate(&t, &store).unwrap_err(),
        CandidateError::ProjectMismatch { .. }
    ));
    fs::remove_file(context.draft.join("studio.json")).unwrap();
    assert!(matches!(
        capture_candidate(&t, &store).unwrap_err(),
        CandidateError::InvalidDraft(_)
    ));
    #[cfg(unix)]
    {
        fs::write(context.draft.join("studio.json"), manifest).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", context.draft.join("media/link")).unwrap();
        assert!(matches!(
            capture_candidate(&t, &store).unwrap_err(),
            CandidateError::InvalidDraft(_)
        ));
    }
}

#[test]
fn restored_revision_is_a_private_immutable_tree_removed_on_drop() {
    let f = fixture();
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let context = start(&mut controller);
    fs::write(context.draft.join("src/lib.rs"), "// restored").unwrap();
    let t = ticket(&mut controller, &context);
    let store = checkpoints(&f, &controller);
    let captured = capture_candidate(&t, &store).unwrap();
    let restored = restore_revision(&store, captured.candidate().revision()).unwrap();
    assert_eq!(
        &restored.project.inventory.revision,
        captured.candidate().revision()
    );
    assert_eq!(
        fs::read_to_string(restored.root().join("src/lib.rs")).unwrap(),
        "// restored"
    );
    // Editing the stable draft afterwards cannot affect the restored tree.
    fs::write(context.draft.join("src/lib.rs"), "// later").unwrap();
    assert_eq!(
        fs::read_to_string(restored.root().join("src/lib.rs")).unwrap(),
        "// restored"
    );
    let root = restored.root().to_owned();
    drop(restored);
    assert!(!root.exists());
}
