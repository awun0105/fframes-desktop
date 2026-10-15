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
                .map(|note| div().child(self.line(note.clone())));
        }
        let mut bar = div().flex().flex_col().items_start().gap(px(8.));
        if controls.modes {
            let mut rail = self.segment_rail();
            for mode in &snapshot.options.modes {
                let selected = snapshot.options.current_mode.as_deref() == Some(mode.id.as_str());
                let id = mode.id.clone();
                rail = rail.child(self.segment_item(
                    format!("mode-{}", mode.id),
                    if selected {
                        format!("● {}", mode.name)
                    } else {
                        mode.name.clone()
                    },
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
            bar = bar.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .items_center()
                    .child(self.line("Mode"))
                    .child(rail),
            );
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
                            div().w_full().child(field_row(
                                &self.theme,
                                label.clone(),
                                Toggle::plain(&self.theme, id, *current)
                                    .label(label)
                                    .on_change(move |_, _, app| {
                                        entity.update(app, |this, cx| {
                                            if let Some(w) = this.workflow().cloned() {
                                                let r =
                                                    w.set_config_option(&option_id, next.clone());
                                                this.command(r, cx);
                                            }
                                        });
                                    }),
                            )),
                        );
                    }
                    ConfigKind::Select { current, values } => {
                        let name = values
                            .iter()
                            .find(|v| &v.value == current)
                            .map_or(current.as_str(), |v| v.name.as_str());
                        let entity = cx.entity();
                        bar = bar.child(
                            div().w_full().child(field_row(
                                &self.theme,
                                option.name.clone(),
                                div().min_w(px(160.)).child(
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
                            )),
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
        let handle = self.prompt.read(cx).focus_handle();
        window.focus(&handle, cx);
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
        // Everything above the composer shrinks (the transcript first) and is clipped when
        // the panel is short; the composer stays pinned at the bottom.
        let mut column = div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(8.))
            .px(px(12.))
            .pt(px(12.));
        let g = guidance(
            &snapshot.adapter.readiness,
            snapshot.adapter.provider.is_some(),
        );
        if let Some(banner) = self.banner(&snapshot, controls, cx) {
            column = column.child(banner);
        } else if !g.usable || snapshot.rows.is_empty() {
            column = column.child(
                self.section(&g.headline)
                    .children(g.steps.iter().take(3).map(|s| self.line(s.clone()))),
            );
        }
        if let Some(bar) = self.options_bar(&snapshot, controls, cx) {
            column = column.child(bar);
        }
        if let Some(error) = &self.scope_error {
            column = column.child(self.line(format!("Timeline scope unavailable: {error}")));
        } else if let Some(scope) = &self.task_scope {
            let same_as_task = snapshot
                .task
                .as_ref()
                .filter(|task| !task.phase.is_terminal())
                .is_some_and(|task| task.scope.label() == scope.label());
            if !same_as_task {
                column = column.child(self.line(format!("Submit scope: {}", scope.label())));
            }
        }
        if let Some(task) = &snapshot.task {
            let label = if task.phase.is_terminal() {
                "Last task scope"
            } else {
                "Frozen task scope"
            };
            column = column.child(self.line(format!("{label}: {}", task.scope.label())));
        }
        if let Some(task) = &snapshot.task {
            let mut evidence_cards = div().flex().gap_1();
            let mut has_evidence_card = false;
            for (name, view) in [("Before", &task.before), ("After", &task.after)] {
                if let Some(view) = view {
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
                    if let (Some(artifact), Some(image)) = (selected, image) {
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
                    } else {
                        column = column.child(self.line(evidence_summary(name, view)));
                    }
                }
            }
            if has_evidence_card {
                column = column.child(evidence_cards);
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
                column = column.child(self.line(summary.to_owned()));
            }
        }
        if self.log_has_older() {
            let loading = self.page.loading.load(Ordering::Acquire);
            column = column.child(self.action_button(
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
            ));
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
        let mut transcript = div()
            .id("agent-transcript")
            .role(gpui::Role::List)
            .aria_label("Agent conversation")
            .relative()
            .flex_1()
            .min_h(px(80.))
            .child(list);
        if self.hidden_newer > 0 || !self.older.is_empty() {
            // Floats above the composer while the reader is away from the newest rows.
            transcript = transcript.child(
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
        column = column.child(transcript);
        if !snapshot.queue.is_empty() {
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
            column = column.child(queue);
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(column)
            .child(
                div()
                    .flex_none()
                    .px(px(12.))
                    .pt(px(8.))
                    .pb(px(12.))
                    .child(self.prompt_form(controls, cx)),
            )
            .into_any_element()
    }

    pub(super) fn prompt_form(&mut self, controls: &Controls, cx: &mut Context<Self>) -> gpui::Div {
        let theme = self.theme;
        let enabled = controls.submit.blocked.is_none();
        let label = controls.submit.label();
        let hint = controls.submit.blocked.clone();
        let entity = cx.entity();
        // An icon button: the submit label is its tooltip and accessible name.
        let send = IconButton::editor(&theme, "agent-send", Icon::ArrowUp.path())
            .editor_size(IconButtonSize::Small)
            .accent(&theme)
            .rounded(px(999.))
            .label(label)
            .tooltip(&theme, label)
            .disabled(!enabled)
            .on_click(Self::click(cx, |this, _, cx| this.submit(cx)))
            .into_any_element();
        let stop = controls
            .stop
            .is_some()
            .then(|| self.stop_button(cx).into_any_element());
        // A fixed 64 px container around the single-line input: the input row on top and
        // the actions at the bottom right, so the height never depends on the content.
        let composer = div()
            .relative()
            .flex_none()
            .w_full()
            .h(px(COMPOSER_H))
            .rounded(px(METRICS.radius_card))
            .bg(theme.editor.ctl)
            .child(
                div()
                    .id("agent-prompt")
                    .absolute()
                    .top(px(COMPOSER_INSET))
                    .left(px(COMPOSER_INSET))
                    .right(px(COMPOSER_INSET))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                        let prompt = this.prompt.clone();
                        this.key_in_field(&prompt, event, true, window, cx);
                    }))
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                entity.update(cx, |panel, _| {
                                    panel.prompt_bounds = Some(probe::bounds_to_record(bounds));
                                });
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .child(TextInput::plain(
                        &self.theme,
                        "agent-prompt-input",
                        &self.prompt,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .right(px(COMPOSER_INSET))
                    .bottom(px(COMPOSER_INSET))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .children(stop)
                    .child(send),
            );
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(4.))
            .child(composer)
            .children(hint.map(|text| self.line(text)))
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
