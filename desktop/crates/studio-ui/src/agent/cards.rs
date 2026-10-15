//! Transcript rows and their cards: tool calls, permissions, errors, validation, changes, banners and conflicts.

use super::*;

impl ConversationPanel {
    pub(super) fn row_id(row: &Row) -> ElementId {
        ElementId::from(("agent-row", row.id.0))
    }

    pub(super) fn row_element(&mut self, row: &Arc<Row>, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        match &row.kind {
            RowKind::User { text, source } => Self::row_shell()
                .child(
                    Card::editor_nested(&self.theme)
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .p(px(10.))
                        .child(self.caption(match source {
                            UserSource::Brief => "You",
                            UserSource::Clarification => "You (answer)",
                            UserSource::Repair => "Automatic repair context (sent by Studio)",
                        }))
                        .child(
                            text_style(div(), TextStyle::Body)
                                .text_color(palette.text_1)
                                .child(text.clone()),
                        ),
                )
                .into_any_element(),
            RowKind::Agent {
                text, streaming, ..
            } => Self::row_shell()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .px(px(2.))
                        .child(self.caption(if *streaming {
                            "Agent · writing…"
                        } else {
                            "Agent"
                        }))
                        .child(
                            text_style(div(), TextStyle::Body)
                                .text_color(palette.text_1)
                                .child(text.clone()),
                        ),
                )
                .into_any_element(),
            RowKind::Thought {
                text, streaming, ..
            } => {
                let id = row.id.0;
                let open = self.thoughts_open.contains(&id);
                let entity = cx.entity();
                Self::row_shell()
                    .child(
                        CollapsibleSection::new(("agent-thought", id), open)
                            // A fixed header height: the badge appears and goes without
                            // changing the row's measured height.
                            .header(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .h(px(THOUGHT_HEADER_H))
                                    .child(
                                        text_style(div(), TextStyle::Small)
                                            .italic()
                                            .text_color(palette.text_2)
                                            .child("Reasoning"),
                                    )
                                    .children(
                                        streaming
                                            .then(|| Badge::new("Thinking…").tone(UiTone::Info)),
                                    ),
                            )
                            .content(
                                text_style(div(), TextStyle::Small)
                                    .italic()
                                    .pt(px(4.))
                                    .text_color(palette.text_2)
                                    .child(text.clone()),
                            )
                            .on_toggle(move |next, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.toggle_thought(id, *next);
                                    cx.notify();
                                });
                            }),
                    )
                    .into_any_element()
            }
            RowKind::Tool(card) => self.tool_card(card),
            RowKind::Permission(card) => self.permission_card(card, cx),
            RowKind::Notice { level, text } => Self::row_shell()
                .child(Notice::new(
                    Self::row_id(row),
                    match level {
                        NoticeLevel::Info => UiTone::Info,
                        NoticeLevel::Warning => UiTone::Warning,
                    },
                    text.clone(),
                ))
                .into_any_element(),
            RowKind::Error(error) => self.error_card(row, error),
            RowKind::Validation(card) => self.validation_card(row, card),
            RowKind::Changes(card) => self.changes_card(card),
            RowKind::Outcome(card) => {
                let (tone, title) = outcome_style(card.kind);
                let mut lines = vec![card.summary.clone()];
                if let Some(p) = &card.published {
                    lines.push(format!("Revision {}", &p[..p.len().min(12)]));
                }
                if card.draft_retained {
                    lines.push("The agent's working copy was kept.".to_owned());
                }
                Self::row_shell()
                    .child(Notice::new(Self::row_id(row), tone, title).body(lines.join("\n")))
                    .into_any_element()
            }
        }
    }

    /// Records the expansion of a reasoning row and re-measures that row only.
    pub(super) fn toggle_thought(&mut self, id: u64, open: bool) {
        if open {
            self.thoughts_open.insert(id);
        } else {
            self.thoughts_open.remove(&id);
        }
        if let Some(ix) = self.view_rows.iter().position(|row| row.id.0 == id) {
            self.list.remeasure_items(ix..ix + 1);
        }
    }

    pub(super) fn tool_card(&self, card: &ToolCard) -> AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let (status, tone) = tool_badge(card.status);
        Self::row_shell()
            .child(
                Card::editor_nested(&self.theme)
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .p(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(icon(Icon::Hammer, 14., palette.text_2))
                            .child(
                                label(
                                    &self.theme,
                                    format!("Tool · {}", card.kind.as_deref().unwrap_or("call")),
                                )
                                .flex_1()
                                .min_w_0(),
                            )
                            .child(Badge::new(status).tone(tone)),
                    )
                    .child(
                        text_style(div(), TextStyle::Body)
                            .text_color(palette.text_2)
                            .child(card.title.clone().unwrap_or_else(|| card.call_id.clone())),
                    )
                    .children(
                        (card.updates > 1).then(|| self.line(format!("{} updates", card.updates))),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn permission_card(
        &mut self,
        card: &PermissionCard,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pending = self.replies.is_pending(&card.reference);
        let open = card.state == PermissionState::Open && !pending;
        let state = match &card.state {
            PermissionState::Open if pending => "Answer sent…".to_owned(),
            PermissionState::Open => "Waiting for your choice".to_owned(),
            PermissionState::Selected { option_id } => format!(
                "You chose {}",
                card.options
                    .iter()
                    .find(|o| &o.option_id == option_id)
                    .map_or(option_id.as_str(), |o| o.name.as_str())
            ),
            PermissionState::Cancelled => "Closed without an answer".to_owned(),
        };
        let key = format!(
            "{}-{}-{}",
            &card.reference.task.0[..card.reference.task.0.len().min(8)],
            card.reference.writer,
            card.reference.request
        );
        let mut notice = Notice::new(
            format!("perm-notice-{key}"),
            if card.state == PermissionState::Open {
                UiTone::Warning
            } else {
                UiTone::Neutral
            },
            "Permission requested",
        )
        .body(format!("{}\n{state}", card.title));
        if open {
            for option in &card.options {
                let reference = card.reference.clone();
                let id = option.option_id.clone();
                notice = notice.action(self.action_button(
                    format!("perm-{key}-{}", option.option_id),
                    option.name.clone(),
                    true,
                    if option.kind.starts_with("allow") {
                        Tone::Primary
                    } else {
                        Tone::Normal
                    },
                    cx,
                    move |this, _, cx| {
                        this.answer_permission(
                            reference.clone(),
                            PermissionAnswer::Select(id.clone()),
                            cx,
                        )
                    },
                ));
            }
            let reference = card.reference.clone();
            notice = notice.action(self.action_button(
                format!("perm-{key}-decline"),
                "Decline",
                true,
                Tone::Normal,
                cx,
                move |this, _, cx| {
                    this.answer_permission(reference.clone(), PermissionAnswer::Cancel, cx)
                },
            ));
        }
        Self::row_shell().child(notice).into_any_element()
    }

    pub(super) fn error_card(&self, row: &Row, error: &StructuredError) -> AnyElement {
        let mut lines = vec![error.detail.clone()];
        if let Some(a) = &error.action {
            lines.push(format!("Next: {a}"));
        }
        if let Some(p) = &error.phase {
            lines.push(format!("During {p} · code {}", error.code));
        }
        if error.retained {
            lines.push("The working copy and candidate were kept.".to_owned());
        }
        Self::row_shell()
            .child(
                Notice::new(Self::row_id(row), UiTone::Danger, error.title.clone())
                    .body(lines.join("\n")),
            )
            .into_any_element()
    }

    pub(super) fn validation_card(&self, row: &Row, card: &ValidationCard) -> AnyElement {
        let mut lines: Vec<String> = Vec::new();
        if let Some(c) = &card.coverage {
            lines.push(format!(
                "Coverage: {} of {} frames rendered, {} inspected, {} boundary frames, playhead {}{}{}",
                c.rendered_frames,
                c.total_frames,
                c.inspected_frames,
                c.boundary_frames,
                c.playhead,
                c.broadened
                    .as_ref()
                    .map(|b| format!(", broadened: {b}"))
                    .unwrap_or_default(),
                if c.complete { "" } else { " · PARTIAL" }
            ));
        }
        if let Some(a) = &card.audio {
            lines.push(format!(
                "Audio: {}, peak {:.2}, {} Hz, placement {}",
                if a.silent { "silent" } else { "present" },
                a.peak,
                a.sample_rate,
                if a.placement_verified {
                    "verified"
                } else {
                    "not verified"
                }
            ));
        }
        for error in card.errors.iter().take(6) {
            lines.push(format!("{:?} · {}", error.stage, error.message));
        }
        if card.errors_total > card.errors.len().min(6) {
            let more = card.errors_total - card.errors.len().min(6);
            lines.push(format!(
                "{more} more error{}",
                if more == 1 { "" } else { "s" }
            ));
        }
        if !card.warnings.is_empty() {
            lines.push(format!(
                "{} warning{}: {}",
                card.warnings.len(),
                if card.warnings.len() == 1 { "" } else { "s" },
                card.warnings
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" · ")
            ));
        }
        if !card.capability_gaps.is_empty() {
            lines.push(format!(
                "Not verified by this preview: {}",
                card.capability_gaps.join(", ")
            ));
        }
        if let Some(f) = &card.failure {
            lines.push(format!(
                "{:?} failure in {:?}: {}",
                f.kind, f.stage, f.summary
            ));
        }
        lines.push(format!(
            "Candidate {}{}{} · repair {}",
            &card.candidate[..card.candidate.len().min(12)],
            card.build_key
                .as_ref()
                .map(|k| format!(" · build {}", &k[..k.len().min(12)]))
                .unwrap_or_default(),
            format_args!(" · {} diagnostics", card.diagnostics_total),
            card.repair_count
        ));
        let (tone, head) = if card.passed {
            (UiTone::Success, "Validation passed")
        } else {
            (UiTone::Danger, "Validation failed")
        };
        Self::row_shell()
            .child(
                Notice::new(
                    Self::row_id(row),
                    tone,
                    format!("{head} · {}", card.summary),
                )
                .body(lines.join("\n")),
            )
            .into_any_element()
    }

    pub(super) fn changes_card(&self, card: &ChangeCard) -> AnyElement {
        use studio_engine::candidate_validation::ChangeKind;
        let theme = self.theme;
        let palette = theme.editor;
        let mut body = Card::editor_nested(&self.theme)
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(10.))
            .child(label(
                &self.theme,
                format!(
                    "{} changed file{}",
                    card.total,
                    if card.total == 1 { "" } else { "s" }
                ),
            ));
        for entry in &card.entries {
            let (mark, color) = match entry.change {
                ChangeKind::Added => ("A", theme.green_11),
                ChangeKind::Modified => ("M", palette.text_2),
                ChangeKind::Deleted => ("D", theme.red_11),
                ChangeKind::Mode => ("X", palette.text_3),
            };
            body = body.child(
                text_style(div(), TextStyle::Small)
                    .flex()
                    .gap(px(8.))
                    .text_color(palette.text_1)
                    .child(div().w(px(10.)).flex_none().text_color(color).child(mark))
                    .child(div().min_w_0().child(entry.path.clone())),
            );
        }
        if card.truncated || card.total > card.entries.len() {
            body = body.child(self.line(format!(
                "…and {} more",
                card.total.saturating_sub(card.entries.len())
            )));
        }
        Self::row_shell().child(body).into_any_element()
    }

    // ---- tabs ----------------------------------------------------------------------------------------

    pub(super) fn banner(
        &mut self,
        snapshot: &WorkflowSnapshot,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Div> {
        let task = snapshot.task.as_ref()?;
        let tone = banner_tone(task.phase);
        let id = task.id.0.as_str();
        let mut lines = vec![format!(
            "Task {} · generation {} · base {}",
            &id[..id.len().min(8)],
            task.generation,
            task.source_base
        )];
        if !matches!(
            task.phase,
            TaskPhase::AwaitingReview | TaskPhase::AwaitingEvidence
        ) {
            lines.push(format!(
                "{} · {} of {} automatic repairs",
                match task.review_policy {
                    ReviewPolicy::AutoApply => "Apply automatically",
                    ReviewPolicy::ManualReview => "Manual review",
                },
                task.repair.used,
                task.repair.max
            ));
        }
        if let Some(reason) = &task.reason {
            lines.push(reason.clone());
        }
        if let Some(note) = &controls.stop
            && !task.phase.is_terminal()
        {
            lines.push(note.clone());
        }
        if let Some(note) = controls.review.as_ref().and_then(|r| r.apply_note.as_ref()) {
            lines.push(format!(
                "Apply is blocked: {note}. The candidate is kept; retry after fixing it."
            ));
        }
        let mut notice = Notice::new(
            "agent-banner",
            tone,
            format!(
                "{}{}",
                task.phase.label(),
                if task.stop_requested {
                    " · stopping"
                } else {
                    ""
                }
            ),
        )
        .body(lines.join("\n"));
        if let Some(review) = &controls.review {
            for button in self.review_buttons(review, cx) {
                notice = notice.action(button);
            }
        }
        let mut banner = div().flex().flex_col().gap(px(8.)).child(notice);
        if let Some(conflict) = &task.conflict {
            banner = banner.child(self.conflict_block(conflict));
        }
        Some(banner)
    }

    pub(super) fn review_buttons(
        &mut self,
        review: &ReviewControls,
        cx: &mut Context<Self>,
    ) -> Vec<EditorButton> {
        vec![
            self.action_button(
                "agent-apply",
                if review.apply_note.is_some() {
                    "Retry Apply"
                } else {
                    "Apply"
                },
                true,
                Tone::Primary,
                cx,
                |this, _, cx| this.apply(cx),
            ),
            self.action_button(
                "agent-discard",
                "Discard",
                true,
                Tone::Danger,
                cx,
                |this, _, cx| {
                    if let Some(w) = this.workflow().cloned() {
                        let r = w.discard();
                        this.command(r, cx);
                    }
                },
            ),
            self.action_button(
                "agent-export-candidate",
                "Export candidate…",
                true,
                Tone::Normal,
                cx,
                |this, _, cx| this.export(Export::Candidate, cx),
            ),
        ]
    }

    pub(super) fn conflict_block(&self, conflict: &ConflictView) -> gpui::Div {
        let list = |title: &str, paths: &[String]| {
            let mut block = div().flex().flex_col();
            if !paths.is_empty() {
                block = block.child(self.line(title.to_owned()));
                for path in paths.iter().take(8) {
                    block = block.child(self.line(format!("  {path}")));
                }
                if paths.len() > 8 {
                    block = block.child(self.line(format!("  …and {} more", paths.len() - 8)));
                }
            }
            block
        };
        Card::editor_nested(&self.theme)
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(10.))
            .child(label(&self.theme, format!("Conflict: {}", conflict.reason)))
            .child(list(
                "Changed in the project meanwhile:",
                &conflict.external_paths,
            ))
            .child(list("The candidate changes:", &conflict.candidate_paths))
            .child(list("Changed on both sides:", &conflict.overlapping))
            .children((!conflict.retained_variants.is_empty()).then(|| {
                let mut block = div().flex().flex_col();
                block = block.child(self.line("Preserved variants (nothing was overwritten):"));
                for variant in conflict.retained_variants.iter().take(8) {
                    block =
                        block.child(self.line(format!("  {} · {}", variant.path, variant.role)));
                }
                block
            }))
    }
}
