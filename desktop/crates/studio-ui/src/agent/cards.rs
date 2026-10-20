//! Transcript rows and their cards: tool calls, permissions, errors, validation, changes, banners and conflicts.

use super::markdown::{MarkdownRow, Voice};
use super::*;

impl ConversationPanel {
    pub(super) fn row_id(row: &Row) -> ElementId {
        ElementId::from(("agent-row", row.id.0))
    }

    pub(super) fn row_element(&mut self, row: &Arc<Row>, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        match &row.kind {
            // The user's messages are bubbles on the right; the agent answers on the left
            // without a frame, and its tool calls run as a log between them.
            RowKind::User { text, source } => Self::row_shell()
                .pt(px(4.))
                .flex()
                .flex_col()
                .items_end()
                .gap(px(2.))
                .children(match source {
                    UserSource::Brief => None,
                    UserSource::Clarification => Some(self.caption("Your answer")),
                    UserSource::Followup => Some(self.caption("Follow-up")),
                    UserSource::Repair => {
                        Some(self.caption("Automatic repair context (sent by Studio)"))
                    }
                })
                .child(
                    div()
                        .max_w(gpui::relative(0.88))
                        .rounded(px(12.))
                        .bg(palette.ctl)
                        .px(px(12.))
                        .py(px(8.))
                        .child(
                            text_style(div(), TextStyle::Body)
                                .text_color(palette.text_1)
                                .child(selectable(text.clone()).id(("agent-row-text", row.id.0))),
                        ),
                )
                .into_any_element(),
            RowKind::Agent { .. } => Self::row_shell()
                .child(div().px(px(2.)).child(
                    MarkdownRow::new(row, Voice::Answer).project_root(self.project_root.clone()),
                ))
                .into_any_element(),
            RowKind::Thought { streaming, .. } => {
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
                                div().pt(px(4.)).child(
                                    MarkdownRow::new(row, Voice::Reasoning)
                                        .project_root(self.project_root.clone()),
                                ),
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
            RowKind::Tool(card) => self.tool_card(row, card),
            RowKind::Permission(card) => self.permission_card(card),
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
            RowKind::Changes(card) => self.changes_card(card, cx),
            RowKind::Outcome(card) => {
                let (tone, title) = outcome_style(card.kind);
                let mut lines = vec![card.summary.clone()];
                if let Some(p) = &card.published {
                    lines.push(format!("Revision {}", &p[..p.len().min(12)]));
                }
                if card.draft_retained {
                    lines.push("The agent's working copy was kept.".to_owned());
                }
                // The end of a task is one line of the log; the status strip and the error
                // card above it carry the detail.
                let glyph = match tone {
                    UiTone::Success => (Icon::CircleCheck, theme.green_11.into()),
                    UiTone::Danger => (Icon::AlertTriangle, theme.red_11.into()),
                    UiTone::Warning => (Icon::AlertTriangle, theme.amber_11.into()),
                    _ => (Icon::Info, palette.text_3.into()),
                };
                self.activity_row(
                    Self::row_id(row),
                    glyph,
                    title,
                    lines.join(" "),
                    div().into_any_element(),
                )
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

    /// A tool call as one line of the activity log: what kind of step, on what, and how
    /// it went. The full title is in the tooltip when the line truncates it.
    pub(super) fn tool_card(&self, row: &Row, card: &ToolCard) -> AnyElement {
        let theme = self.theme;
        let (verb, glyph) = tool_verb(card.kind.as_deref());
        let title = card.title.clone().unwrap_or_else(|| card.call_id.clone());
        // Agents usually start the title with the verb ("Read src/main.rs"): then the
        // title's own first word is the verb, not a repeat of it.
        let (verb, title) = match title.split_once(' ') {
            Some((first, rest)) if first.eq_ignore_ascii_case(verb) => {
                (first.to_owned(), rest.to_owned())
            }
            _ => (verb.to_owned(), title),
        };
        let status = match card.status {
            Some(ToolStatus::Completed) => {
                icon(Icon::Check, 12., theme.green_11).into_any_element()
            }
            Some(ToolStatus::Failed) => {
                Badge::new("failed").tone(UiTone::Danger).into_any_element()
            }
            other => text_style(div(), TextStyle::Small)
                .text_color(theme.editor.text_3)
                .child(tool_badge(other).0)
                .into_any_element(),
        };
        self.activity_row(
            ("agent-tool", row.id.0),
            (glyph, theme.editor.text_3.into()),
            verb,
            title,
            status,
        )
    }

    /// One line of the activity log: a glyph, a verb, a truncated subject with its full
    /// text in a tooltip, and a status at the end. Consecutive lines sit close together.
    fn activity_row(
        &self,
        id: impl Into<ElementId>,
        (glyph, tint): (Icon, gpui::Hsla),
        verb: impl Into<SharedString>,
        subject: impl Into<SharedString>,
        status: AnyElement,
    ) -> AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let verb: SharedString = verb.into();
        let subject: SharedString = subject.into();
        let tip: SharedString = format!("{verb} {subject}").into();
        div()
            .w_full()
            .pb(px(2.))
            .child(
                div()
                    .id(id)
                    .h(px(ACTIVITY_ROW_H))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(6.))
                    .rounded(px(METRICS.radius_control))
                    .hover(|style| style.bg(palette.ctl))
                    .tooltip_show_delay(crate::kit::TOOLTIP_SHOW_DELAY)
                    .tooltip(move |_, cx| crate::kit::Tooltip::new(&theme, tip.clone()).view(cx))
                    .child(icon(glyph, 13., tint))
                    .child(
                        text_style(div().flex_none(), TextStyle::Small)
                            .text_color(palette.text_1)
                            .child(verb),
                    )
                    .child(
                        text_style(div().flex_1().min_w_0().truncate(), TextStyle::Small)
                            .text_color(palette.text_2)
                            .child(subject),
                    )
                    .child(div().flex_none().child(status)),
            )
            .into_any_element()
    }

    /// Whether `card` is an agent question, which has its own card above the composer.
    pub(super) fn is_question(&self, card: &PermissionCard) -> bool {
        self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot
                .composer
                .questions
                .iter()
                .any(|question| question.reference == card.reference)
        })
    }

    fn permission_key(card: &PermissionCard) -> String {
        format!(
            "{}-{}-{}",
            &card.reference.task.0[..card.reference.task.0.len().min(8)],
            card.reference.writer,
            card.reference.request
        )
    }

    /// The transcript row of a permission request. An open request is answered in its
    /// card above the composer ([`Self::permission_request`]); the row only records it.
    pub(super) fn permission_card(&self, card: &PermissionCard) -> AnyElement {
        let key = Self::permission_key(card);
        if let Some(policy) = card.answered_by {
            let state = if card.state == PermissionState::Cancelled {
                "closed before the agent took it"
            } else {
                policy.label()
            };
            return self.activity_row(
                format!("perm-auto-{key}"),
                (Icon::CircleCheck, self.theme.green_11.into()),
                "Allowed",
                card.title.clone(),
                self.caption(state).into_any_element(),
            );
        }
        let (glyph, state) = match &card.state {
            PermissionState::Open if self.is_question(card) => {
                (Icon::Info, "answer in the question card".to_owned())
            }
            // The pinned request shows the command; the row stays one line until answered.
            PermissionState::Open => {
                return self.activity_row(
                    format!("perm-open-{key}"),
                    (Icon::AlertTriangle, self.theme.amber_11.into()),
                    "Permission",
                    card.title.clone(),
                    self.caption("answer below").into_any_element(),
                );
            }
            PermissionState::Selected { option_id } => (
                Icon::CircleCheck,
                format!(
                    "you chose {}",
                    card.options
                        .iter()
                        .find(|o| &o.option_id == option_id)
                        .map_or(option_id.as_str(), |o| o.name.as_str())
                ),
            ),
            PermissionState::Cancelled => (Icon::X, "closed without an answer".to_owned()),
        };
        self.activity_row(
            format!("perm-notice-{key}"),
            (glyph, self.theme.editor.text_3.into()),
            "Permission",
            card.title.clone(),
            self.caption(&state).into_any_element(),
        )
    }

    /// The open permission request pinned above the composer, with the agent's options.
    /// Decline (closes the request and ends the turn) is offered only when the agent gives
    /// no way to refuse; otherwise it would sit next to the agent's own Reject.
    pub(super) fn permission_request(
        &mut self,
        card: &PermissionCard,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let pending = self.replies.is_pending(&card.reference);
        let key = Self::permission_key(card);
        let mut notice = Notice::new(
            format!("perm-request-{key}"),
            UiTone::Warning,
            "Permission requested",
        )
        .body(if pending {
            format!("{}\nAnswer sent…", card.title)
        } else {
            card.title.clone()
        });
        if !pending {
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
            if !card.options.iter().any(|o| o.kind.starts_with("reject")) {
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
        }
        div().child(notice)
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

    pub(super) fn changes_card(&mut self, card: &ChangeCard, cx: &mut Context<Self>) -> AnyElement {
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
        if let Some(button) = self.candidate_review_button(card, cx) {
            body = body.child(button);
        }
        Self::row_shell().child(body).into_any_element()
    }

    // ---- tabs ----------------------------------------------------------------------------------------

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
