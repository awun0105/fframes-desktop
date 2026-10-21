//! The Chat tab: the options bar and its menu, the transcript and the prompt.

use super::*;

impl ConversationPanel {
    pub(super) fn options_bar(
        &mut self,
        snapshot: &WorkflowSnapshot,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Div> {
        if !controls.modes && !controls.config_options {
            return controls
                .options_note
                .as_ref()
                .map(|note| div().child(self.caption(note)));
        }
        // One wrapping toolbar under the composer, like the mode and model pickers of other
        // agent chats: it configures the next message, so it sits next to the prompt.
        let mut bar = div().flex().flex_wrap().items_center().gap(px(6.));
        if controls.modes {
            let mut rail = self.segment_rail();
            for mode in &snapshot.options.modes {
                let selected = snapshot.options.current_mode.as_deref() == Some(mode.id.as_str());
                let id = mode.id.clone();
                rail = rail.child(self.segment_item(
                    format!("mode-{}", mode.id),
                    mode.name.clone(),
                    selected,
                    cx,
                    move |this, _, cx| {
                        if let Some(w) = this.workflow().cloned() {
                            let r = w.set_mode(&id);
                            this.command(r, cx);
                        }
                    },
                ));
            }
            bar = bar.child(rail);
        }
        if controls.config_options {
            for option in &snapshot.options.config {
                let option_id = option.id.clone();
                let id = format!("config-{}", option.id);
                match &option.kind {
                    ConfigKind::Boolean { current } => {
                        let next = OptionValue::Boolean(!current);
                        let label =
                            format!("{}: {}", option.name, if *current { "on" } else { "off" });
                        let entity = cx.entity();
                        bar = bar.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(self.caption(&option.name))
                                .child(
                                    Toggle::plain(&self.theme, id, *current)
                                        .label(label)
                                        .on_change(move |_, _, app| {
                                            entity.update(app, |this, cx| {
                                                if let Some(w) = this.workflow().cloned() {
                                                    let r = w.set_config_option(
                                                        &option_id,
                                                        next.clone(),
                                                    );
                                                    this.command(r, cx);
                                                }
                                            });
                                        }),
                                ),
                        );
                    }
                    ConfigKind::Select { current, values } => {
                        let name = values
                            .iter()
                            .find(|v| &v.value == current)
                            .map_or(current.as_str(), |v| v.name.as_str());
                        let entity = cx.entity();
                        bar = bar.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(self.caption(&option.name))
                                .child(
                                    div().min_w(px(120.)).child(
                                        Select::plain(&self.theme, id, name.to_owned())
                                            .stretch_label()
                                            .on_open(move |bounds, window, app| {
                                                entity.update(app, |this, cx| {
                                                    this.toggle_config_menu(
                                                        &option_id, *bounds, window, cx,
                                                    );
                                                });
                                            }),
                                    ),
                                ),
                        );
                    }
                }
            }
        }
        Some(bar)
    }

    pub(super) fn toggle_config_menu(
        &mut self,
        option_id: &str,
        trigger: gpui::Bounds<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_config.as_deref() == Some(option_id) {
            self.close_config_menu(window, cx);
            return;
        }
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let Some(mut menu) = ConfigMenu::of(&snapshot.options.config, option_id) else {
            return;
        };
        menu.state = MenuState::anchored(trigger, &menu.items);
        self.config_menus.insert(option_id.to_owned(), menu);
        self.open_config = Some(option_id.to_owned());
        window.focus(&self.config_focus, cx);
        cx.notify();
    }

    pub(super) fn close_config_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_config.take().is_some() {
            self.focus_prompt(window, cx);
            cx.notify();
        }
    }

    /// Shows the chat and moves the keyboard focus to its prompt.
    pub fn focus_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = Tab::Chat;
        self.focus_composer(window, cx);
    }

    /// Sets option `option_id` to `value`, the value of the row the reader clicked in the
    /// menu as it was painted.
    pub(super) fn commit_config(
        &mut self,
        option_id: &str,
        value: Option<OptionValue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_config.as_deref() != Some(option_id) {
            return;
        }
        self.open_config = None;
        if let (Some(value), Some(w)) = (value, self.workflow().cloned()) {
            let r = w.set_config_option(option_id, value);
            self.command(r, cx);
        }
        self.focus_prompt(window, cx);
        cx.notify();
    }

    /// Keeps an open menu in step with the options the agent advertises now: changed
    /// values rebuild it, a withdrawn option closes it.
    pub(super) fn sync_config_menu(&mut self) {
        let Some(option_id) = self.open_config.clone() else {
            return;
        };
        let fresh = self
            .snapshot
            .as_ref()
            .and_then(|s| ConfigMenu::of(&s.options.config, &option_id));
        let Some(fresh) = fresh else {
            self.open_config = None;
            return;
        };
        let Some(menu) = self.config_menus.get_mut(&option_id) else {
            self.open_config = None;
            return;
        };
        menu.refresh(fresh);
    }

    pub(super) fn render_config_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let option_id = self.open_config.as_ref()?;
        let menu = self.config_menus.get(option_id)?;
        let highlight = cx.entity();
        let select = cx.entity();
        let dismiss = cx.entity();
        let key = option_id.clone();
        let chosen = option_id.clone();
        // The values of the rows as painted now: a click commits the one it hit.
        let shown = menu.values.clone();
        Some(
            Menu::editor(
                &self.theme,
                "agent-config-menu",
                menu.items.clone(),
                &menu.state,
            )
            .focus(&self.config_focus)
            .on_highlight(move |index, _, cx| {
                highlight.update(cx, |this, cx| {
                    if let Some(menu) = this.config_menus.get_mut(&key) {
                        menu.state.set_highlighted(*index);
                    }
                    cx.notify();
                });
            })
            .on_select(move |index, window, cx| {
                let value = shown.get(*index).cloned().map(OptionValue::Value);
                select.update(cx, |this, cx| {
                    this.commit_config(&chosen, value, window, cx)
                });
            })
            .on_dismiss(move |window, cx| {
                dismiss.update(cx, |this, cx| this.close_config_menu(window, cx));
            })
            .into_any_element(),
        )
    }

    pub(super) fn chat_tab(
        &mut self,
        snapshot: Arc<WorkflowSnapshot>,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Top to bottom: the task in one line, the transcript, then the dock (cards that
        // wait for an answer, the composer and its toolbar). The transcript keeps a minimum
        // height; the dock's cards scroll instead of pushing it out of the panel. The setup
        // guidance opens the transcript area, where it scrolls in a short sidebar.
        let theme = self.theme;
        let g = guidance(
            &snapshot.adapter.readiness,
            snapshot.adapter.provider.is_some(),
        );
        let mut top = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(8.))
            .px(px(12.))
            .pt(px(10.))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        text_style(div().flex_1().min_w_0(), TextStyle::Small)
                            .text_color(if g.usable {
                                theme.editor.text_2
                            } else {
                                theme.editor.accent
                            })
                            .child(g.headline.clone()),
                    )
                    .child(super::policy_badge(self.permission_policy)),
            );
        let strip = self.status_strip(&snapshot, controls, cx);
        let show_guidance = strip.is_none() && (!g.usable || snapshot.rows.is_empty());
        top = top.children(strip);
        let answering =
            !snapshot.composer.questions.is_empty() || snapshot.resources.open_permissions > 0;
        // An empty transcript (the setup guidance scrolls) needs no more room than one
        // that gives way to an open question.
        let mut transcript = div()
            .flex_1()
            .min_h(px(if answering || snapshot.rows.is_empty() {
                TRANSCRIPT_MIN_ANSWERING_H
            } else {
                TRANSCRIPT_MIN_H
            }))
            .flex()
            .flex_col()
            .gap(px(8.))
            .px(px(12.))
            .pt(px(8.));
        if show_guidance {
            transcript = transcript.child(
                div()
                    .id("agent-guidance")
                    .flex_shrink(1.)
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        self.section(&g.headline)
                            .children(g.steps.iter().take(3).map(|s| self.line(s.clone()))),
                    ),
            );
        }
        if self.log_has_older() {
            let loading = self.page.loading.load(Ordering::Acquire);
            transcript = transcript.child(div().flex_none().child(self.action_button(
                "agent-load-earlier",
                if loading {
                    "Loading earlier messages…"
                } else {
                    "Load earlier messages"
                },
                !loading,
                Tone::Normal,
                cx,
                |this, _, cx| {
                    if let Some(first) = this.view_rows.first() {
                        let before = first.id.0;
                        this.request_page(before, rows::PAGE_ROWS);
                        cx.notify();
                    }
                },
            )));
        }
        let list = list(
            self.list.clone(),
            cx.processor(
                |this, ix: usize, _window, cx| match this.view_rows.get(ix).cloned() {
                    Some(row) => this.row_element(&row, cx),
                    None => div().into_any_element(),
                },
            ),
        )
        .size_full();
        let mut rows = div()
            .id("agent-transcript")
            .role(gpui::Role::List)
            .aria_label("Agent conversation")
            .relative()
            .flex_1()
            .min_h_0()
            .child(list);
        if self.hidden_newer > 0 || !self.older.is_empty() {
            // Floats over the newest rows while the reader is away from them.
            rows = rows.child(
                div()
                    .absolute()
                    .bottom(px(8.))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .rounded(px(METRICS.radius_control))
                            .bg(self.theme.editor.card)
                            .child(self.action_button(
                                "agent-jump-latest",
                                if self.hidden_newer > 0 {
                                    format!(
                                        "Jump to latest · {} newer messages hidden",
                                        self.hidden_newer
                                    )
                                } else {
                                    "Jump to latest".to_owned()
                                },
                                true,
                                Tone::Normal,
                                cx,
                                |this, _, cx| this.jump_to_latest(cx),
                            )),
                    ),
            );
        }
        transcript = transcript.child(rows);
        // A sidebar shorter than the transcript's minimum and the composer scrolls rather
        // than cutting off Send.
        div()
            .id("agent-chat")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .child(top)
            .child(transcript)
            .child(self.composer_dock(&snapshot, controls, cx))
            .into_any_element()
    }

    /// The task in one line: its phase, why it is there, Stop and a Details toggle. Review
    /// actions and a conflict stay in view; ids, repairs, scope and evidence wait behind
    /// Details, so the strip never crowds out the transcript.
    pub(super) fn status_strip(
        &mut self,
        snapshot: &WorkflowSnapshot,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Div> {
        let task = snapshot.task.as_ref()?;
        let palette = self.theme.editor;
        let open = self.task_details_open;
        let phase = format!(
            "{}{}",
            task.phase.label(),
            if task.stop_requested {
                " · stopping"
            } else {
                ""
            }
        );
        let summary = task.reason.clone().unwrap_or_else(|| task.scope.label());
        let mut head = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .min_h(px(24.))
            .child(Badge::new(phase).tone(banner_tone(task.phase)))
            .child(
                text_style(div().flex_1().min_w_0().truncate(), TextStyle::Small)
                    .text_color(palette.text_2)
                    .child(summary),
            );
        // The composer's own Stop covers a live turn; this one every other state a task
        // can be in (waiting, reviewing, queued briefs).
        if !snapshot.composer.turn_live && controls.stop.is_some() {
            head = head.child(self.stop_button(cx));
        }
        head = head.child(self.icon_button(
            "agent-task-details",
            if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            if open { "Hide details" } else { "Task details" },
            Tone::Normal,
            cx,
            |this, _, cx| {
                this.task_details_open = !this.task_details_open;
                cx.notify();
            },
        ));
        let mut strip = Card::editor_nested(&self.theme)
            .flex()
            .flex_col()
            .gap(px(6.))
            .px(px(10.))
            .py(px(6.))
            .child(head);
        if let Some(review) = &controls.review {
            if let Some(note) = &review.apply_note {
                strip = strip.child(self.line(format!(
                    "Apply is blocked: {note}. The candidate is kept; retry after fixing it."
                )));
            }
            let mut buttons = div().flex().flex_wrap().gap(px(6.));
            for button in self.review_buttons(review, cx) {
                buttons = buttons.child(button);
            }
            strip = strip.child(buttons.child(self.banner_review_button(cx)));
        }
        if let Some(conflict) = &task.conflict {
            strip = strip.child(self.conflict_block(conflict));
        }
        if open {
            strip = strip.child(self.task_details(snapshot, controls));
        }
        Some(strip)
    }

    /// What the status strip keeps behind Details: the task's identity, its repair budget,
    /// what Stop does now, the frozen scope and the before/after evidence.
    fn task_details(&self, snapshot: &WorkflowSnapshot, controls: &Controls) -> gpui::Div {
        let mut details = div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .pt(px(6.))
            .border_t_1()
            .border_color(self.theme.editor.line);
        let Some(task) = &snapshot.task else {
            return details;
        };
        let id = task.id.0.as_str();
        details = details.child(self.line(format!(
            "Task {} · generation {} · base {}",
            &id[..id.len().min(8)],
            task.generation,
            task.source_base
        )));
        if !matches!(
            task.phase,
            TaskPhase::AwaitingReview | TaskPhase::AwaitingEvidence
        ) {
            details = details.child(self.line(format!(
                "{} · {} of {} automatic repairs",
                match task.review_policy {
                    ReviewPolicy::AutoApply => "Apply automatically",
                    ReviewPolicy::ManualReview => "Manual review",
                },
                task.repair.used,
                task.repair.max
            )));
        }
        if let Some(note) = &controls.stop
            && !task.phase.is_terminal()
        {
            details = details.child(self.line(note.clone()));
        }
        details = details.child(self.line(format!(
            "{}: {}",
            if task.phase.is_terminal() {
                "Last task scope"
            } else {
                "Frozen task scope"
            },
            task.scope.label()
        )));
        let mut evidence_cards = div().flex().gap_1();
        let mut has_evidence_card = false;
        for (name, view) in [("Before", &task.before), ("After", &task.after)] {
            let Some(view) = view else {
                continue;
            };
            let selected = view
                .artifacts
                .iter()
                .find(|artifact| artifact.label == "selected")
                .or_else(|| view.artifacts.first());
            let image = selected.and_then(|artifact| {
                self.evidence_images
                    .get(&evidence_image_key(&task.id, &artifact.id))
                    .map(|(_, image)| image)
            });
            let (Some(artifact), Some(image)) = (selected, image) else {
                details = details.child(self.line(evidence_summary(name, view)));
                continue;
            };
            has_evidence_card = true;
            let boundaries = view
                .artifacts
                .iter()
                .filter(|other| other.id != artifact.id)
                .flat_map(|other| other.frames.iter().copied())
                .map(|frame| frame.to_string())
                .collect::<Vec<_>>()
                .join(",");
            evidence_cards = evidence_cards.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(label_text(format!("{name} · revision {}", view.revision)))
                    .child(gpui::img(image.clone()).w(px(128.)).h(px(72.)))
                    .child(
                        text_style(div(), TextStyle::Label)
                            .text_color(self.theme.editor.text_2)
                            .child(selectable(format!(
                                "{} · {} frames{}",
                                artifact.label,
                                artifact.frames.len(),
                                if boundaries.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · edge {boundaries}")
                                }
                            ))),
                    ),
            );
        }
        if has_evidence_card {
            details = details.child(evidence_cards);
        }
        if let Some(limit) = task
            .image_limitation
            .as_ref()
            .filter(|_| task.before.is_some())
        {
            let summary = if limit.contains("Text-only") {
                "Image context is text-only; the agent receives artifact references."
            } else {
                limit
            };
            details = details.child(self.line(summary.to_owned()));
        }
        details
    }

    /// Briefs waiting for the running task, each with a remove button.
    pub(super) fn queue_section(
        &mut self,
        snapshot: &WorkflowSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Div> {
        if snapshot.queue.is_empty() {
            return None;
        }
        let mut queue = self.section(&format!("Queued ({})", snapshot.queue.len()));
        for item in &snapshot.queue {
            let id = item.id;
            queue = queue.child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(label_text(item.summary.clone()))
                            .children(item.scope.as_ref().map(|scope| {
                                self.line(format!(
                                    "{scope}{}",
                                    if item.stale_scope {
                                        " · stale — will be refused"
                                    } else {
                                        ""
                                    }
                                ))
                            })),
                    )
                    .child(self.icon_button(
                        format!("queue-remove-{id}"),
                        Icon::X,
                        "Remove",
                        Tone::Normal,
                        cx,
                        move |this, _, cx| {
                            if let Some(w) = this.workflow().cloned() {
                                let r = w.cancel_queued(id);
                                this.command(r, cx);
                            }
                        },
                    )),
            );
        }
        Some(queue)
    }

    pub(super) fn stop_button(&mut self, cx: &mut Context<Self>) -> IconButton {
        self.icon_button(
            "agent-stop",
            Icon::Square,
            "Stop",
            Tone::Danger,
            cx,
            |this, _, cx| this.stop(cx),
        )
        .rounded(px(999.))
    }
}
