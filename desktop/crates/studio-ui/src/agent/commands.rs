//! Commands the panel sends to the workflow: prompts, Stop, permissions, review, adapter setup and export.

use super::*;

impl ConversationPanel {
    pub(super) fn fill_setup(&mut self, adapter: Option<&AdapterFile>, cx: &mut Context<Self>) {
        let fields = adapter.map(AdapterFile::fields).unwrap_or_default();
        let set = |input: &Entity<TextInputState>, text: String, cx: &mut Context<Self>| {
            input.update(cx, |input, cx| input.set_text(text, cx));
        };
        set(&self.setup.provider, fields.provider, cx);
        set(&self.setup.executable, fields.executable, cx);
        set(&self.setup.args, fields.args, cx);
        set(&self.setup.env_names, fields.env_names, cx);
    }

    pub(super) fn setup_fields(&self, cx: &gpui::App) -> AdapterFields {
        let text = |input: &Entity<TextInputState>| input.read(cx).text().to_owned();
        AdapterFields {
            provider: text(&self.setup.provider),
            executable: text(&self.setup.executable),
            args: text(&self.setup.args),
            env_names: text(&self.setup.env_names),
        }
    }

    // ---- commands ------------------------------------------------------------------------------

    pub(super) fn say(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.notice = Some(message.into());
        cx.notify();
    }

    pub(super) fn command<T>(
        &mut self,
        result: Result<T, studio_engine::agent::WorkflowError>,
        cx: &mut Context<Self>,
    ) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.say(error.to_string(), cx);
                None
            }
        }
    }

    pub fn submit(&mut self, cx: &mut Context<Self>) {
        let (Some(attached), Some(snapshot)) = (&self.attached, &self.snapshot) else {
            return;
        };
        let controls = derive_controls(snapshot, self.prompt_is_empty(cx), self.sdk_ready);
        if let Some(reason) = controls.submit.blocked {
            self.say(reason, cx);
            return;
        }
        if let Some(error) = &self.scope_error {
            self.say(
                format!("The selected timeline scope is unavailable: {error}"),
                cx,
            );
            return;
        }
        let text = self.prompt.read(cx).text().to_owned();
        let workflow = attached.workflow.clone();
        let scope = self.task_scope.clone();
        let sent = match &controls.submit.kind {
            SubmitKind::Clarify(task) => workflow.reply_clarification(task, &text),
            SubmitKind::Brief | SubmitKind::Queue => match scope {
                Some(scope) => workflow.submit_scoped(&text, scope).map(|_| ()),
                None => workflow.submit(&text).map(|_| ()),
            },
        };
        if self.command(sent, cx).is_some() {
            self.notice = None;
            self.prompt.update(cx, |input, cx| input.set_text("", cx));
        }
    }

    pub(super) fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(workflow) = self.workflow().cloned() {
            let result = workflow.stop();
            self.command(result, cx);
        }
    }

    /// Answers the oldest open permission request: `option` is one of the ids it offers,
    /// `None` declines it.
    pub fn answer_oldest_permission(&mut self, option: Option<String>, cx: &mut Context<Self>) {
        let Some(reference) = self.snapshot.as_ref().and_then(|snapshot| {
            snapshot
                .open_permissions()
                .first()
                .map(|card| card.reference.clone())
        }) else {
            self.say("No permission request is open.", cx);
            return;
        };
        let answer = option.map_or(PermissionAnswer::Cancel, PermissionAnswer::Select);
        self.answer_permission(reference, answer, cx);
    }

    /// Switches later tasks between automatic Apply and review before Apply.
    pub fn toggle_review_policy(&mut self, cx: &mut Context<Self>) {
        let (Some(workflow), Some(snapshot)) = (self.workflow().cloned(), &self.snapshot) else {
            return;
        };
        let next = match snapshot.review_policy {
            ReviewPolicy::AutoApply => ReviewPolicy::ManualReview,
            ReviewPolicy::ManualReview => ReviewPolicy::AutoApply,
        };
        let result = workflow.set_review_policy(next);
        self.command(result, cx);
    }

    /// Manual review: publishes the candidate (refused with the reason while Apply is
    /// blocked).
    pub fn apply(&mut self, cx: &mut Context<Self>) {
        if let Some(workflow) = self.workflow().cloned() {
            let result = workflow.apply();
            self.command(result, cx);
        }
    }

    pub(super) fn answer_permission(
        &mut self,
        reference: PermissionRef,
        answer: PermissionAnswer,
        cx: &mut Context<Self>,
    ) {
        let (Some(attached), Some(snapshot)) = (&self.attached, &self.snapshot) else {
            return;
        };
        let workflow = attached.workflow.clone();
        let open = snapshot.open_permissions();
        if let Err(refusal) = self.replies.begin(&reference, &open) {
            self.say(refusal.to_string(), cx);
            return;
        }
        let result = workflow.reply_permission(&reference, answer);
        self.command(result, cx);
        cx.notify();
    }

    /// Saves the setup fields. The settings are resolved and the file is written on
    /// background threads; at most one
    /// save is in flight (a second click is refused until it completes), the running
    /// workflow is only reconfigured while idle, and only the in-flight save's completion
    /// is reported. The Save button never starts the adapter.
    pub(super) fn save_adapter(&mut self, cx: &mut Context<Self>) {
        self.save_adapter_with(None, cx);
    }

    /// [`Self::save_adapter`]. With `turn_on` (the profile being turned on and the
    /// description its launch was filled into) the save writes that profile, found by its
    /// id, and runs one connection test once it is written.
    pub(super) fn save_adapter_with(
        &mut self,
        turn_on: Option<(String, &AdapterFile)>,
        cx: &mut Context<Self>,
    ) {
        let Some(attached) = &self.attached else {
            return;
        };
        let paths = attached.paths.clone();
        // What the fields do not show (`auth_method`) comes from the profile being turned
        // on, never from the adapter configured before it.
        let base = match &turn_on {
            Some((_, file)) => Some(*file),
            None => self.saved_adapter.as_ref(),
        };
        let built = AdapterFile::from_fields(base, &self.setup_fields(cx), self.mcp_choice);
        let file = match built {
            Ok(file) => file,
            Err(error) => {
                self.say(error, cx);
                return;
            }
        };
        if let Some(snapshot) = &self.snapshot
            && !derive_controls(snapshot, self.prompt_is_empty(cx), self.sdk_ready)
                .settings_editable
        {
            self.say(
                "Settings can change after the running task finishes or is stopped.",
                cx,
            );
            return;
        }
        let ticket = match self.saves.begin() {
            Ok(ticket) => ticket,
            Err(refusal) => {
                self.say(refusal, cx);
                return;
            }
        };
        // The profile a turn-on writes is fixed now: the row state below may be reset while
        // the save is in flight, the save target may not.
        let target = turn_on.map(|(profile, _)| profile);
        self.agent_setup.turn_on = match &target {
            Some(profile) => provider_setup::TurnOn::Saving {
                ticket,
                profile: profile.clone(),
            },
            // A plain Save replaces whatever adapter a turn-on configured.
            None => provider_setup::TurnOn::Idle,
        };
        let policy = self.policy.clone();
        let resolving = {
            let file = file.clone();
            cx.background_executor()
                .spawn(async move { file.map(|file| file.resolve(&policy)) })
        };
        cx.spawn(async move |this, cx| {
            let resolved = resolving.await;
            let _ = this.update(cx, |panel, cx| {
                panel.apply_saved(ticket, target, file, resolved, paths, cx)
            });
        })
        .detach();
        self.say("Saving the adapter settings…", cx);
    }

    /// Second half of a save, back on the UI thread: reconfigure the workflow, then persist.
    pub(super) fn apply_saved(
        &mut self,
        ticket: u64,
        target: Option<String>,
        file: Option<AdapterFile>,
        settings: Option<studio_engine::agent::AdapterSettings>,
        paths: AppPaths,
        cx: &mut Context<Self>,
    ) {
        if !self.saves.is_current(ticket) {
            return;
        }
        let Some(attached) = &self.attached else {
            self.saves.finish(ticket);
            return;
        };
        let applied = attached.workflow.set_adapter(settings);
        if self.command(applied, cx).is_none() {
            self.saves.finish(ticket);
            self.agent_setup.turn_on.finish(ticket, false);
            return;
        }
        self.saved_adapter = file.clone();
        self.settings_error = None;
        // A turn-on saves its profile by id; a plain Save goes through the label-matching
        // settings path.
        let (written, profile) = (file.clone(), target.clone());
        let task = cx.background_executor().spawn(async move {
            match (&written, &profile) {
                (Some(file), Some(profile)) => host::save_profile_settings(&paths, profile, file),
                (Some(file), None) => host::save_settings(&paths, file),
                (None, _) => host::clear_settings(&paths),
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |panel, cx| {
                if !panel.saves.finish(ticket) {
                    return;
                }
                let turn_on = panel.agent_setup.turn_on.finish(ticket, result.is_ok());
                if result.is_ok()
                    && let (Some(profile), Some(file)) = (&target, file)
                {
                    // The registry on disk now selects this profile with this launch; the
                    // copy the Providers section shows and later selections start from
                    // follows it.
                    if let Err(error) = panel.registry.save_profile_adapter(profile, file) {
                        panel.say(error, cx);
                        return;
                    }
                }
                panel.notice = Some(match result {
                    Ok(()) if turn_on => {
                        panel.test_turned_on("Adapter settings saved on this computer.", cx)
                    }
                    Ok(()) => {
                        "Adapter settings saved on this computer. Use Check adapter to verify them."
                            .into()
                    }
                    Err(error) => error,
                });
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn export(&mut self, kind: Export, cx: &mut Context<Self>) {
        let Some(workflow) = self.workflow().cloned() else {
            return;
        };
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let picker = cx.prompt_for_new_path(
            &home,
            Some(match kind {
                Export::Candidate => "Candidate video",
                Export::Draft => "Retained draft",
            }),
        );
        cx.spawn(async move |this, cx| {
            let chosen = picker.await;
            let _ = this.update(cx, |panel, cx| match chosen {
                Ok(Ok(Some(path))) => {
                    let sent = match kind {
                        Export::Candidate => workflow.export_candidate(&path),
                        Export::Draft => workflow.export_draft(&path),
                    };
                    if panel.command(sent, cx).is_some() {
                        panel.say(format!("Exporting to {}…", path.display()), cx);
                    }
                }
                Ok(Ok(None)) => (),
                other => panel.say(
                    format!("Native picker failed: {other:?}; check the desktop portal/file chooser and retry"),
                    cx,
                ),
            });
        })
        .detach();
    }
}
