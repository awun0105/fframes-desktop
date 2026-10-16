//! Review of what an agent changed: unified and split diffs of each changed file.
//!
//! [`Changes`] is a standalone gpui view. It takes the changed files as [`DiffFile`]s (full text
//! before and after), diffs them by lines, and paints a file list beside the diff. It never
//! applies anything: choices leave as [`ChangesEvent`]s for the owner to act on.
//!
//! Rows are virtualised with a `gpui::list`, so a diff of any size builds only the rows on
//! screen. Code is clipped at the column edge. Highlighting is paint-only: the text is laid out
//! as is and `agent::syntax` spans only change the colour of its runs.

mod file_list;
mod model;
mod paint;
mod split;
#[cfg(test)]
mod tests;
mod unified;

use std::collections::{BTreeSet, HashSet};

use gpui::{
    AnyElement, ClickEvent, Context, EventEmitter, InteractiveElement, IntoElement, ListAlignment,
    ListOffset, ListState, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Window, div, list, px,
};
use studio_theme::{TextStyle, Theme, text_style};

pub use model::{DiffFile, DiffMode, NotShown};

use model::{DiffRow, FileDiff, flatten_rows};
use paint::{Colors, FILE_HEADER_H, HUNK_HEADER_H, Highlights, MONO_FAMILY, NOTICE_H, RowStyle};

use crate::kit::{Button, ButtonSize, ButtonVariant, Icon, SegmentOption, SegmentedControl, icon};

/// Rows kept built beyond the viewport edges.
const OVERDRAW_PX: f32 = 600.;

/// Where the changes are in their life, which decides what a reviewer can do with a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Review {
    /// Not applied yet: each file has an accept toggle and rejected files can be dropped.
    #[default]
    Pending,
    /// Already applied: each file can be reverted.
    Applied,
}

/// What the viewer asks its owner to do. The viewer performs none of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangesEvent {
    /// A file's accept toggle was flipped.
    FileToggled { path: String, accepted: bool },
    /// Drop these files (every file currently toggled off) from the candidate.
    Reject(Vec<String>),
    /// Undo the change to this one file after apply.
    RevertFile(String),
    /// Leave the viewer.
    Close,
}

impl EventEmitter<ChangesEvent> for Changes {}

pub struct Changes {
    files: Vec<FileDiff>,
    highlights: Vec<Highlights>,
    mode: DiffMode,
    /// Long lines wrap onto more rows instead of clipping at the column edge.
    wrap: bool,
    review: Review,
    collapsed: HashSet<String>,
    rejected: BTreeSet<String>,
    rows: Vec<DiffRow>,
    /// Row index of each file's header.
    headers: Vec<usize>,
    list: ListState,
}

impl Changes {
    pub fn new(files: Vec<DiffFile>) -> Self {
        let mut changes = Self {
            files: Vec::new(),
            highlights: Vec::new(),
            mode: DiffMode::default(),
            wrap: true,
            review: Review::default(),
            collapsed: HashSet::new(),
            rejected: BTreeSet::new(),
            rows: Vec::new(),
            headers: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW_PX)),
        };
        changes.load(files);
        changes
    }

    /// Replaces the files. Collapse and accept choices survive for paths that are still there.
    pub fn set_files(&mut self, files: Vec<DiffFile>, cx: &mut Context<Self>) {
        self.load(files);
        cx.notify();
    }

    fn load(&mut self, files: Vec<DiffFile>) {
        self.files = files.iter().map(FileDiff::compute).collect();
        self.highlights = files
            .iter()
            .map(|file| {
                Highlights::compute(&file.path, file.before.as_deref(), file.after.as_deref())
            })
            .collect();
        let present: HashSet<&str> = self.files.iter().map(|f| f.path.as_str()).collect();
        self.collapsed
            .retain(|path| present.contains(path.as_str()));
        self.rejected.retain(|path| present.contains(path.as_str()));
        self.rebuild();
    }

    pub fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            self.rebuild();
            cx.notify();
        }
    }

    pub fn mode(&self) -> DiffMode {
        self.mode
    }

    pub fn set_wrap(&mut self, wrap: bool, cx: &mut Context<Self>) {
        if self.wrap != wrap {
            self.wrap = wrap;
            self.list.reset(self.rows.len());
            cx.notify();
        }
    }

    pub fn wrap(&self) -> bool {
        self.wrap
    }

    pub fn set_review(&mut self, review: Review, cx: &mut Context<Self>) {
        if self.review != review {
            self.review = review;
            cx.notify();
        }
    }

    /// The files toggled off, in file order.
    pub fn rejected(&self) -> Vec<String> {
        self.files
            .iter()
            .filter(|file| self.rejected.contains(&file.path))
            .map(|file| file.path.clone())
            .collect()
    }

    fn rebuild(&mut self) {
        let (rows, headers) = flatten_rows(&self.files, self.mode, |ix| {
            self.collapsed.contains(&self.files[ix].path)
        });
        self.rows = rows;
        self.headers = headers;
        self.list.reset(self.rows.len());
    }

    /// Scrolls so the file's header is at the top.
    fn reveal(&mut self, path: &str, cx: &mut Context<Self>) {
        let header = self
            .files
            .iter()
            .position(|file| file.path == path)
            .and_then(|ix| self.headers.get(ix));
        if let Some(&item_ix) = header {
            self.list.scroll_to(ListOffset {
                item_ix,
                offset_in_item: px(0.),
            });
            cx.notify();
        }
    }

    fn toggle_collapsed(&mut self, path: &str, cx: &mut Context<Self>) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_owned());
        }
        self.rebuild();
        self.reveal(path, cx);
    }

    fn set_all_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.collapsed = if collapsed {
            self.files.iter().map(|file| file.path.clone()).collect()
        } else {
            HashSet::new()
        };
        self.rebuild();
        cx.notify();
    }

    fn set_accepted(&mut self, path: &str, accepted: bool, cx: &mut Context<Self>) {
        if accepted {
            self.rejected.remove(path);
        } else {
            self.rejected.insert(path.to_owned());
        }
        cx.emit(ChangesEvent::FileToggled {
            path: path.to_owned(),
            accepted,
        });
        cx.notify();
    }

    fn file_header(
        &self,
        file_ix: usize,
        theme: &Theme,
        colors: &Colors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let file = &self.files[file_ix];
        let path = file.path.clone();
        let collapsed = self.collapsed.contains(&file.path);
        let revert_path = file.path.clone();
        let mut header = text_style(div(), TextStyle::Label)
            .id(SharedString::from(format!("changes-file-{file_ix}")))
            .w_full()
            .h(px(FILE_HEADER_H))
            .px(px(10.))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .bg(colors.header_bg)
            .border_b_1()
            .border_color(colors.line)
            .text_color(colors.text)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.toggle_collapsed(&path, cx);
            }))
            .child(icon(
                if collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                },
                14.,
                colors.muted,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .font_family(MONO_FAMILY)
                    .child(SharedString::from(file.path.clone())),
            );
        if file.not_shown.is_none() {
            header = header
                .child(
                    div()
                        .text_color(colors.add)
                        .child(SharedString::from(format!("+{}", file.additions))),
                )
                .child(
                    div()
                        .text_color(colors.del)
                        .child(SharedString::from(format!("-{}", file.deletions))),
                );
        }
        if self.review == Review::Applied {
            header = header.child(
                div()
                    .debug_selector(move || format!("changes-revert-{file_ix}"))
                    .child(
                        Button::plain(
                            theme,
                            SharedString::from(format!("changes-revert-button-{file_ix}")),
                            ButtonVariant::Gray,
                            ButtonSize::Xs,
                        )
                        .label("Revert")
                        .on_click(cx.listener(
                            move |_, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                cx.emit(ChangesEvent::RevertFile(revert_path.clone()));
                            },
                        )),
                    ),
            );
        }
        header.into_any_element()
    }

    fn row_style<'a>(&'a self, file: u32, theme: &'a Theme, colors: &'a Colors) -> RowStyle<'a> {
        RowStyle {
            highlights: &self.highlights[file as usize],
            max_line: self.files[file as usize].max_line,
            wrap: self.wrap,
            theme,
            colors,
        }
    }

    fn row(&self, ix: usize, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(&row) = self.rows.get(ix) else {
            return div().into_any_element();
        };
        let theme = Theme::for_window(window, cx);
        let colors = Colors::new(&theme);
        match row {
            DiffRow::FileHeader { file } => self.file_header(file as usize, &theme, &colors, cx),
            DiffRow::Notice { file, notice } => {
                let text = self.files[file as usize].notices[notice as usize].clone();
                text_style(div(), TextStyle::Label)
                    .debug_selector(move || format!("changes-notice-{file}-{notice}"))
                    .w_full()
                    .h(px(NOTICE_H))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .text_color(colors.muted)
                    .child(SharedString::from(text))
                    .into_any_element()
            }
            DiffRow::HunkHeader { file, hunk } => {
                let header = self.files[file as usize].hunks[hunk as usize]
                    .header
                    .clone();
                div()
                    .w_full()
                    .h(px(HUNK_HEADER_H))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .bg(colors.header_bg.opacity(0.5))
                    .font_family(MONO_FAMILY)
                    .text_size(px(11.))
                    .text_color(colors.faint)
                    .child(SharedString::from(header))
                    .into_any_element()
            }
            DiffRow::Line { file, hunk, line } => {
                let f = &self.files[file as usize];
                let line = &f.hunks[hunk as usize].lines[line as usize];
                let style = self.row_style(file, &theme, &colors);
                unified::row(line, &style)
                    .debug_selector(move || format!("changes-line-{ix}"))
                    .into_any_element()
            }
            DiffRow::SplitLine {
                file,
                hunk,
                left,
                right,
            } => {
                let hunk = &self.files[file as usize].hunks[hunk as usize];
                let style = self.row_style(file, &theme, &colors);
                split::row(hunk, left, right, &style)
                    .debug_selector(move || format!("changes-split-{ix}"))
                    .into_any_element()
            }
        }
    }

    fn toolbar(&self, theme: &Theme, colors: &Colors, cx: &mut Context<Self>) -> AnyElement {
        let (files, additions, deletions) = file_list::totals(&self.files);
        let rejected = self.rejected();
        let all_collapsed = !self.files.is_empty() && self.collapsed.len() == self.files.len();
        let mode = self.mode;
        let wrap = self.wrap;
        let mut bar = text_style(div(), TextStyle::Label)
            .w_full()
            .h(px(40.))
            .flex_none()
            .px(px(10.))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .border_b_1()
            .border_color(colors.line)
            .text_color(colors.text)
            .child(SharedString::from(format!(
                "{files} {}",
                if files == 1 { "file" } else { "files" }
            )))
            .child(
                div()
                    .text_color(colors.add)
                    .child(SharedString::from(format!("+{additions}"))),
            )
            .child(
                div()
                    .text_color(colors.del)
                    .child(SharedString::from(format!("-{deletions}"))),
            )
            .child(div().flex_1())
            .child(
                div().debug_selector(|| "changes-mode".into()).child(
                    SegmentedControl::editor(
                        theme,
                        "changes-mode-control",
                        vec![
                            SegmentOption::new("Unified", mode == DiffMode::Unified),
                            SegmentOption::new("Split", mode == DiffMode::Split),
                        ],
                    )
                    .on_select(cx.listener(|this, ix: &usize, _, cx| {
                        let mode = if *ix == 0 {
                            DiffMode::Unified
                        } else {
                            DiffMode::Split
                        };
                        this.set_mode(mode, cx);
                    })),
                ),
            )
            .child(
                Button::plain(
                    theme,
                    "changes-collapse",
                    ButtonVariant::Gray,
                    ButtonSize::Sm,
                )
                .label(if all_collapsed {
                    "Expand all"
                } else {
                    "Collapse all"
                })
                .disabled(self.files.is_empty())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_all_collapsed(!all_collapsed, cx);
                })),
            )
            .child(
                div().debug_selector(|| "changes-wrap".into()).child(
                    Button::plain(theme, "changes-wrap", ButtonVariant::Gray, ButtonSize::Sm)
                        .label(if wrap { "No wrap" } else { "Wrap" })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_wrap(!wrap, cx);
                        })),
                ),
            );
        if self.review == Review::Pending {
            let paths = rejected.clone();
            bar = bar.child(
                div().debug_selector(|| "changes-reject".into()).child(
                    Button::plain(
                        theme,
                        "changes-reject-button",
                        ButtonVariant::Destructive,
                        ButtonSize::Sm,
                    )
                    .label(if rejected.is_empty() {
                        SharedString::from("Reject")
                    } else {
                        SharedString::from(format!("Reject {}", rejected.len()))
                    })
                    .disabled(rejected.is_empty())
                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                        cx.emit(ChangesEvent::Reject(paths.clone()));
                    })),
                ),
            );
        }
        bar.child(
            div().debug_selector(|| "changes-close".into()).child(
                Button::plain(
                    theme,
                    "changes-close-button",
                    ButtonVariant::Gray,
                    ButtonSize::Sm,
                )
                .label("Close")
                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                    cx.emit(ChangesEvent::Close);
                })),
            ),
        )
        .into_any_element()
    }
}

impl Render for Changes {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let colors = Colors::new(&theme);
        let body = if self.files.is_empty() {
            text_style(div(), TextStyle::Body)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(colors.muted)
                .child("No changes")
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_row()
                .child(self.file_list(&theme, &colors, cx))
                .child(
                    div().flex_1().min_w_0().h_full().child(
                        list(
                            self.list.clone(),
                            cx.processor(|this, ix: usize, window, cx| this.row(ix, window, cx)),
                        )
                        .size_full(),
                    ),
                )
                .into_any_element()
        };
        div()
            .id("changes")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.editor.card)
            .child(self.toolbar(&theme, &colors, cx))
            .child(body)
    }
}
