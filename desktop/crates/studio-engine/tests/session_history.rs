//! Multi-session storage: migration of the single-session layout, listing and loading.

use std::fs;
use std::path::Path;

use studio_engine::agent::log::{RowLimits, RowStore};
use studio_engine::agent::model::{
    NoticeLevel, OutcomeCard, OutcomeKind, Row, RowId, RowKind, UserSource,
};
use studio_engine::agent::session_store::layout::LEGACY_SESSION_ID;
use studio_engine::agent::session_store::{
    SESSION_MANIFEST_SCHEMA_VERSION, SessionManifest, SessionManifestState, SessionStore,
};
use studio_engine::app_paths::AppPaths;
use studio_project::ProjectId;

struct Fixture {
    _dir: tempfile::TempDir,
    paths: AppPaths,
    project: ProjectId,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path().join("data")).unwrap();
    let project = ProjectId::try_from("project-1".to_owned()).unwrap();
    fs::create_dir_all(paths.project(&project)).unwrap();
    Fixture {
        _dir: dir,
        paths,
        project,
    }
}

fn row(id: u64, at: u64, kind: RowKind) -> Row {
    Row {
        id: RowId(id),
        task: None,
        at_unix: at,
        kind,
    }
}

fn user(id: u64, at: u64, text: &str) -> Row {
    row(
        id,
        at,
        RowKind::User {
            text: text.into(),
            source: UserSource::Brief,
        },
    )
}

fn agent(id: u64, at: u64, text: &str) -> Row {
    row(
        id,
        at,
        RowKind::Agent {
            text: text.into(),
            turn: 1,
            streaming: false,
        },
    )
}

fn outcome(id: u64, at: u64, kind: OutcomeKind) -> Row {
    row(
        id,
        at,
        RowKind::Outcome(OutcomeCard {
            kind,
            summary: "done".into(),
            published: None,
            draft_retained: false,
        }),
    )
}

fn write_log(path: &Path, rows: &[Row]) {
    let mut text = String::new();
    for row in rows {
        text.push_str(&serde_json::json!({"v": 1, "row": row}).to_string());
        text.push('\n');
    }
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn manifest(version: u32, provider: &str, at: u64) -> SessionManifest {
    SessionManifest {
        schema_version: version,
        provider_id: provider.into(),
        wire_session_id: Some("wire-session-0123456789".into()),
        project_id: "project-1".into(),
        draft_path: "draft".into(),
        launch_digest: "digest".into(),
        last_source_revision: "r1".into(),
        last_accepted_revision: "r1".into(),
        context_schema: "m6-context/1".into(),
        capability_snapshot: None,
        transcript_boundary: 3,
        state: SessionManifestState::Ready,
        created_at: at.to_string(),
        updated_at: at.to_string(),
    }
}

fn write_legacy_manifest(f: &Fixture, manifest: &SessionManifest) {
    let path = f.paths.agent_session_manifest(&f.project);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(manifest).unwrap()).unwrap();
}

/// A project as the single-session builds leave it: a manifest, a log and one archived log.
fn legacy_project() -> Fixture {
    let f = fixture();
    write_legacy_manifest(&f, &manifest(1, "claude", 1_000));
    write_log(
        &f.paths.legacy_conversation_log(&f.project),
        &[
            user(1, 1_000, "Make the intro faster"),
            agent(2, 1_010, "Done"),
            outcome(3, 1_020, OutcomeKind::Accepted),
        ],
    );
    write_log(
        &f.paths.project(&f.project).join("conversation-42.jsonl"),
        &[user(1, 500, "Older chat about colours")],
    );
    f
}

fn ids(f: &Fixture) -> Vec<String> {
    SessionStore::list_sessions(&f.paths, &f.project)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect()
}

#[test]
fn the_single_session_api_is_untouched_before_migration() {
    let f = fixture();
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
    SessionStore::save(&f.paths, &f.project, &manifest(2, "claude", 1)).unwrap();
    assert!(f.paths.agent_session_manifest(&f.project).is_file());
    assert!(!f.paths.agent_sessions(&f.project).exists());
    assert_eq!(
        SessionStore::current_log_path(&f.paths, &f.project),
        f.paths.legacy_conversation_log(&f.project)
    );
    SessionStore::clear(&f.paths, &f.project).unwrap();
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
}

#[test]
fn a_single_session_project_migrates_with_its_rows_and_manifest() {
    let f = legacy_project();
    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert_eq!(report.migrated, vec![LEGACY_SESSION_ID, "archive-42"]);
    assert_eq!(report.removed_legacy.len(), 3);
    assert!(report.kept_legacy.is_empty());

    assert!(!f.paths.legacy_conversation_log(&f.project).exists());
    assert!(!f.paths.agent_session_manifest(&f.project).exists());
    assert!(
        !f.paths
            .project(&f.project)
            .join("conversation-42.jsonl")
            .exists()
    );
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(LEGACY_SESSION_ID)
    );

    // The single-session API now serves the migrated session.
    let loaded = SessionStore::load(&f.paths, &f.project)
        .unwrap()
        .expect("manifest survives");
    assert_eq!(loaded.schema_version, SESSION_MANIFEST_SCHEMA_VERSION);
    assert_eq!(loaded.provider_id, "claude");
    assert!(loaded.is_resumable());
    assert_eq!(
        SessionStore::current_log_path(&f.paths, &f.project),
        f.paths
            .agent_session_dir(&f.project, LEGACY_SESSION_ID)
            .join("conversation.jsonl")
    );
    let session = SessionStore::load_session(&f.paths, &f.project, LEGACY_SESSION_ID, 100).unwrap();
    assert_eq!(session.rows.len(), 3);
    assert!(session.manifest.is_some());

    let archive = SessionStore::load_session(&f.paths, &f.project, "archive-42", 100).unwrap();
    assert_eq!(archive.rows.len(), 1);
    assert!(archive.manifest.is_none());

    // Saving after migration writes into the session, not the legacy path.
    SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 2_000)).unwrap();
    assert!(!f.paths.agent_session_manifest(&f.project).exists());
    assert_eq!(
        SessionStore::load(&f.paths, &f.project)
            .unwrap()
            .unwrap()
            .provider_id,
        "codex"
    );
}

#[test]
fn migrating_twice_changes_nothing() {
    let f = legacy_project();
    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    let before = ids(&f);
    let again = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert!(again.migrated.is_empty() && again.removed_legacy.is_empty());
    assert_eq!(ids(&f), before);
    assert_eq!(before.len(), 2);
}

#[test]
fn a_project_with_nothing_stored_migrates_to_nothing() {
    let f = fixture();
    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert_eq!(report, Default::default());
    assert!(SessionStore::current_session_id(&f.paths, &f.project).is_none());
    assert!(ids(&f).is_empty());
}

#[test]
fn an_interrupted_staging_directory_is_discarded_and_the_run_completes() {
    let f = legacy_project();
    let staging = f.paths.agent_sessions(&f.project).join(".staging-legacy");
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("conversation.jsonl"), "half a copy").unwrap();

    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert!(!staging.exists());
    let session = SessionStore::load_session(&f.paths, &f.project, LEGACY_SESSION_ID, 100).unwrap();
    assert_eq!(session.rows.len(), 3);
}

#[test]
fn a_run_interrupted_after_publishing_is_finished_by_the_rerun() {
    let f = legacy_project();
    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();

    // Put the state of a crash between publishing and cleanup back: the pointer is not
    // written yet and the legacy files are still there, unchanged.
    fs::remove_file(f.paths.agent_session_current(&f.project)).unwrap();
    let dir = f.paths.agent_session_dir(&f.project, LEGACY_SESSION_ID);
    fs::copy(
        dir.join("conversation.jsonl"),
        f.paths.legacy_conversation_log(&f.project),
    )
    .unwrap();
    write_legacy_manifest(&f, &manifest(1, "claude", 1_000));

    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert!(report.migrated.is_empty());
    assert_eq!(report.removed_legacy.len(), 2);
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(LEGACY_SESSION_ID)
    );
    assert!(!f.paths.legacy_conversation_log(&f.project).exists());
    assert_eq!(ids(&f).len(), 2);
}

#[test]
fn a_legacy_file_that_differs_from_its_copy_is_imported_as_its_own_session() {
    let f = legacy_project();
    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    write_log(
        &f.paths.legacy_conversation_log(&f.project),
        &[user(1, 3_000, "written by an older build after migration")],
    );
    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert!(report.migrated.is_empty() && report.kept_legacy.is_empty());
    assert_eq!(report.imported.len(), 1);
    assert!(report.imported[0].starts_with("legacy-"));
    assert_eq!(
        report.removed_legacy,
        vec![f.paths.legacy_conversation_log(&f.project)]
    );
    assert!(!f.paths.legacy_conversation_log(&f.project).exists());

    let imported =
        SessionStore::load_session(&f.paths, &f.project, &report.imported[0], 100).unwrap();
    assert_eq!(imported.rows.len(), 1);
    // The earlier copy is untouched and still current.
    let original =
        SessionStore::load_session(&f.paths, &f.project, LEGACY_SESSION_ID, 100).unwrap();
    assert_eq!(original.rows.len(), 3);
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(LEGACY_SESSION_ID)
    );
    assert_eq!(ids(&f).len(), 3);
}

#[test]
fn a_corrupt_manifest_is_copied_verbatim_so_load_answers_the_same() {
    let f = legacy_project();
    let path = f.paths.agent_session_manifest(&f.project);
    fs::write(&path, "{ not json").unwrap();
    let before = SessionStore::load(&f.paths, &f.project).unwrap_err();

    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert!(report.kept_legacy.is_empty());
    assert!(!path.exists());
    assert_eq!(
        SessionStore::load(&f.paths, &f.project).unwrap_err(),
        before
    );
    let copy = f
        .paths
        .agent_session_dir(&f.project, LEGACY_SESSION_ID)
        .join("manifest.json");
    assert_eq!(fs::read_to_string(copy).unwrap(), "{ not json");
    let session = SessionStore::load_session(&f.paths, &f.project, LEGACY_SESSION_ID, 100).unwrap();
    assert_eq!(session.rows.len(), 3);
    assert!(session.manifest.is_none());
}

#[test]
fn a_manifest_from_a_newer_schema_stays_unusable_after_migration() {
    let f = legacy_project();
    write_legacy_manifest(&f, &manifest(99, "claude", 1_000));
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert!(!f.paths.agent_session_manifest(&f.project).exists());
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
}

fn break_pointer(f: &Fixture, how: &str) {
    let pointer = f.paths.agent_session_current(&f.project);
    match how {
        "missing" => fs::remove_file(&pointer).unwrap(),
        "garbage" => fs::write(&pointer, "../../etc").unwrap(),
        _ => fs::write(&pointer, "ghost-session").unwrap(),
    }
}

#[test]
fn a_broken_current_pointer_is_repaired_and_never_falls_back_to_legacy_files() {
    for how in ["missing", "garbage", "dangling"] {
        let f = legacy_project();
        SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
        // Make the legacy session the most recently updated one.
        SessionStore::save(&f.paths, &f.project, &manifest(2, "claude", 5_000)).unwrap();
        break_pointer(&f, how);

        let log_path = SessionStore::current_log_path(&f.paths, &f.project);
        assert_eq!(
            log_path,
            f.paths
                .agent_session_dir(&f.project, LEGACY_SESSION_ID)
                .join("conversation.jsonl"),
            "{how}"
        );
        assert_eq!(
            fs::read_to_string(f.paths.agent_session_current(&f.project)).unwrap(),
            LEGACY_SESSION_ID,
            "{how}: the pointer is rewritten"
        );

        break_pointer(&f, how);
        assert_eq!(
            SessionStore::load(&f.paths, &f.project)
                .unwrap()
                .unwrap()
                .provider_id,
            "claude",
            "{how}"
        );
        break_pointer(&f, how);
        SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 6_000)).unwrap();
        assert!(
            !f.paths.agent_session_manifest(&f.project).exists(),
            "{how}"
        );
        assert_eq!(
            SessionStore::load(&f.paths, &f.project)
                .unwrap()
                .unwrap()
                .provider_id,
            "codex",
            "{how}"
        );

        break_pointer(&f, how);
        let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
        assert_eq!(report, Default::default(), "{how}");
        assert_eq!(
            SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
            Some(LEGACY_SESSION_ID),
            "{how}"
        );
    }
}

#[test]
fn a_project_with_sessions_but_no_pointer_is_not_treated_as_unmigrated() {
    let f = fixture();
    let id = SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(id.as_str())
    );
    assert_eq!(
        SessionStore::current_log_path(&f.paths, &f.project),
        SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap()
    );
}

#[test]
fn one_item_that_cannot_be_copied_does_not_stop_the_others() {
    let f = legacy_project();
    // A directory where an archived log should be: copying it fails.
    let blocked = f.paths.project(&f.project).join("conversation-7.jsonl");
    fs::create_dir_all(&blocked).unwrap();

    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert_eq!(report.migrated, vec![LEGACY_SESSION_ID, "archive-42"]);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].0, "archive-7");
    assert!(blocked.is_dir());
    assert!(
        !f.paths
            .agent_sessions(&f.project)
            .join(".staging-archive-7")
            .exists()
    );
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(LEGACY_SESSION_ID)
    );
    assert!(!f.paths.legacy_conversation_log(&f.project).exists());
}

#[test]
fn an_archive_with_a_very_long_stamp_gets_a_valid_unique_id() {
    let f = fixture();
    let one = "7".repeat(80);
    let two = format!("{}8", "7".repeat(79));
    for stamp in [&one, &two] {
        write_log(
            &f.paths
                .project(&f.project)
                .join(format!("conversation-{stamp}.jsonl")),
            &[user(1, 5, "long")],
        );
    }
    let report = SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    assert_eq!(report.migrated.len(), 2);
    assert!(report.failed.is_empty());
    assert!(report.migrated.iter().all(|id| id.len() <= 64));
    assert_eq!(ids(&f).len(), 2);
    for id in &report.migrated {
        assert_eq!(
            SessionStore::load_session(&f.paths, &f.project, id, 10)
                .unwrap()
                .rows
                .len(),
            1
        );
    }
}

#[test]
fn an_invalid_session_id_never_resolves_inside_a_real_session_directory() {
    let f = fixture();
    SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    for bad in ["../x", "a/b", "", &"a".repeat(65), "."] {
        let dir = f.paths.agent_session_dir(&f.project, bad);
        assert_eq!(dir, f.paths.agent_sessions(&f.project).join(".invalid"));
        assert!(!dir.exists());
    }
}

#[test]
fn start_new_refuses_a_log_inside_a_session_directory() {
    let f = fixture();
    let id = SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    let path = SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap();
    write_log(&path, &[user(1, 5, "keep me")]);
    let mut store = RowStore::open(path.clone(), RowLimits::default()).unwrap();
    let error = store.start_new(99).unwrap_err().to_string();
    assert!(error.contains("session"), "{error}");
    assert!(path.is_file());
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 2);
}

#[test]
fn clear_removes_only_the_current_sessions_manifest() {
    let f = legacy_project();
    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    let dir = f.paths.agent_session_dir(&f.project, LEGACY_SESSION_ID);
    SessionStore::clear(&f.paths, &f.project).unwrap();
    assert!(!dir.join("manifest.json").exists());
    assert!(dir.join("conversation.jsonl").is_file());
    assert!(dir.join("meta.json").is_file());
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
    assert_eq!(ids(&f).len(), 2);
}

#[test]
fn row_ids_continue_across_sessions() {
    let f = fixture();
    let a = SessionStore::create_session_at(&f.paths, &f.project, None, 1).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &a).unwrap();
    assert_eq!(SessionStore::row_id_floor(&f.paths, &f.project, &a), 1);
    let rows: Vec<Row> = (1..=5).map(|i| agent(i, i, "a")).collect();
    write_log(&SessionStore::current_log_path(&f.paths, &f.project), &rows);

    let b = SessionStore::create_session_at(&f.paths, &f.project, None, 2).unwrap();
    assert_eq!(SessionStore::row_id_floor(&f.paths, &f.project, &b), 6);
    SessionStore::set_current(&f.paths, &f.project, &b).unwrap();
    let path = SessionStore::current_log_path(&f.paths, &f.project);
    let floor = SessionStore::row_id_floor(&f.paths, &f.project, &b);
    let mut store = RowStore::open_with_floor(path.clone(), RowLimits::default(), floor).unwrap();
    assert_eq!(store.next_id(), RowId(6));
    drop(store);
    let rows: Vec<Row> = (6..=8).map(|i| agent(i, i, "b")).collect();
    write_log(&path, &rows);

    let c = SessionStore::create_session_at(&f.paths, &f.project, None, 3).unwrap();
    let floor = SessionStore::row_id_floor(&f.paths, &f.project, &c);
    assert_eq!(floor, 9);
    let log_c = SessionStore::session_log_path(&f.paths, &f.project, &c).unwrap();
    let mut store = RowStore::open_with_floor(log_c, RowLimits::default(), floor).unwrap();
    assert_eq!(store.next_id(), RowId(9));
    // A log that already holds higher ids is never rewound by a lower floor.
    let mut again = RowStore::open_with_floor(path, RowLimits::default(), 2).unwrap();
    assert_eq!(again.next_id(), RowId(9));
}

#[test]
fn a_session_whose_log_cannot_be_read_is_marked_and_does_not_disturb_the_order() {
    let f = fixture();
    let ok = SessionStore::create_session_at(&f.paths, &f.project, None, 100).unwrap();
    write_log(
        &SessionStore::session_log_path(&f.paths, &f.project, &ok).unwrap(),
        &[user(1, 100, "readable")],
    );
    let bad = SessionStore::create_session_at(&f.paths, &f.project, None, 200).unwrap();
    // A directory where the log should be.
    fs::create_dir_all(SessionStore::session_log_path(&f.paths, &f.project, &bad).unwrap())
        .unwrap();
    let newer =
        SessionStore::create_session_at(&f.paths, &f.project, Some("Newer".into()), 300).unwrap();

    let list = SessionStore::list_sessions(&f.paths, &f.project).unwrap();
    let order: Vec<_> = list.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(order, [newer.as_str(), bad.as_str(), ok.as_str()]);
    let summary = list.iter().find(|s| s.id == bad).unwrap();
    assert!(summary.unreadable);
    assert_eq!(summary.title, "Unreadable session");
    assert!(list.iter().filter(|s| s.unreadable).count() == 1);
    assert_eq!(list.iter().find(|s| s.id == ok).unwrap().title, "readable");
}
#[test]
fn a_version_one_manifest_is_still_resumable_before_migration() {
    let f = fixture();
    write_legacy_manifest(&f, &manifest(1, "claude", 1_000));
    let loaded = SessionStore::load(&f.paths, &f.project).unwrap().unwrap();
    assert!(loaded.is_resumable());
}

#[test]
fn sessions_list_newest_first_with_title_agent_and_outcome() {
    let f = fixture();
    let mk = |title: Option<&str>, rows: &[Row], provider: Option<(&str, u64)>| {
        let created = rows.first().map_or(1, |r| r.at_unix);
        let id =
            SessionStore::create_session_at(&f.paths, &f.project, title.map(Into::into), created)
                .unwrap();
        write_log(
            &SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap(),
            rows,
        );
        if let Some((provider, at)) = provider {
            let current = SessionStore::current_session_id(&f.paths, &f.project);
            SessionStore::set_current(&f.paths, &f.project, &id).unwrap();
            SessionStore::save(&f.paths, &f.project, &manifest(2, provider, at)).unwrap();
            if let Some(current) = current {
                SessionStore::set_current(&f.paths, &f.project, &current).unwrap();
            }
        }
        id
    };
    let old = mk(
        None,
        &[
            user(1, 100, "\n  Add a lower third\nwith a logo"),
            outcome(2, 110, OutcomeKind::Failed),
        ],
        None,
    );
    let named = mk(
        Some("Colour pass"),
        &[
            user(1, 200, "tweak"),
            outcome(2, 210, OutcomeKind::Accepted),
        ],
        Some(("codex", 300)),
    );
    let newest = mk(None, &[user(1, 400, "Fix the outro audio")], None);
    let empty = SessionStore::create_session_at(&f.paths, &f.project, None, 1).unwrap();

    let list = SessionStore::list_sessions(&f.paths, &f.project).unwrap();
    let order: Vec<_> = list.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(order[..3], [newest.as_str(), named.as_str(), old.as_str()]);
    assert_eq!(order[3], empty);

    let by_id = |id: &str| list.iter().find(|s| s.id == id).unwrap();
    // Title: stored, else the first non-empty line of the first prompt, else a placeholder.
    assert_eq!(by_id(&named).title, "Colour pass");
    assert_eq!(by_id(&old).title, "Add a lower third");
    assert_eq!(by_id(&newest).title, "Fix the outro audio");
    assert_eq!(by_id(&empty).title, "Untitled session");
    assert_eq!(
        by_id(&old).prompt_text,
        "\n  Add a lower third\nwith a logo"
    );
    // Agent and outcome.
    assert_eq!(by_id(&named).agent.as_deref(), Some("codex"));
    assert_eq!(by_id(&named).outcome, Some(OutcomeKind::Accepted));
    assert!(by_id(&named).resumable);
    assert_eq!(by_id(&old).agent, None);
    assert_eq!(by_id(&old).outcome, Some(OutcomeKind::Failed));
    // Time: the manifest update (300) and the newest row both count.
    assert_eq!(by_id(&named).updated_unix, 300);
    assert_eq!(by_id(&old).updated_unix, 110);
    assert_eq!(by_id(&old).created_unix, 100);
}

#[test]
fn a_long_first_prompt_is_cut_for_the_title_and_repair_prompts_are_ignored() {
    let f = fixture();
    let id = SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    let long = "word ".repeat(40);
    write_log(
        &SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap(),
        &[
            row(
                1,
                10,
                RowKind::User {
                    text: "automatic repair context".into(),
                    source: UserSource::Repair,
                },
            ),
            user(2, 11, &long),
            row(
                3,
                12,
                RowKind::Notice {
                    level: NoticeLevel::Info,
                    text: "ignored".into(),
                },
            ),
        ],
    );
    let list = SessionStore::list_sessions(&f.paths, &f.project).unwrap();
    assert_eq!(list[0].title.chars().count(), 80);
    assert!(list[0].title.ends_with('\u{2026}'));
    assert!(!list[0].prompt_text.contains("repair"));
}

#[test]
fn creating_a_session_does_not_move_the_current_one() {
    let f = legacy_project();
    SessionStore::migrate_legacy(&f.paths, &f.project).unwrap();
    let id = SessionStore::create_session(&f.paths, &f.project, Some("Next".into())).unwrap();
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(LEGACY_SESSION_ID)
    );
    let other = SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    assert_ne!(id, other);
    SessionStore::set_current(&f.paths, &f.project, &id).unwrap();
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(id.as_str())
    );
    assert!(SessionStore::set_current(&f.paths, &f.project, "missing").is_err());
}

#[test]
fn load_returns_the_newest_rows_and_reports_older_ones() {
    let f = fixture();
    let id = SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    let rows: Vec<Row> = (1..=10).map(|i| agent(i, i, &format!("row {i}"))).collect();
    let mut with_update = rows.clone();
    // A later version of row 10 replaces the earlier one.
    with_update.push(agent(10, 11, "row 10 final"));
    write_log(
        &SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap(),
        &with_update,
    );

    let all = SessionStore::load_session(&f.paths, &f.project, &id, 100).unwrap();
    assert_eq!(all.rows.len(), 10);
    assert!(!all.has_older);
    assert_eq!(
        all.rows.iter().map(|r| r.id.0).collect::<Vec<_>>(),
        (1..=10).collect::<Vec<_>>()
    );
    assert_eq!(*all.rows[9], agent(10, 11, "row 10 final"));

    let tail = SessionStore::load_session(&f.paths, &f.project, &id, 4).unwrap();
    assert_eq!(
        tail.rows.iter().map(|r| r.id.0).collect::<Vec<_>>(),
        vec![7, 8, 9, 10]
    );
    assert!(tail.has_older);
}

#[test]
fn loading_and_listing_never_create_anything() {
    let f = fixture();
    assert!(
        SessionStore::list_sessions(&f.paths, &f.project)
            .unwrap()
            .is_empty()
    );
    assert!(SessionStore::load_session(&f.paths, &f.project, "nope", 10).is_err());
    assert!(SessionStore::load_session(&f.paths, &f.project, "../x", 10).is_err());
    assert!(SessionStore::load_session(&f.paths, &f.project, "", 10).is_err());
    assert!(SessionStore::session_log_path(&f.paths, &f.project, "a/b").is_err());
    assert!(!f.paths.agent_sessions(&f.project).exists());
}
