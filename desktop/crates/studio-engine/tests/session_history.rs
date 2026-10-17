//! Multi-session storage: listing, loading, and refusing what an older Studio left.

use std::fs;
use std::path::{Path, PathBuf};

use studio_engine::agent::log::{RowLimits, RowStore};
use studio_engine::agent::model::{
    NoticeLevel, OutcomeCard, OutcomeKind, Row, RowId, RowKind, UserSource,
};
use studio_engine::agent::session_store::{
    ManifestStatus, SESSION_MANIFEST_SCHEMA_VERSION, SessionManifest, SessionManifestState,
    SessionStore,
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

fn manifest_path(f: &Fixture, id: &str) -> PathBuf {
    f.paths
        .agent_session_dir(&f.project, id)
        .join("manifest.json")
}

/// A project with one current session: three rows and a manifest of the current version.
fn current_project() -> (Fixture, String) {
    let f = fixture();
    let id = SessionStore::create_session_at(&f.paths, &f.project, None, 1_000).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &id).unwrap();
    write_log(
        &SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap(),
        &[
            user(1, 1_000, "Make the intro faster"),
            agent(2, 1_010, "Done"),
            outcome(3, 1_020, OutcomeKind::Accepted),
        ],
    );
    let current = manifest(SESSION_MANIFEST_SCHEMA_VERSION, "claude", 1_000);
    SessionStore::save(&f.paths, &f.project, &current).unwrap();
    (f, id)
}

/// Replaces the stored manifest of the current session with one of another schema version and
/// returns its path and exact bytes.
fn foreign_manifest(f: &Fixture, id: &str, version: u32) -> (PathBuf, Vec<u8>) {
    let path = manifest_path(f, id);
    let bytes = serde_json::to_vec_pretty(&manifest(version, "claude", 1_000)).unwrap();
    fs::write(&path, &bytes).unwrap();
    (path, bytes)
}

fn ids(f: &Fixture) -> Vec<String> {
    SessionStore::list_sessions(&f.paths, &f.project)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect()
}

#[test]
fn a_manifest_of_the_current_version_is_saved_loaded_and_cleared_in_the_current_session() {
    let (f, id) = current_project();
    let loaded = SessionStore::load(&f.paths, &f.project)
        .unwrap()
        .expect("the manifest was saved");
    assert_eq!(loaded.schema_version, SESSION_MANIFEST_SCHEMA_VERSION);
    assert_eq!(loaded.provider_id, "claude");
    assert!(loaded.is_resumable());
    assert!(manifest_path(&f, &id).is_file());

    SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 2_000)).unwrap();
    assert_eq!(
        SessionStore::load(&f.paths, &f.project)
            .unwrap()
            .unwrap()
            .provider_id,
        "codex"
    );
    let session = SessionStore::load_session(&f.paths, &f.project, &id, 100).unwrap();
    assert_eq!(session.rows.len(), 3);
    assert!(session.manifest.is_some());
}

#[test]
fn a_project_without_a_session_has_nothing_to_load_and_nowhere_to_save() {
    let f = fixture();
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
    assert!(SessionStore::current_session_id(&f.paths, &f.project).is_none());
    assert!(SessionStore::current_log_path(&f.paths, &f.project).is_none());
    assert!(SessionStore::save(&f.paths, &f.project, &manifest(2, "claude", 1)).is_err());
    SessionStore::clear(&f.paths, &f.project).unwrap();
    assert!(!f.paths.agent_sessions(&f.project).exists());
}

#[test]
fn files_of_the_single_session_layout_are_ignored_and_left_as_they_are() {
    let f = fixture();
    let old_manifest = f.paths.agent(&f.project).join("session-manifest.json");
    let old_log = f.paths.project(&f.project).join("conversation.jsonl");
    let old_archive = f.paths.project(&f.project).join("conversation-42.jsonl");
    fs::create_dir_all(old_manifest.parent().unwrap()).unwrap();
    fs::write(
        &old_manifest,
        serde_json::to_vec_pretty(&manifest(1, "claude", 1_000)).unwrap(),
    )
    .unwrap();
    write_log(&old_log, &[user(1, 1_000, "Make the intro faster")]);
    write_log(&old_archive, &[user(1, 500, "Older chat about colours")]);
    let files = [&old_manifest, &old_log, &old_archive];
    let before: Vec<Vec<u8>> = files.iter().map(|p| fs::read(p).unwrap()).collect();

    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
    assert!(ids(&f).is_empty());
    let startup = SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert!(startup.log.starts_with(f.paths.agent_sessions(&f.project)));
    assert_eq!(ids(&f).len(), 1, "only the first session of this layout");
    let current = SessionStore::load_session(
        &f.paths,
        &f.project,
        &SessionStore::current_session_id(&f.paths, &f.project).unwrap(),
        100,
    )
    .unwrap();
    assert!(current.rows.is_empty());
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());

    let after: Vec<Vec<u8>> = files.iter().map(|p| fs::read(p).unwrap()).collect();
    assert_eq!(before, after);
}

#[test]
fn a_manifest_of_an_older_format_is_refused_and_left_as_it_is() {
    let (f, id) = current_project();
    let (path, bytes) = foreign_manifest(&f, &id, 1);

    let error = SessionStore::load(&f.paths, &f.project).unwrap_err();
    assert!(error.contains("older Studio"), "{error}");
    assert!(error.contains(&path.display().to_string()), "{error}");
    let error = SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 2_000)).unwrap_err();
    assert!(error.contains("older Studio"), "{error}");
    assert!(error.contains("manifest.json"), "{error}");
    let error = SessionStore::clear(&f.paths, &f.project).unwrap_err();
    assert!(error.contains("older Studio"), "{error}");
    assert!(error.contains("manifest.json"), "{error}");
    assert_eq!(fs::read(&path).unwrap(), bytes);
    match SessionStore::manifest_status(&f.paths, &f.project, &id) {
        ManifestStatus::Refused(reason) => assert!(reason.contains("older Studio"), "{reason}"),
        _ => panic!("an older manifest is refused"),
    }

    // The session itself still lists and opens; its manifest is simply not used.
    assert_eq!(ids(&f), vec![id.clone()]);
    let session = SessionStore::load_session(&f.paths, &f.project, &id, 100).unwrap();
    assert_eq!(session.rows.len(), 3);
    assert!(session.manifest.is_none());
    assert!(SessionStore::session_manifest(&f.paths, &f.project, &id).is_none());
    assert!(!SessionStore::list_sessions(&f.paths, &f.project).unwrap()[0].resumable);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn a_manifest_of_a_newer_format_is_refused_and_left_as_it_is() {
    let (f, id) = current_project();
    let (path, bytes) = foreign_manifest(&f, &id, 99);

    let error = SessionStore::load(&f.paths, &f.project).unwrap_err();
    assert!(error.contains("newer Studio"), "{error}");
    assert!(error.contains("manifest.json"), "{error}");
    assert!(SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 2_000)).is_err());
    assert!(SessionStore::clear(&f.paths, &f.project).is_err());
    assert!(SessionStore::session_manifest(&f.paths, &f.project, &id).is_none());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn a_manifest_that_is_not_json_is_not_an_old_format_and_can_be_replaced_or_cleared() {
    let (f, id) = current_project();
    let path = manifest_path(&f, &id);
    fs::write(&path, "{ not json").unwrap();
    assert!(SessionStore::load(&f.paths, &f.project).is_err());
    SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 2_000)).unwrap();
    assert_eq!(
        SessionStore::load(&f.paths, &f.project)
            .unwrap()
            .unwrap()
            .provider_id,
        "codex"
    );
    fs::write(&path, "{ not json").unwrap();
    SessionStore::clear(&f.paths, &f.project).unwrap();
    assert!(!path.exists());
}

#[test]
fn valid_json_of_another_shape_is_refused_not_replaced() {
    let (f, id) = current_project();
    let path = manifest_path(&f, &id);
    let bytes = br#"{"schema_version": 2, "unexpected": [1, 2]}"#.to_vec();
    fs::write(&path, &bytes).unwrap();

    let error = SessionStore::load(&f.paths, &f.project).unwrap_err();
    assert!(error.contains("not in the format"), "{error}");
    assert!(error.contains("manifest.json"), "{error}");
    assert!(SessionStore::save(&f.paths, &f.project, &manifest(2, "codex", 2_000)).is_err());
    assert!(SessionStore::clear(&f.paths, &f.project).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(matches!(
        SessionStore::manifest_status(&f.paths, &f.project, &id),
        ManifestStatus::Refused(_)
    ));
}

#[test]
fn manifest_status_tells_missing_usable_and_refused_apart() {
    let (f, id) = current_project();
    assert!(matches!(
        SessionStore::manifest_status(&f.paths, &f.project, &id),
        ManifestStatus::Ok
    ));
    assert!(matches!(
        SessionStore::manifest_status(&f.paths, &f.project, "no-such-session"),
        ManifestStatus::Missing
    ));
    assert!(matches!(
        SessionStore::manifest_status(&f.paths, &f.project, "../escape"),
        ManifestStatus::Missing
    ));
    fs::remove_file(manifest_path(&f, &id)).unwrap();
    assert!(matches!(
        SessionStore::manifest_status(&f.paths, &f.project, &id),
        ManifestStatus::Missing
    ));
}

#[test]
fn a_refused_manifest_stays_byte_identical_through_prepare_pointer_repair_and_new_sessions() {
    for version in [1, 99] {
        let (f, id) = current_project();
        let (path, bytes) = foreign_manifest(&f, &id, version);

        SessionStore::prepare(&f.paths, &f.project).unwrap();
        break_pointer(&f, "dangling");
        assert!(SessionStore::current_log_path(&f.paths, &f.project).is_some());
        break_pointer(&f, "missing");
        SessionStore::prepare(&f.paths, &f.project).unwrap();
        // A new session is renamed into place, the one with the refused manifest is not touched.
        let other = SessionStore::create_session_at(&f.paths, &f.project, None, 2_000).unwrap();
        SessionStore::set_current(&f.paths, &f.project, &other).unwrap();
        SessionStore::set_current(&f.paths, &f.project, &id).unwrap();
        SessionStore::record_last_row_id(&f.paths, &f.project, &id, 500).unwrap();

        assert_eq!(fs::read(&path).unwrap(), bytes, "version {version}");
    }
}

#[test]
fn a_meta_with_an_unknown_field_is_never_rewritten_and_its_session_still_opens() {
    let (f, id) = current_project();
    let meta_path = f.paths.agent_session_dir(&f.project, &id).join("meta.json");
    let original = fs::read_to_string(&meta_path).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&original).unwrap();
    value["source_hash"] = serde_json::json!("abc123");
    let foreign = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(&meta_path, &foreign).unwrap();

    // What a session switch records, and a second session created next to it.
    SessionStore::record_last_row_id(&f.paths, &f.project, &id, 900).unwrap();
    let other = SessionStore::create_session_at(&f.paths, &f.project, None, 2_000).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &other).unwrap();
    SessionStore::record_last_row_id(&f.paths, &f.project, &id, 1_000).unwrap();
    assert_eq!(fs::read(&meta_path).unwrap(), foreign);

    assert!(ids(&f).contains(&id));
    let session = SessionStore::load_session(&f.paths, &f.project, &id, 100).unwrap();
    assert_eq!(session.rows.len(), 3);
}

#[test]
fn a_meta_of_another_schema_version_is_not_used_and_not_rewritten() {
    let (f, id) = current_project();
    let meta_path = f.paths.agent_session_dir(&f.project, &id).join("meta.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
    value["schema_version"] = serde_json::json!(7);
    let foreign = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(&meta_path, &foreign).unwrap();

    SessionStore::record_last_row_id(&f.paths, &f.project, &id, 900).unwrap();
    assert_eq!(fs::read(&meta_path).unwrap(), foreign);
    assert!(ids(&f).contains(&id));
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
fn a_broken_current_pointer_is_repaired_to_the_most_recent_session() {
    for how in ["missing", "garbage", "dangling"] {
        let (f, id) = current_project();
        SessionStore::save(&f.paths, &f.project, &manifest(2, "claude", 5_000)).unwrap();
        break_pointer(&f, how);

        assert_eq!(
            SessionStore::current_log_path(&f.paths, &f.project),
            Some(
                f.paths
                    .agent_session_dir(&f.project, &id)
                    .join("conversation.jsonl")
            ),
            "{how}"
        );
        assert_eq!(
            fs::read_to_string(f.paths.agent_session_current(&f.project)).unwrap(),
            id,
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
        assert_eq!(
            SessionStore::load(&f.paths, &f.project)
                .unwrap()
                .unwrap()
                .provider_id,
            "codex",
            "{how}"
        );
        assert_eq!(ids(&f).len(), 1, "{how}");
    }
}

#[test]
fn a_project_with_sessions_but_no_pointer_gets_its_pointer_repaired() {
    let f = fixture();
    let id = SessionStore::create_session(&f.paths, &f.project, None).unwrap();
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(id.as_str())
    );
    assert_eq!(
        SessionStore::current_log_path(&f.paths, &f.project),
        Some(SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap())
    );
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
fn clear_removes_only_the_current_sessions_manifest() {
    let (f, id) = current_project();
    let dir = f.paths.agent_session_dir(&f.project, &id);
    SessionStore::clear(&f.paths, &f.project).unwrap();
    assert!(!dir.join("manifest.json").exists());
    assert!(dir.join("conversation.jsonl").is_file());
    assert!(dir.join("meta.json").is_file());
    assert!(SessionStore::load(&f.paths, &f.project).unwrap().is_none());
    assert_eq!(ids(&f).len(), 1);
}

#[test]
fn row_ids_continue_across_sessions() {
    let f = fixture();
    let a = SessionStore::create_session_at(&f.paths, &f.project, None, 1).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &a).unwrap();
    assert_eq!(SessionStore::row_id_floor(&f.paths, &f.project, &a), 1);
    let rows: Vec<Row> = (1..=5).map(|i| agent(i, i, "a")).collect();
    write_log(
        &SessionStore::current_log_path(&f.paths, &f.project).unwrap(),
        &rows,
    );

    let b = SessionStore::create_session_at(&f.paths, &f.project, None, 2).unwrap();
    assert_eq!(SessionStore::row_id_floor(&f.paths, &f.project, &b), 6);
    SessionStore::set_current(&f.paths, &f.project, &b).unwrap();
    let path = SessionStore::current_log_path(&f.paths, &f.project).unwrap();
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
fn a_listing_reflects_every_change_since_the_previous_listing() {
    let f = fixture();
    let first = SessionStore::create_session_at(&f.paths, &f.project, None, 100).unwrap();
    let log = SessionStore::session_log_path(&f.paths, &f.project, &first).unwrap();
    write_log(&log, &[user(1, 100, "Make the intro faster")]);
    let before = SessionStore::list_sessions(&f.paths, &f.project).unwrap();
    assert_eq!(before.len(), 1);
    assert!(before[0].is_current);

    write_log(
        &log,
        &[
            user(1, 100, "Make the intro faster"),
            outcome(2, 500, OutcomeKind::Accepted),
        ],
    );
    let second =
        SessionStore::create_session_at(&f.paths, &f.project, Some("Second".into()), 200).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &second).unwrap();
    let after = SessionStore::list_sessions(&f.paths, &f.project).unwrap();
    let order: Vec<_> = after.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(order, [first.as_str(), second.as_str()]);
    assert_eq!(after[0].rows, 2);
    assert_eq!(after[0].updated_unix, 500);
    assert_eq!(after[0].outcome, Some(OutcomeKind::Accepted));
    assert!(!after[0].is_current);
    assert!(after[1].is_current);
    assert_eq!(after[1].title, "Second");

    fs::remove_dir_all(f.paths.agent_session_dir(&f.project, &second)).unwrap();
    assert_eq!(ids(&f), [first]);
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
    let (f, current) = current_project();
    let id = SessionStore::create_session(&f.paths, &f.project, Some("Next".into())).unwrap();
    assert_eq!(
        SessionStore::current_session_id(&f.paths, &f.project).as_deref(),
        Some(current.as_str())
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

#[test]
fn preparing_a_fresh_project_creates_its_first_session_once() {
    let f = fixture();
    let first = SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert_eq!(first.row_floor, 1);
    let current = SessionStore::current_session_id(&f.paths, &f.project).unwrap();
    assert_eq!(
        Some(first.log.clone()),
        SessionStore::current_log_path(&f.paths, &f.project)
    );
    assert!(first.log.starts_with(f.paths.agent_sessions(&f.project)));

    let again = SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert_eq!(again.log, first.log);
    assert_eq!(ids(&f), vec![current]);
}

#[test]
fn preparing_a_project_with_a_session_serves_it_and_creates_nothing() {
    let (f, id) = current_project();
    let startup = SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert_eq!(
        startup.log,
        SessionStore::session_log_path(&f.paths, &f.project, &id).unwrap()
    );
    assert!(startup.log.is_file());
    assert_eq!(ids(&f), vec![id]);
}

#[test]
fn preparing_removes_interrupted_staging_directories_and_keeps_the_sessions() {
    let (f, id) = current_project();
    let staging = f.paths.agent_sessions(&f.project).join(".staging-half");
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("conversation.jsonl"), "half a copy").unwrap();
    SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert!(!staging.exists());
    assert_eq!(ids(&f), vec![id]);
}

#[test]
fn preparing_leaves_a_manifest_of_an_older_format_as_it_is() {
    let (f, id) = current_project();
    let (path, bytes) = foreign_manifest(&f, &id, 1);
    let startup = SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert!(startup.log.is_file());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(ids(&f), vec![id]);
}
#[test]
fn row_ids_stay_unique_when_a_session_that_used_higher_ids_is_not_current_after_a_restart() {
    let f = fixture();
    let a = SessionStore::create_session_at(&f.paths, &f.project, None, 1).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &a).unwrap();
    let rows: Vec<Row> = (1..=5).map(|i| agent(i, i, "a")).collect();
    write_log(
        &SessionStore::current_log_path(&f.paths, &f.project).unwrap(),
        &rows,
    );
    SessionStore::record_last_row_id(&f.paths, &f.project, &a, 5).unwrap();
    // Recording is monotonic.
    SessionStore::record_last_row_id(&f.paths, &f.project, &a, 3).unwrap();
    assert_eq!(SessionStore::project_row_floor(&f.paths, &f.project), 6);

    // A second session uses ids past its first one and is left; the first becomes current
    // again and the project restarts: the floor still counts the second session's ids.
    let b = SessionStore::create_session_from(&f.paths, &f.project, None, 2, 6).unwrap();
    let rows: Vec<Row> = (6..=10).map(|i| agent(i, i, "b")).collect();
    write_log(
        &SessionStore::session_log_path(&f.paths, &f.project, &b).unwrap(),
        &rows,
    );
    SessionStore::record_last_row_id(&f.paths, &f.project, &b, 10).unwrap();
    SessionStore::set_current(&f.paths, &f.project, &a).unwrap();
    let startup = SessionStore::prepare(&f.paths, &f.project).unwrap();
    assert_eq!(startup.row_floor, 11);
    let mut store =
        RowStore::open_with_floor(startup.log, RowLimits::default(), startup.row_floor).unwrap();
    assert_eq!(store.next_id(), RowId(11));
}

#[test]
fn a_project_whose_sessions_cannot_be_written_fails_to_prepare_and_changes_nothing() {
    let f = fixture();
    let old_log = f.paths.project(&f.project).join("conversation.jsonl");
    write_log(&old_log, &[user(1, 1_000, "Make the intro faster")]);
    let before = fs::read(&old_log).unwrap();
    // A file where the sessions directory belongs: no session can be created.
    fs::create_dir_all(f.paths.agent(&f.project)).unwrap();
    fs::write(f.paths.agent_sessions(&f.project), b"in the way").unwrap();
    let error = SessionStore::prepare(&f.paths, &f.project).unwrap_err();
    assert!(error.contains("first session"), "{error}");
    assert_eq!(fs::read(&old_log).unwrap(), before);
    assert_eq!(
        fs::read(f.paths.agent_sessions(&f.project)).unwrap(),
        b"in the way"
    );
}
