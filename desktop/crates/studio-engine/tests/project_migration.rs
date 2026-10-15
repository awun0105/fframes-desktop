//! Migration of a project of the earlier template to the current framework: one durable,
//! source-fenced revision outside task history, undone byte for byte, `studio.json`
//! included, and repaired on open when the app stopped before the manifest write.

#[path = "support/tx_fixture.rs"]
mod tx_fixture;

use std::{fs, path::Path};
use studio_engine::{
    Boundary, Controller, EngineError, ProjectMigration, PromotionError, TimingEditing,
    TransactionKind, app_paths::AppPaths, build_materialization::sdk_pin, journal::TaskJournal,
    project_migration::MIGRATE_TO_EDIT_TIMING,
};
use studio_sdk::CompatibilityManifest;
use tx_fixture::*;

const FIXTURE_FILES: [&str; 5] = [
    "AGENTS.md",
    "Cargo.toml",
    "src/lib.rs",
    "src/main.rs",
    "src/bin/studio_worker.rs",
];

fn compatibility() -> CompatibilityManifest {
    CompatibilityManifest::default_linux_x64()
}

/// A project of the earlier template: the checked-in scaffold, generated with the first
/// instructions, pinned to this SDK.
fn legacy() -> Fx {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let compatibility = compatibility();
    let mut project = studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        compatibility.studio_runtime_version(),
    )
    .unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/tests/fixtures/pre_m4_project");
    fs::remove_file(root.join("src/video.rs")).unwrap();
    for file in FIXTURE_FILES {
        fs::copy(fixture.join(file), root.join(file)).unwrap();
    }
    project.manifest.generated_instruction_version = 1;
    studio_project::lifecycle::write_manifest(&root, &project.manifest).unwrap();
    Fx { temp, root, paths }
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn migrate(c: &mut Controller) -> Result<studio_engine::MigrationOutcome, EngineError> {
    let source = c.state().source().clone();
    c.migrate_project(&source, &compatibility())
}

fn undo(c: &mut Controller) -> Result<studio_engine::MigrationOutcome, EngineError> {
    let source = c.state().source().clone();
    c.undo_migration(&source)
}

#[test]
fn migration_moves_the_project_to_the_current_framework_and_undo_restores_every_byte() {
    let fx = legacy();
    let mut c = open_retrying(&fx.root, &fx.paths).unwrap();
    assert_eq!(
        c.timing_editing(),
        TimingEditing::Disabled(MIGRATE_TO_EDIT_TIMING.into())
    );
    assert_eq!(
        c.migration_status(&compatibility()),
        ProjectMigration::Available
    );
    assert!(c.undoable_migration().is_none());
    let before = without_internal(&snapshot(&fx.root));
    let lib = read(&fx.root, "src/lib.rs");
    let history = c.state().task_history().head_id();
    let accepted = c.state().accepted().clone();

    let outcome = migrate(&mut c).unwrap();
    assert_eq!(outcome.record.kind, TransactionKind::Migration);
    assert!(outcome.notes.is_empty(), "{:?}", outcome.notes);
    assert_eq!(&outcome.source, c.state().source());
    let compatibility = compatibility();
    let cargo = read(&fx.root, "Cargo.toml");
    assert!(
        cargo.contains(&format!("version = \"={}\"", compatibility.fframes_version)),
        "{cargo}"
    );
    assert_eq!(
        cargo
            .matches(&format!("\"={}\"", compatibility.studio_runtime_version()))
            .count(),
        2,
        "{cargo}"
    );
    let worker = read(&fx.root, "src/bin/studio_worker.rs");
    assert!(worker.contains("with_timing_args"), "{worker}");
    assert!(worker.contains("run_export"), "{worker}");
    assert!(read(&fx.root, "src/main.rs").contains(".timing_file(\"timeline.json\")"));
    assert!(read(&fx.root, "AGENTS.md").contains("timeline.json"));
    assert_eq!(
        read(&fx.root, "src/lib.rs"),
        lib,
        "the user's scenes are never touched"
    );
    let manifest = studio_project::open(&fx.root).unwrap().manifest;
    assert_eq!(manifest.generated_instruction_version, 2);
    assert_eq!(manifest.sdk, sdk_pin(&compatibility));
    assert_eq!(c.project.manifest.generated_instruction_version, 2);
    assert_eq!(c.timing_editing(), TimingEditing::Enabled);
    assert_eq!(
        c.migration_status(&compatibility),
        ProjectMigration::Current
    );
    // Neither task history nor the saved checkpoint moves.
    assert_eq!(c.state().task_history().head_id(), history);
    assert_eq!(c.state().accepted(), &accepted);
    let replay = TaskJournal::inspect(&fx.task_journal(&c)).unwrap();
    assert!(replay.committed().is_empty());
    assert_eq!(replay.committed_migrations().len(), 1);
    assert!(c.undoable_migration().is_some());

    let undone = undo(&mut c).unwrap();
    assert_eq!(
        undone.record.undoes.as_deref(),
        Some(outcome.record.id.as_str())
    );
    assert_eq!(
        differences(&before, &without_internal(&snapshot(&fx.root))),
        Vec::<String>::new()
    );
    assert_eq!(
        c.timing_editing(),
        TimingEditing::Disabled(MIGRATE_TO_EDIT_TIMING.into())
    );
    assert!(c.undoable_migration().is_none());
    assert!(matches!(
        undo(&mut c),
        Err(EngineError::Promotion(PromotionError::UndoUnavailable(_)))
    ));
    assert_eq!(
        c.migration_status(&compatibility),
        ProjectMigration::Available
    );

    // Reopening repairs nothing and remembers both records.
    drop(c);
    let c = open_retrying(&fx.root, &fx.paths).unwrap();
    assert_eq!(
        differences(&before, &without_internal(&snapshot(&fx.root))),
        Vec::<String>::new()
    );
    assert!(c.undoable_migration().is_none());
    let replay = TaskJournal::inspect(&fx.task_journal(&c)).unwrap();
    assert_eq!(replay.committed_migrations().len(), 2);
}

#[test]
fn undo_refuses_when_a_migrated_file_or_the_manifest_changed_since() {
    let fx = legacy();
    let mut c = open_retrying(&fx.root, &fx.paths).unwrap();
    migrate(&mut c).unwrap();
    let main = read(&fx.root, "src/main.rs");
    fs::write(fx.root.join("src/main.rs"), format!("{main}// mine\n")).unwrap();
    c.reconcile().unwrap();
    assert!(matches!(
        undo(&mut c),
        Err(EngineError::Promotion(PromotionError::UndoConflict(conflicts)))
            if conflicts.iter().any(|c| c.path.as_str() == "src/main.rs")
    ));
    assert!(read(&fx.root, "src/main.rs").ends_with("// mine\n"));
    fs::write(fx.root.join("src/main.rs"), main).unwrap();

    let manifest = read(&fx.root, "studio.json");
    fs::write(fx.root.join("studio.json"), manifest.replace("\n", "\r\n")).unwrap();
    c.reconcile().unwrap();
    assert!(matches!(
        undo(&mut c),
        Err(EngineError::Promotion(PromotionError::UndoUnavailable(reason)))
            if reason.contains("studio.json")
    ));
}

#[test]
fn a_stale_source_fence_or_an_unrecognised_worker_refuses_before_anything_is_written() {
    let fx = legacy();
    let mut c = open_retrying(&fx.root, &fx.paths).unwrap();
    let stale = c.state().source().clone();
    let lib = read(&fx.root, "src/lib.rs");
    fs::write(fx.root.join("src/lib.rs"), format!("{lib}// edited\n")).unwrap();
    c.reconcile().unwrap();
    let before = without_internal(&snapshot(&fx.root));
    assert!(matches!(
        c.migrate_project(&stale, &compatibility()),
        Err(EngineError::Promotion(PromotionError::SourceChanged { .. }))
    ));
    assert_eq!(
        differences(&before, &without_internal(&snapshot(&fx.root))),
        Vec::<String>::new()
    );

    fs::write(fx.root.join("src/bin/studio_worker.rs"), "fn main() {}\n").unwrap();
    c.reconcile().unwrap();
    let ProjectMigration::Blocked(reason) = c.migration_status(&compatibility()) else {
        panic!("a hand-written worker must block the migration");
    };
    assert!(reason.contains("not a generated Studio worker"), "{reason}");
    let before = without_internal(&snapshot(&fx.root));
    assert!(migrate(&mut c).is_err());
    assert_eq!(
        differences(&before, &without_internal(&snapshot(&fx.root))),
        Vec::<String>::new()
    );
}

#[test]
fn a_migration_interrupted_before_the_manifest_write_is_completed_on_open() {
    let recording = Tape::recording();
    {
        let fx = legacy();
        let mut c = open_retrying(&fx.root, &fx.paths).unwrap();
        c.set_transaction_hooks(recording.hooks());
        migrate(&mut c).unwrap();
    }
    let at = recording
        .boundaries()
        .iter()
        .position(|b| *b == Boundary::Manifest { after: false })
        .expect("the manifest write is a durable boundary");

    let fx = legacy();
    let original = read(&fx.root, "studio.json");
    let mut c = open_retrying(&fx.root, &fx.paths).unwrap();
    c.set_transaction_hooks(Tape::crashing_at(at).hooks());
    assert!(is_crash(&migrate(&mut c).unwrap_err()));
    drop(c);
    assert_eq!(read(&fx.root, "studio.json"), original);
    assert!(read(&fx.root, "src/main.rs").contains(".timing_file("));

    let mut c = open_retrying(&fx.root, &fx.paths).unwrap();
    let manifest = studio_project::open(&fx.root).unwrap().manifest;
    assert_eq!(manifest.generated_instruction_version, 2);
    assert_eq!(c.timing_editing(), TimingEditing::Enabled);
    assert!(c.undoable_migration().is_some());
    undo(&mut c).unwrap();
    assert_eq!(read(&fx.root, "studio.json"), original);
}
