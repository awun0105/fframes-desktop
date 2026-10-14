//! Parts of the window that render as views of their own, after Cap's `EditorSectionView`.
//!
//! Each section renders the shell's own `render_*` method and re-renders whenever the shell
//! notifies. The header, sidebar and agent column are mounted `.cached(..)`, so a new frame,
//! which notifies only the player section, leaves them as they were.

use gpui::{
    AppContext, Context, Entity, IntoElement, Render, Styled, Subscription, WeakEntity, Window,
    canvas, div,
};

use super::StudioShell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Header,
    Sidebar,
    Agent,
    Player,
}

impl Section {
    pub(super) fn name(self) -> &'static str {
        match self {
            Section::Header => "header",
            Section::Sidebar => "sidebar",
            Section::Agent => "agent",
            Section::Player => "player",
        }
    }
}

pub(super) struct SectionView {
    shell: WeakEntity<StudioShell>,
    pub(super) section: Section,
    /// How often the section rendered, reported by the qualification telemetry.
    pub(super) renders: u64,
    _shell_subscription: Subscription,
}

impl SectionView {
    fn new(shell: &Entity<StudioShell>, section: Section, cx: &mut Context<Self>) -> Self {
        Self {
            shell: shell.downgrade(),
            section,
            renders: 0,
            _shell_subscription: cx.observe(shell, |_, _, cx| cx.notify()),
        }
    }
}

impl Render for SectionView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(shell) = self.shell.upgrade() else {
            return div().into_any_element();
        };
        self.renders += 1;
        let section = self.section;
        shell.update(cx, |shell, cx| match section {
            Section::Header => shell.render_header(cx),
            Section::Sidebar => shell.render_left_column(cx),
            Section::Agent => shell.render_agent_column(cx),
            Section::Player => shell.render_player_area(cx),
        })
    }
}

/// Records the bounds of the element it fills into the shell's `section_bounds`, so tests see
/// the box a cached section was given: a cached view lays its content out on its own.
pub(super) fn section_probe(
    section: Section,
    cx: &mut Context<StudioShell>,
) -> impl IntoElement + use<> {
    box_probe(section.name(), cx)
}

/// Records the bounds of the element it fills into the shell's `section_bounds` as `name`.
pub(super) fn box_probe(
    name: &'static str,
    cx: &mut Context<StudioShell>,
) -> impl IntoElement + use<> {
    let shell = cx.entity();
    canvas(
        move |bounds, _, cx| {
            let measured = [
                f32::from(bounds.origin.x),
                f32::from(bounds.origin.y),
                f32::from(bounds.size.width),
                f32::from(bounds.size.height),
            ];
            shell.update(cx, |shell, _| {
                shell.section_bounds.insert(name, measured);
            });
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The section views, created on the shell's first render.
#[derive(Clone)]
pub(super) struct Sections {
    pub(super) header: Entity<SectionView>,
    pub(super) sidebar: Entity<SectionView>,
    pub(super) agent: Entity<SectionView>,
    pub(super) player: Entity<SectionView>,
}

impl Sections {
    pub(super) fn new(cx: &mut Context<StudioShell>) -> Self {
        let shell = cx.entity();
        let mut view = |section| cx.new(|cx| SectionView::new(&shell, section, cx));
        Self {
            header: view(Section::Header),
            sidebar: view(Section::Sidebar),
            agent: view(Section::Agent),
            player: view(Section::Player),
        }
    }

    pub(super) fn all(&self) -> [&Entity<SectionView>; 4] {
        [&self.header, &self.sidebar, &self.agent, &self.player]
    }
}
