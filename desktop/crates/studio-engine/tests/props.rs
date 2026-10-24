//! Props edits of an open project: field values, gestures, undo and redo, the write of
//! `props.json` and how it meets outside writes, crashes, invalid files and agent tasks.
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use fframes_studio_protocol::{EditorObjectIdentity, PropsDocument};
use serde_json::{Value, json};
use studio_engine::{
    Boundary, Controller, Fault, JobKind, JobResult, JobState, TaskState,
    app_paths::AppPaths,
    build_materialization::sdk_pin,
    props::{PropsCommand, PropsEditing},
    timing::file::WriteStep,
};
use studio_project::revision::PROPS_FILE;
use studio_sdk::CompatibilityManifest;

struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    paths: AppPaths,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let compatibility = CompatibilityManifest::default_linux_x64();
    studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        &compatibility.runtime_version,
    )
    .unwrap();
    Fixture {
        _temp: temp,
        root,
        paths,
    }
}

fn built(controller: &mut Controller) {
    let tag = controller.begin_job(JobKind::Build).unwrap();
    controller
        .complete(&tag, JobResult::Built(tag.base_source.clone()))
        .unwrap();
}

fn opened(f: &Fixture) -> Controller {
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    built(&mut controller);
    assert_eq!(controller.props_snapshot().editing, PropsEditing::Enabled);
    controller
}

fn title() -> EditorObjectIdentity {
    EditorObjectIdentity {
        scene_instance_key: "intro".into(),
        component_key: "Title".into(),
        object_key: "headline".into(),
        repeat_key: "main".into(),
    }
}

fn set(field: &str, value: Option<Value>) -> PropsCommand {
    PropsCommand::Set {
        identity: title(),
        field: field.into(),
        value,
    }
}

fn live(f: &Fixture) -> PropsDocument {
    PropsDocument::parse(&fs::read_to_string(f.root.join(PROPS_FILE)).unwrap()).unwrap()
}

fn live_value(f: &Fixture, field: &str) -> Option<Value> {
    live(f)
        .values(&title())
        .and_then(|values| values.get(field).cloned())
}

fn outside_doc(size: u32) -> String {
    let mut doc = PropsDocument::default();
    doc.set(&title(), "size", Some(json!(size)));
    doc.to_json()
}

fn diagnostic(result: Result<(), studio_engine::EngineError>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn props_edits_write_each_value_and_undo_redo_and_reset_them() {
    let f = fixture();
    let mut c = opened(&f);
    let snapshot = c.props_snapshot();
    assert_eq!(
        snapshot.props_json, None,
        "no document draws the code's values"
    );
    assert_eq!(c.props_for_export(), Ok(None));
    let first = snapshot.generation;

    c.props_command(set("size", Some(json!(48)))).unwrap();
    c.props_command(set("text", Some(json!("Hello")))).unwrap();
    assert_eq!(live_value(&f, "size"), Some(json!(48)));
    assert_eq!(live_value(&f, "text"), Some(json!("Hello")));
    let snapshot = c.props_snapshot();
    assert!(snapshot.generation > first);
    assert!(snapshot.can_undo && !snapshot.can_redo);
    assert_eq!(
        snapshot.props_json.as_deref(),
        Some(
            fs::read_to_string(f.root.join(PROPS_FILE))
                .unwrap()
                .as_str()
        )
    );
    assert_eq!(
        c.props_for_export()
            .unwrap()
            .map(|json| PropsDocument::parse(&json).unwrap()),
        Some(live(&f))
    );

    c.props_command(PropsCommand::Undo).unwrap();
    assert_eq!(live_value(&f, "text"), None);
    assert_eq!(live_value(&f, "size"), Some(json!(48)));
    c.props_command(PropsCommand::Redo).unwrap();
    assert_eq!(live_value(&f, "text"), Some(json!("Hello")));

    // Reset to the code's value drops the field, and the last field drops the object.
    c.props_command(set("text", None)).unwrap();
    c.props_command(set("size", None)).unwrap();
    assert!(live(&f).objects.is_empty());
    assert_eq!(c.props_snapshot().props_json, None);
    assert_eq!(c.props_for_export(), Ok(None));
}

#[test]
fn props_gesture_is_one_undo_step_one_write_and_no_build() {
    let f = fixture();
    let mut c = opened(&f);
    let writes = Arc::new(AtomicUsize::new(0));
    let counted = writes.clone();
    c.set_transaction_hooks(Arc::new(move |boundary: &Boundary| {
        if matches!(boundary, Boundary::PropsWrite(WriteStep::Staged)) {
            counted.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }));
    let job = c.state().job().clone();
    let generation = c.state().generation();
    let build = c.state().build_source().clone();
    let first = c.props_snapshot().generation;

    c.props_command(PropsCommand::BeginGesture).unwrap();
    for step in 0..40 {
        c.props_command(set("size", Some(json!(step)))).unwrap();
    }
    let dragging = c.props_snapshot();
    assert!(dragging.generation > first, "every step previews live");
    assert!(!dragging.can_undo, "nothing to undo while dragging");
    assert!(
        !f.root.join(PROPS_FILE).exists(),
        "nothing written mid-drag"
    );
    let error = diagnostic(c.props_command(PropsCommand::Undo));
    assert!(error.contains("Finish the drag"), "{error}");
    c.props_command(PropsCommand::EndGesture).unwrap();

    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert_eq!(live_value(&f, "size"), Some(json!(39)));
    // The props document is no part of the build: the build stands.
    assert_eq!(c.state().build_source(), &build);
    assert_eq!(c.state().built(), Some(&build));
    assert_eq!(c.state().generation(), generation);
    assert_eq!(c.state().job(), &job, "no build job was queued");
    assert!(matches!(c.state().job(), JobState::Succeeded(_)));
    let inventory = studio_project::open(&f.root).unwrap().inventory;
    assert!(
        inventory
            .files
            .iter()
            .all(|file| !file.path.as_str().starts_with(".fframes")),
        "{:?}",
        inventory.files
    );
    assert_eq!(
        inventory.props_file().map(|file| file.path.as_str()),
        Some(PROPS_FILE)
    );

    c.props_command(PropsCommand::Undo).unwrap();
    assert!(live(&f).objects.is_empty());
    assert!(!c.props_snapshot().can_undo, "the drag was one step");
    assert_eq!(writes.load(Ordering::SeqCst), 2);
}

#[test]
fn props_crash_between_displace_and_publish_is_recovered_on_open() {
    let f = fixture();
    let mut c = opened(&f);
    c.props_command(set("size", Some(json!(10)))).unwrap();
    c.set_transaction_hooks(Arc::new(|boundary: &Boundary| {
        if matches!(boundary, Boundary::PropsWrite(WriteStep::Displaced)) {
            Err(Fault::Crash)
        } else {
            Ok(())
        }
    }));
    assert!(c.props_command(set("size", Some(json!(20)))).is_err());
    assert!(
        !f.root.join(PROPS_FILE).exists(),
        "the crash left no live file"
    );
    drop(c);

    let mut reopened = Controller::open(&f.root, &f.paths).unwrap();
    assert_eq!(
        live_value(&f, "size"),
        Some(json!(20)),
        "the interrupted write was completed"
    );
    assert!(
        reopened
            .recovery_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("interrupted save")),
        "{:?}",
        reopened.recovery_notice
    );
    built(&mut reopened);
    let snapshot = reopened.props_snapshot();
    assert_eq!(snapshot.editing, PropsEditing::Enabled);
    assert_eq!(
        snapshot.doc.and_then(|doc| doc.values(&title()).cloned()),
        live(&f).values(&title()).cloned()
    );
}

#[test]
fn props_outside_change_reloads_and_refuses_the_old_undo() {
    let f = fixture();
    let mut c = opened(&f);
    c.props_command(set("size", Some(json!(48)))).unwrap();
    assert!(c.props_snapshot().can_undo);
    fs::write(f.root.join(PROPS_FILE), outside_doc(7)).unwrap();
    c.reconcile().unwrap();
    let snapshot = c.props_snapshot();
    assert!(!snapshot.can_undo);
    assert!(
        snapshot
            .notice
            .as_deref()
            .is_some_and(|notice| notice.contains("changed outside"))
    );
    assert_eq!(
        snapshot
            .props_json
            .map(|json| PropsDocument::parse(&json).unwrap()),
        Some(PropsDocument::parse(&outside_doc(7)).unwrap()),
        "the session draws the outside version"
    );
    let error = diagnostic(c.props_command(PropsCommand::Undo));
    assert!(error.contains("changed outside"), "{error}");
    assert_eq!(
        fs::read_to_string(f.root.join(PROPS_FILE)).unwrap(),
        outside_doc(7)
    );

    // Seen by a rescan in the middle of a drag: the drag is abandoned, the file stands.
    c.props_command(PropsCommand::BeginGesture).unwrap();
    c.props_command(set("size", Some(json!(1)))).unwrap();
    fs::write(f.root.join(PROPS_FILE), outside_doc(9)).unwrap();
    c.reconcile().unwrap();
    let error = diagnostic(c.props_command(set("size", Some(json!(2)))));
    assert!(error.contains("changed outside"), "{error}");
    let _ = c.props_command(PropsCommand::EndGesture);
    assert_eq!(
        fs::read_to_string(f.root.join(PROPS_FILE)).unwrap(),
        outside_doc(9)
    );
}

#[test]
fn props_invalid_file_disables_editing_and_refuses_the_export() {
    let f = fixture();
    fs::write(f.root.join(PROPS_FILE), "{ not json").unwrap();
    let mut c = Controller::open(&f.root, &f.paths).unwrap();
    built(&mut c);
    let PropsEditing::Disabled(reason) = c.props_snapshot().editing else {
        panic!("an invalid document cannot be edited");
    };
    assert!(reason.contains(PROPS_FILE), "{reason}");
    assert_eq!(c.props_snapshot().props_json, None);
    let error = c.props_for_export().unwrap_err();
    assert!(error.contains(PROPS_FILE), "{error}");
    assert!(c.props_command(set("size", Some(json!(1)))).is_err());
    assert_eq!(
        fs::read_to_string(f.root.join(PROPS_FILE)).unwrap(),
        "{ not json",
        "the file is never overwritten"
    );

    // Fixed outside the studio, it can be edited again.
    fs::write(f.root.join(PROPS_FILE), outside_doc(3)).unwrap();
    c.reconcile().unwrap();
    assert_eq!(c.props_snapshot().editing, PropsEditing::Enabled);
    c.props_command(set("size", Some(json!(4)))).unwrap();
    assert_eq!(live_value(&f, "size"), Some(json!(4)));
}

#[test]
fn props_edits_are_refused_while_an_agent_task_runs() {
    let f = fixture();
    let mut c = opened(&f);
    let context = c.begin_agent_task("make the title bigger").unwrap();
    let PropsEditing::Disabled(reason) = c.props_snapshot().editing else {
        panic!("editing is disabled during a task");
    };
    assert!(reason.contains("agent task"), "{reason}");
    for command in [
        set("size", Some(json!(1))),
        PropsCommand::BeginGesture,
        PropsCommand::Undo,
    ] {
        let error = diagnostic(c.props_command(command));
        assert!(error.contains("agent task"), "{error}");
    }
    assert!(!f.root.join(PROPS_FILE).exists());
    c.finish_agent_task(&context.identity, TaskState::Cancelled, "user stop")
        .unwrap();
    built(&mut c);
    c.props_command(set("size", Some(json!(1)))).unwrap();
}
