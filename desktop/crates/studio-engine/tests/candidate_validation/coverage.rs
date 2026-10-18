//! Which frames a validation covers, and when it broadens.

use super::*;

#[test]
fn passing_report_names_every_identity_and_needs_no_pixel_or_title_change() {
    let f = fixture();
    // No source change at all: frame zero cannot differ from the base, and that is fine.
    let (_c, context, captured) = captured_with(&f, |_| {});
    let revision = captured.candidate().revision().as_str().to_owned();
    let mut probe = Fake::new(&revision, 300);
    let report = validate_candidate(
        &captured,
        BuildOutcome::Built(build_identity(&captured)),
        123,
        0,
        Some(&mut probe),
    );
    assert!(report.passed(), "{:?}", report.failure());
    assert_eq!(report.task(), &context.identity);
    assert_eq!(report.source_base(), &context.source_base);
    assert_eq!(report.candidate(), captured.candidate());
    assert_eq!(report.manifest_sha256(), captured.manifest_sha256());
    assert_eq!(report.build().unwrap().key_digest, "k".repeat(64));
    assert_eq!(report.audio().unwrap().sample_rate, 48_000);
    let coverage = report.coverage().unwrap();
    assert_eq!(coverage.broadened, None);
    assert!(coverage.complete);
    assert_eq!(coverage.playhead, 123);
    for required in [0usize, 299, 123, 150] {
        assert!(coverage.rendered_frames.contains(&required), "{required}");
    }
    assert!(coverage.sample_frames.len() <= MAX_SAMPLE_FRAMES);
    assert_eq!(report.frames().len(), coverage.rendered_frames.len());
    // Identical frames hash identically: no pixel-difference requirement.
    assert!(
        report
            .frames()
            .windows(2)
            .all(|w| w[0].sha256 == w[1].sha256)
    );
    assert!(report.changes().is_empty());
    // The report is serializable evidence.
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains(&revision));
}

#[test]
fn coverage_has_boundaries_playhead_and_at_most_24_evenly_spaced_frames() {
    let t = timeline("r", 1000, &[(0, 300), (300, 700), (700, 1000)], &[]);
    let plan = plan_coverage(&t, 421, &ChangeSet::default());
    assert_eq!(plan.broadened, None);
    for boundary in [0usize, 299, 300, 699, 700, 999] {
        assert!(plan.boundary_frames.contains(&boundary));
        assert!(plan.rendered_frames.contains(&boundary));
        assert!(plan.inspect_frames.contains(&boundary));
    }
    assert!(plan.rendered_frames.contains(&421));
    assert_eq!(plan.sample_frames.len(), MAX_SAMPLE_FRAMES);
    assert_eq!(plan.sample_frames[0], 0);
    assert_eq!(*plan.sample_frames.last().unwrap(), 999);
    let gaps: Vec<usize> = plan.sample_frames.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.iter().max().unwrap() - gaps.iter().min().unwrap() <= 1);
    assert!(plan.rendered_frames.len() <= MAX_RENDERED_FRAMES);
    assert!(plan.complete);
    // Short videos never ask for frames that do not exist.
    let short = timeline("r", 5, &[], &[]);
    let plan = plan_coverage(&short, 99, &ChangeSet::default());
    assert!(plan.rendered_frames.iter().all(|f| *f < 5));
    assert_eq!(evenly_spaced(5, 24), vec![0, 1, 2, 3, 4]);
    assert_eq!(evenly_spaced(0, 24), Vec::<usize>::new());
}

#[test]
fn scoped_coverage_requires_selected_samples_and_refuses_shrunken_candidates() {
    let scope = TaskScope {
        project_id: "p".into(),
        source_revision: "a".repeat(64),
        selection: ScopeSelection::FrameRange,
        compiled: Some(CompiledScope {
            preview: PreviewIdentity {
                project_id: "p".into(),
                open_session: "s".into(),
                source_revision: "a".repeat(64),
                worker_generation: 1,
            },
            fps: 30,
            total_frames: 1000,
            start_frame: 300,
            end_frame: 400,
            scenes: vec![ScopedScene {
                instance_id: "scene-1".into(),
                name: "Selected".into(),
                full_name: "Video::Selected".into(),
                start_frame: 250,
                end_frame: 450,
            }],
            boundary_frames: vec![249, 250, 299, 300, 399, 400, 449, 450],
            scene_context_truncated: false,
        }),
        scene_sources: vec![],
        scene_source_search_truncated: false,
        style_snapshot: None,
        canvas_selection: None,
    };
    scope.validate().unwrap();

    let candidate = timeline("x", 1000, &[(0, 1000)], &[]);
    let plan = plan_coverage_scoped(&candidate, 10, &ChangeSet::default(), Some(&scope));
    assert_eq!(plan.requested_scope, "Frames [300..400)");
    assert_eq!(plan.requested_interval, Some([300, 400]));
    assert!(plan.complete);
    for frame in &plan.requested_frames {
        assert!(
            plan.inspect_frames.contains(frame),
            "not inspected: {frame}"
        );
        assert!(
            plan.rendered_frames.contains(frame),
            "not rendered: {frame}"
        );
    }
    assert!(plan.requested_frames.contains(&300));
    assert!(plan.requested_frames.contains(&399));
    assert!(plan.requested_frames.contains(&249));
    assert!(plan.requested_frames.contains(&450));

    let shortened = timeline("x", 350, &[(0, 350)], &[]);
    let plan = plan_coverage_scoped(&shortened, 10, &ChangeSet::default(), Some(&scope));
    assert!(!plan.complete);
    assert!(plan.requested_frames.iter().all(|frame| *frame < 350));
}

fn change(path: &str, change: ChangeKind, kind: studio_project::revision::FileKind) -> ChangedPath {
    ChangedPath {
        path: path.into(),
        change,
        kind,
    }
}

#[test]
fn shared_source_deletions_and_truncated_changes_broaden_to_the_full_video_in_bounded_batches() {
    use studio_project::revision::FileKind::*;
    let t = timeline("r", 1000, &[(0, 500), (500, 1000)], &[]);
    let set = |entries: Vec<ChangedPath>, truncated| ChangeSet {
        total: entries.len(),
        entries,
        truncated,
    };
    assert_eq!(
        broadening(&set(
            vec![change("src/lib.rs", ChangeKind::Modified, Rust)],
            false
        )),
        Some(BroadenReason::SharedSourceChanged)
    );
    assert_eq!(
        broadening(&set(
            vec![change("Cargo.toml", ChangeKind::Modified, Cargo)],
            false
        )),
        Some(BroadenReason::SharedSourceChanged)
    );
    assert_eq!(
        broadening(&set(
            vec![change("media/a.png", ChangeKind::Deleted, Media)],
            false
        )),
        Some(BroadenReason::ExistingFilesChanged)
    );
    assert_eq!(
        broadening(&set(
            vec![change("media/a.png", ChangeKind::Modified, Media)],
            false
        )),
        Some(BroadenReason::ExistingFilesChanged)
    );
    assert_eq!(
        broadening(&set(vec![], true)),
        Some(BroadenReason::ChangeListTruncated)
    );
    // Added media or instruction-only edits keep the targeted coverage.
    assert_eq!(
        broadening(&set(
            vec![change("media/n.png", ChangeKind::Added, Media)],
            false
        )),
        None
    );
    assert_eq!(
        broadening(&set(
            vec![change("AGENTS.md", ChangeKind::Modified, Instructions)],
            false
        )),
        None
    );
    let plan = plan_coverage(
        &t,
        10,
        &set(
            vec![change("src/lib.rs", ChangeKind::Modified, Rust)],
            false,
        ),
    );
    assert_eq!(plan.inspect_frames.len(), 1000);
    assert!(plan.complete);
    // Rendering stays bounded even when inspection broadens.
    assert!(plan.rendered_frames.len() <= MAX_RENDERED_FRAMES);

    let f = fixture();
    let (_c, _ctx, captured) = captured_with(&f, |draft| {
        fs::write(draft.join("src/lib.rs"), "// shared function changed").unwrap();
    });
    let revision = captured.candidate().revision().as_str().to_owned();
    let mut probe = Fake::new(&revision, 1000);
    let report = validate_candidate(
        &captured,
        BuildOutcome::Built(build_identity(&captured)),
        0,
        0,
        Some(&mut probe),
    );
    assert!(report.passed());
    let coverage = report.coverage().unwrap();
    assert_eq!(coverage.broadened, Some(BroadenReason::SharedSourceChanged));
    assert_eq!(coverage.inspected_frames, 1000);
    assert_eq!(coverage.inspection_batches, 4);
    assert_eq!(probe.inspect_batches, vec![256, 256, 256, 232]);
    let mut seen = probe.inspected.clone();
    seen.sort_unstable();
    assert_eq!(seen, (0..1000).collect::<Vec<_>>());
}

#[test]
fn videos_longer_than_the_broadened_bound_report_partial_coverage_visibly() {
    use studio_project::revision::FileKind::Rust;
    let t = timeline("r", MAX_BROADENED_FRAMES + 5000, &[], &[]);
    let changes = ChangeSet {
        total: 1,
        entries: vec![change("src/lib.rs", ChangeKind::Modified, Rust)],
        truncated: false,
    };
    let plan = plan_coverage(&t, 0, &changes);
    assert!(!plan.complete);
    assert!(plan.inspect_frames.len() <= MAX_BROADENED_FRAMES + 3);
    assert!(plan.inspect_frames.contains(&0));
    assert!(plan.inspect_frames.contains(&(MAX_BROADENED_FRAMES + 4999)));
}

#[test]
fn broadening_needs_one_existing_non_instruction_file_to_change_and_unknown_kinds_widen() {
    use studio_project::revision::FileKind::{Instructions, Media, Other};
    let broaden = |entries: Vec<ChangedPath>| {
        broadening(&ChangeSet {
            total: entries.len(),
            entries,
            truncated: false,
        })
    };
    // A modified instruction file plus an *added* asset: no existing scene input changed.
    assert_eq!(
        broaden(vec![
            change("AGENTS.md", ChangeKind::Modified, Instructions),
            change("media/new.png", ChangeKind::Added, Media),
        ]),
        None
    );
    assert_eq!(
        broaden(vec![change(
            "AGENTS.md",
            ChangeKind::Modified,
            Instructions
        )]),
        None
    );
    for kind in [ChangeKind::Modified, ChangeKind::Deleted] {
        assert_eq!(
            broaden(vec![change("media/a.png", kind, Media)]),
            Some(BroadenReason::ExistingFilesChanged)
        );
    }
    assert_eq!(
        broaden(vec![change("fonts/x.ttf", ChangeKind::Added, Other)]),
        Some(BroadenReason::OtherFilesChanged)
    );
}

#[test]
fn targeted_coverage_widens_when_scenes_cannot_establish_it() {
    let widened = |t: PreviewTimelineResponse| {
        let plan = plan_coverage(&t, 10, &ChangeSet::default());
        (plan.broadened, plan.inspect_frames.len(), plan.complete)
    };
    // No scenes at all, or scenes with a hole: inspect every frame instead of guessing.
    assert_eq!(
        widened(timeline("r", 1000, &[], &[])),
        (Some(BroadenReason::ScenesUnavailable), 1000, true)
    );
    assert_eq!(
        widened(timeline("r", 1000, &[(0, 400), (500, 1000)], &[])),
        (Some(BroadenReason::ScenesUnavailable), 1000, true)
    );
    assert_eq!(
        widened(timeline("r", 1000, &[(0, 400)], &[])),
        (Some(BroadenReason::ScenesUnavailable), 1000, true)
    );
    // Overlapping scenes that still cover the whole video keep targeted coverage.
    let (broadened, count, _) = widened(timeline("r", 1000, &[(0, 600), (400, 1000)], &[]));
    assert_eq!(broadened, None);
    assert!(count < 1000);
}
