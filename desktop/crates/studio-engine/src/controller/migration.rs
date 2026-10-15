//! Studio-originated project migration and its undo (see [`crate::project_migration`]).
//!
//! Like a preset mutation it runs under `&mut Controller` and publishes through the
//! durable file-set transaction, as a distinct [`TransactionKind::Migration`] record that
//! is never task history. `studio.json` stays out of the file set (a manifest that is
//! briefly absent would make the project unopenable): it is replaced afterwards by an
//! atomic write of the exact bytes the record keeps, and repaired on the next open when
//! that write did not happen.
use super::{Controller, preset::write_changes};
use crate::{
    EngineError, StateError,
    edit_transaction::{
        ApplyGate, Boundary, FileDelta, PlanError, PromotionError, RecordSeed, TaskRevisionRecord,
        TransactionKind, invert, mode_conflicts, plan as plan_transaction, prompt_summary,
        touched_conflicts,
    },
    project_migration::{self, MIGRATE_TO_EDIT_TIMING, ManifestSwap},
};
use std::{fs, io::Read, path::Path};
use studio_project::{Manifest, SourceRevision};
use studio_sdk::CompatibilityManifest;

const MANIFEST: &str = "studio.json";

/// Whether the open project can be migrated to the SDK's current framework.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectMigration {
    /// The project already uses the current framework.
    Current,
    /// The project can be migrated now.
    Available,
    /// The project needs a migration that cannot run now, with the reason.
    Blocked(String),
}

/// Whether Studio may change the project's timing (`timeline.json`) or export it with
/// settings other than the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimingEditing {
    Enabled,
    /// Disabled with the reason to show next to the controls.
    Disabled(String),
}

/// What a migration or its undo did.
#[derive(Debug, Clone)]
pub struct MigrationOutcome {
    pub record: Box<TaskRevisionRecord>,
    /// The source revision after the change, `studio.json` included.
    pub source: SourceRevision,
    /// What is left for the user to do, and non-fatal observations.
    pub notes: Vec<String>,
}

impl Controller {
    /// Timing editing needs a migrated project: the legacy worker cannot play timing
    /// documents or export with settings.
    pub fn timing_editing(&self) -> TimingEditing {
        if project_migration::needs_migration(&self.project.manifest) {
            TimingEditing::Disabled(MIGRATE_TO_EDIT_TIMING.into())
        } else {
            TimingEditing::Enabled
        }
    }

    /// Whether the project can be migrated to the current framework of the SDK described
    /// by `compatibility`, with the reason when it cannot.
    pub fn migration_status(&mut self, compatibility: &CompatibilityManifest) -> ProjectMigration {
        if !project_migration::needs_migration(&self.project.manifest) {
            return ProjectMigration::Current;
        }
        if let Some(reason) = self.source_mutation_blocker("migrating the project") {
            return ProjectMigration::Blocked(reason);
        }
        match project_migration::plan(
            &self.project.root,
            &self.project.inventory,
            &self.project.manifest,
            compatibility,
        ) {
            Ok(_) => ProjectMigration::Available,
            Err(reason) => ProjectMigration::Blocked(reason),
        }
    }

    /// The migration that [`Controller::undo_migration`] would reverse: the last one, while
    /// it was not undone yet.
    pub fn undoable_migration(&self) -> Option<&TaskRevisionRecord> {
        self.migrations
            .last()
            .filter(|record| record.undoes.is_none())
    }

    /// Migrates the project to the current framework of the SDK described by
    /// `compatibility` as one source-fenced durable revision. `expected` is the source
    /// revision the caller's view was built from.
    pub fn migrate_project(
        &mut self,
        expected: &SourceRevision,
        compatibility: &CompatibilityManifest,
    ) -> Result<MigrationOutcome, EngineError> {
        self.prepare_source_mutation(expected, "migrating the project")?;
        let current = self.project.inventory.clone();
        let plan = project_migration::plan(
            &self.project.root,
            &current,
            &self.project.manifest,
            compatibility,
        )
        .map_err(PromotionError::GateBlocked)?;
        let before = self.read_manifest_bytes()?;
        let after = format!("{}\n", serde_json::to_string_pretty(&plan.manifest)?);
        Manifest::parse(after.as_bytes())?;
        let base = self.capture_current(&current.revision)?;
        let draft = self.checkpoints.draft(
            &base,
            &format!("migration-{}", uuid::Uuid::new_v4().simple()),
        )?;
        let formed = write_changes(&draft, &plan.changes)
            .map_err(EngineError::from)
            .and_then(|()| self.checkpoints.capture(&draft).map_err(EngineError::from));
        let _ = fs::remove_dir_all(&draft);
        let candidate = self.checkpoints.load(&formed?)?;
        let seed = self.migration_seed(
            &current.revision,
            "Migrate the project to the current framework",
            None,
            Vec::new(),
            ManifestSwap { before, after },
        );
        let intent = match plan_transaction(
            &self.project.root,
            &current,
            &candidate,
            seed,
            self.mechanism_override,
        ) {
            Ok(intent) => intent,
            Err(error) => return Err(PromotionError::from(error).into()),
        };
        let record = self.run_intent(&intent)?;
        self.migration_after_commit(record, plan.notes)
    }

    /// Restores every file the last migration changed, `studio.json` included, to its
    /// bytes before the migration. Refuses when one of them changed since.
    pub fn undo_migration(
        &mut self,
        expected: &SourceRevision,
    ) -> Result<MigrationOutcome, EngineError> {
        self.prepare_source_mutation(expected, "undoing the migration")?;
        let target = self
            .undoable_migration()
            .cloned()
            .ok_or_else(|| PromotionError::UndoUnavailable("no migration to undo".into()))?;
        let swap = target.migration.clone().ok_or_else(|| {
            PromotionError::UndoUnavailable("the migration record has no manifest".into())
        })?;
        if self.read_manifest_bytes()? != swap.after {
            return Err(PromotionError::UndoUnavailable(format!(
                "{MANIFEST} changed since the migration"
            ))
            .into());
        }
        let current = self.project.inventory.clone();
        let mut conflicts = touched_conflicts(&target.changes, &current);
        for found in mode_conflicts(&self.project.root, &target.changes) {
            if !conflicts.iter().any(|c| c.path == found.path) {
                conflicts.push(found);
            }
        }
        if !conflicts.is_empty() {
            return Err(PromotionError::UndoConflict(conflicts).into());
        }
        let base = self.capture_current(&current.revision)?;
        let draft = self.checkpoints.draft(
            &base,
            &format!("migration-undo-{}", uuid::Uuid::new_v4().simple()),
        )?;
        let formed = self
            .write_inverse(&draft, &target.changes)
            .and_then(|()| self.checkpoints.capture(&draft).map_err(EngineError::from));
        let _ = fs::remove_dir_all(&draft);
        let candidate = self.checkpoints.load(&formed?)?;
        let mut seed = self.migration_seed(
            &current.revision,
            "Undo the project migration",
            Some(target.id.clone()),
            target.changes.clone(),
            ManifestSwap {
                before: swap.after,
                after: swap.before,
            },
        );
        seed.remove_dirs = target.created_dirs.clone();
        let intent = match plan_transaction(
            &self.project.root,
            &current,
            &candidate,
            seed,
            self.mechanism_override,
        ) {
            Ok(intent) => intent,
            Err(PlanError::NoChange) => {
                return Err(PromotionError::UndoUnavailable(
                    "undoing the migration would not change the source".into(),
                )
                .into());
            }
            Err(error) => return Err(PromotionError::from(error).into()),
        };
        let record = self.run_intent(&intent)?;
        self.migration_after_commit(record, Vec::new())
    }

    /// Writes the manifest of the last migration (or undo) whose file set committed while
    /// `studio.json` still holds the bytes it replaced: the app stopped between the two.
    /// Best effort; a failure is a notice and the next open retries.
    pub(super) fn heal_migration_manifest(&mut self) {
        let Some(record) = self.migrations.last().cloned() else {
            return;
        };
        let Some(swap) = &record.migration else {
            return;
        };
        if self.read_manifest_bytes().ok().as_ref() != Some(&swap.before)
            || !touched_conflicts(&record.changes, &self.project.inventory).is_empty()
            || self
                .source_mutation_blocker("repairing the migration")
                .is_some()
        {
            return;
        }
        let mut notes = Vec::new();
        if self.replace_manifest(swap, &mut notes).is_err() && notes.is_empty() {
            notes.push(format!(
                "{MANIFEST} could not be repaired; it is retried on the next open"
            ));
        }
        self.add_recovery_notice(&notes);
    }

    /// Adds `notes` to the notice shown under the header, after what is already there.
    pub fn add_recovery_notice(&mut self, notes: &[String]) {
        if notes.is_empty() {
            return;
        }
        let notes = notes.join(" ");
        self.recovery_notice = Some(match self.recovery_notice.take() {
            Some(existing) => format!("{existing} {notes}"),
            None => notes,
        });
    }

    /// The gates every Studio-originated source change shares, with the reason when one
    /// is closed.
    fn source_mutation_blocker(&mut self, action: &str) -> Option<String> {
        if let Err(error) = self.ensure_mutable() {
            return Some(error.to_string());
        }
        if let ApplyGate::Blocked(reason) = self.apply_gate() {
            return Some(format!("Source publication is blocked: {reason}"));
        }
        if self.agent.active().is_some() {
            return Some(format!(
                "An agent task is running; finish or cancel it before {action}"
            ));
        }
        None
    }

    fn prepare_source_mutation(
        &mut self,
        expected: &SourceRevision,
        action: &str,
    ) -> Result<(), EngineError> {
        self.ensure_mutable()?;
        if let Some(reason) = self.source_mutation_blocker(action) {
            return Err(PromotionError::GateBlocked(reason).into());
        }
        self.reconcile()?;
        let current = &self.project.inventory.revision;
        if current != expected {
            return Err(PromotionError::SourceChanged {
                expected: expected.as_str()[..12].to_owned(),
                found: current.as_str()[..12].to_owned(),
            }
            .into());
        }
        Ok(())
    }

    /// The current source as immutable objects; it must still be `revision`.
    fn capture_current(&self, revision: &SourceRevision) -> Result<SourceRevision, EngineError> {
        let base = self.checkpoints.capture(&self.project.root)?;
        if &base != revision {
            return Err(StateError::StaleResult.into());
        }
        Ok(base)
    }

    fn migration_seed(
        &self,
        base: &SourceRevision,
        summary: &str,
        undoes: Option<String>,
        reverses: Vec<FileDelta>,
        swap: ManifestSwap,
    ) -> RecordSeed {
        RecordSeed {
            kind: TransactionKind::Migration,
            task: "studio-migration".into(),
            generation: 0,
            session: self.record.state.session().clone(),
            task_base: base.clone(),
            prior_history: self.record.state.task_history().head_id(),
            prior_checkpoint: self.record.state.accepted().clone(),
            undoes,
            prompt_summary: prompt_summary(summary),
            build: None,
            validation_report_sha256: String::new(),
            remove_dirs: Vec::new(),
            reverses,
            preset: None,
            migration: Some(swap),
        }
    }

    /// Applies the inverse of `changes` to a draft copy of the current source: deletions
    /// (and the directories they empty) first, then the restored files.
    fn write_inverse(&self, draft: &Path, changes: &[FileDelta]) -> Result<(), EngineError> {
        let inverse = invert(changes);
        for change in inverse.iter().filter(|c| c.after.is_none()) {
            let target = draft.join(&change.path);
            match fs::remove_file(&target) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
            let mut parent = target.parent();
            while let Some(dir) = parent
                && dir != draft
                && dir.starts_with(draft)
                && fs::remove_dir(dir).is_ok()
            {
                parent = dir.parent();
            }
        }
        for change in inverse.iter().filter(|c| c.after.is_some()) {
            let state = change.after.as_ref().expect("filtered");
            self.write_object(&draft.join(&change.path), state)?;
        }
        Ok(())
    }

    /// The file set is durable: the source is rescanned and persisted, then `studio.json`
    /// replaced with the record's bytes.
    fn migration_after_commit(
        &mut self,
        record: TaskRevisionRecord,
        mut notes: Vec<String>,
    ) -> Result<MigrationOutcome, EngineError> {
        self.migrations.push(record.clone());
        if let Err(error) = self.reconcile() {
            notes.push(format!("source rescan after publication failed: {error}"));
        }
        if let Some(fault) = self.controller_boundary(Boundary::Database { after: false })? {
            self.projection_stale = true;
            notes.push(format!("history database not updated: {fault}"));
        } else {
            if let Err(error) = self.persist(self.record.clone()) {
                self.projection_stale = true;
                notes.push(format!("history database not updated: {error}"));
            }
            self.controller_boundary(Boundary::Database { after: true })?;
        }
        let swap = record
            .migration
            .clone()
            .expect("migration records carry the manifest");
        self.replace_manifest(&swap, &mut notes)?;
        Ok(MigrationOutcome {
            record: Box::new(record),
            source: self.record.state.source().clone(),
            notes,
        })
    }

    /// Replaces `studio.json` with `swap.after` when it still holds `swap.before`, by the
    /// project's same-directory atomic replacement. A failed write is a note; the next
    /// open repairs it.
    fn replace_manifest(
        &mut self,
        swap: &ManifestSwap,
        notes: &mut Vec<String>,
    ) -> Result<(), EngineError> {
        if let Some(fault) = self.controller_boundary(Boundary::Manifest { after: false })? {
            notes.push(format!(
                "{MANIFEST} not updated: {fault}; it is repaired on the next open"
            ));
            return Ok(());
        }
        let written = match self.read_manifest_bytes() {
            Ok(bytes) if bytes == swap.before => studio_project::lifecycle::atomic_write(
                &self.project.root.join(MANIFEST),
                swap.after.as_bytes(),
            )
            .map_err(EngineError::from),
            Ok(_) => Err(EngineError::Diagnostic(format!(
                "{MANIFEST} changed during the migration"
            ))),
            Err(error) => Err(error),
        };
        match written {
            Ok(()) => {
                self.controller_boundary(Boundary::Manifest { after: true })?;
                if let Err(error) = self.reconcile() {
                    notes.push(format!(
                        "source rescan after the manifest update failed: {error}"
                    ));
                } else if let Err(error) = self.persist(self.record.clone()) {
                    self.projection_stale = true;
                    notes.push(format!("history database not updated: {error}"));
                }
            }
            Err(error) => notes.push(format!(
                "{MANIFEST} not updated: {error}; it is repaired on the next open"
            )),
        }
        Ok(())
    }

    /// The raw `studio.json`, which a migration restores byte for byte.
    fn read_manifest_bytes(&self) -> Result<String, EngineError> {
        let mut bytes = Vec::new();
        studio_project::ProjectPath::try_from(MANIFEST.to_owned())?
            .open_file(&self.project.root)?
            .take(studio_project::manifest::MAX_MANIFEST_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        String::from_utf8(bytes)
            .map_err(|_| EngineError::Diagnostic(format!("{MANIFEST} is not UTF-8")))
    }
}
