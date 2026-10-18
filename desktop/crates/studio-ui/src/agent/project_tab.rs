//! The Project tab: review policy, Apply, Undo, accepted edits and recovery.

use super::*;

impl ConversationPanel {
    pub(super) fn project_tab(
        &mut self,
        snapshot: Arc<WorkflowSnapshot>,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut column = div()
            .id("agent-project-tab")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(12.));

        // Review policy: app-local, affects subsequent tasks only.
        let policy = snapshot.review_policy;
        let next = match policy {
            ReviewPolicy::AutoApply => ReviewPolicy::ManualReview,
            ReviewPolicy::ManualReview => ReviewPolicy::AutoApply,
        };
        let mut review = self.section("Review")
            .child(label_text(match policy {
                ReviewPolicy::AutoApply => "Validated changes are applied automatically.",
                ReviewPolicy::ManualReview => "Validated changes wait for your Apply or Discard.",
            }))
            .child(self.line(WorkflowSnapshot::REVIEW_POLICY_NOTICE))
            .child(self.line(
                "A change affects tasks started afterwards; a running task keeps the policy it started with.",
            ));
        if let Some(task) = snapshot.task.as_ref().filter(|t| !t.phase.is_terminal()) {
            review = review.child(self.line(format!(
                "The running task uses: {}",
                match task.review_policy {
                    ReviewPolicy::AutoApply => "automatic Apply",
                    ReviewPolicy::ManualReview => "manual review",
                }
            )));
        }
        review = review.child(field_row(
            &self.theme,
            "Later tasks",
            self.action_button(
                "agent-policy",
                match next {
                    ReviewPolicy::AutoApply => "Apply automatically (later tasks)",
                    ReviewPolicy::ManualReview => "Review before Apply (later tasks)",
                },
                true,
                Tone::Normal,
                cx,
                |this, _, cx| this.toggle_review_policy(cx),
            ),
        ));
        column = column.child(review);

        // Undo and the accepted-awaiting-preview state.
        let mut undo = self.section("Undo");
        match &controls.undo {
            UndoControl::Enabled { summary, .. } => {
                undo = undo
                    .child(self.line(format!("Newest agent edit: {summary}")))
                    .child(field_row(
                        &self.theme,
                        "Undo",
                        self.action_button(
                            "agent-undo",
                            "Undo last agent edit",
                            true,
                            Tone::Normal,
                            cx,
                            |this, _, cx| {
                                if let Some(w) = this.workflow().cloned() {
                                    let r = w.undo(None);
                                    this.command(r, cx);
                                }
                            },
                        ),
                    ));
            }
            UndoControl::Disabled(reason) => {
                undo = undo.child(self.line(reason.clone())).child(field_row(
                    &self.theme,
                    "Undo",
                    self.action_button(
                        "agent-undo",
                        "Undo last agent edit",
                        false,
                        Tone::Normal,
                        cx,
                        |_, _, _| (),
                    ),
                ));
            }
        }
        if let Some(handoff) = &snapshot.handoff {
            undo = undo.child(label_text(match &handoff.state {
                HandoffState::AwaitingPreview { reason } => format!(
                    "Accepted {} is awaiting its preview{}. The preview still shows the previous revision.",
                    &handoff.published[..handoff.published.len().min(12)],
                    reason.as_ref().map(|r| format!(" ({r})")).unwrap_or_default()
                ),
                HandoffState::Adopted => format!(
                    "Accepted {} · the preview is switching to it.",
                    &handoff.published[..handoff.published.len().min(12)]
                ),
                HandoffState::Displayed => format!(
                    "Accepted {} · displayed in the preview.",
                    &handoff.published[..handoff.published.len().min(12)]
                ),
            }));
        }
        column = column.child(undo);

        // Recovery.
        let recovery = &snapshot.recovery;
        if !recovery.rolled_back.is_empty()
            || !recovery.unresolved_conflicts.is_empty()
            || recovery.suspended.is_some()
            || recovery.draft.as_ref().is_some_and(draft_needs_attention)
            || recovery.notice.is_some()
            || controls.export_draft
        {
            let mut section = self.section("Recovery");
            if let Some(reason) = &recovery.suspended {
                section = section.child(label_text(format!(
                    "Source changes are suspended until the project is reopened: {reason}"
                )));
            }
            if let Some(notice) = &recovery.notice {
                section = section.child(self.line(notice.clone()));
            }
            for id in recovery.rolled_back.iter().take(5) {
                section = section.child(self.line(format!(
                    "An unfinished edit ({}) was rolled back when the project opened; your files are as they were.",
                    &id[..id.len().min(8)]
                )));
            }
            for conflict in &recovery.unresolved_conflicts {
                section = section.child(self.conflict_block(conflict));
            }
            if let Some(draft) = recovery.draft.as_ref().filter(|d| draft_needs_attention(d)) {
                section = section.child(label_text(match &draft.state {
                    _ if draft.interrupted_by_restart => {
                        "A task was interrupted when the app last closed. Its working copy was kept and is locked until you confirm nothing is writing there.".to_owned()
                    }
                    DraftState::UnsafeWriter { reason, .. } => format!(
                        "The agent's working copy is locked because a writer may still run: {reason}"
                    ),
                    DraftState::Retained { reason, .. } => format!(
                        "The last task's working copy was kept ({reason}). It is archived before the next task starts."
                    ),
                    _ => "The agent's working copy is retained.".to_owned(),
                }));
                section = section.child(self.line(draft.draft.display().to_string()));
                if controls.acknowledge_writer_gone {
                    section = section
                        .child(self.line(
                            "Confirm only if you are sure no agent process is still writing there.",
                        ))
                        .child(field_row(
                            &self.theme,
                            "Writer",
                            self.action_button(
                                "agent-ack-writer",
                                "Nothing is writing; unlock the working copy",
                                true,
                                Tone::Normal,
                                cx,
                                |this, _, cx| {
                                    if let Some(w) = this.workflow().cloned() {
                                        let r = w.acknowledge_writer_gone();
                                        this.command(r, cx);
                                    }
                                },
                            ),
                        ));
                }
            }
            if controls.export_draft {
                section = section.child(self.action_button(
                    "agent-export-draft",
                    "Export the retained draft…",
                    true,
                    Tone::Normal,
                    cx,
                    |this, _, cx| this.export(Export::Draft, cx),
                ));
            }
            column = column.child(section);
        }

        // History.
        let mut history = self.section(&format!(
            "Accepted agent edits ({})",
            snapshot.history.len()
        ));
        if snapshot.history.is_empty() {
            history =
                history.child(self.line("No agent edit has been accepted in this project yet."));
        }
        for entry in snapshot.history.iter().rev().take(10) {
            let mut line = history_line(&self.theme, entry);
            if entry.kind == "Apply" {
                line = line.child(self.revision_review_button(&entry.id, cx));
            }
            history = history.child(line);
        }
        column = column.child(history);

        // Resources.
        let r = snapshot.resources;
        column = column.child(self.line(format!(
            "Resident messages {}/{} · {} KiB of {} KiB · {} processes · {} tool grants · {} tool workers",
            r.resident_rows,
            r.max_resident_rows,
            r.resident_bytes / 1024,
            r.max_resident_bytes / 1024,
            r.owned_processes,
            r.broker_grants,
            r.tool_workers
        )));
        column.into_any_element()
    }
}
