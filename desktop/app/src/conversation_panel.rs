//! The native agent panel: setup and readiness, a virtualized conversation, review,
//! Undo and recovery. It renders immutable [`WorkflowSnapshot`]s and sends commands; it
//! never touches the filesystem, the adapter, hashing or compilation (the workflow's
//! threads do), and it never holds a lock across a frame.
//!
//! * Rows live in a `gpui::list` (visible rows plus [`rows::OVERDRAW_PX`] of overscan are
//!   measured and painted). [`rows`] decides which rows the list owns and how a new
//!   snapshot becomes list operations, so streaming text and tool-card updates re-measure
//!   in place while the reader's position holds.
//! * [`controls`] decides which controls exist in which state (pure, unit-tested).
//! * [`studio_engine::providers::host`] holds the app-local adapter description and the workflow inbox.
//! * [`provider_setup`] selects providers, detects and installs adapters and turns them on.
pub mod controls;
pub mod provider_setup;
pub mod rows;

/// Redacted observation of an evidence set: state, revision and frame labels only.
fn evidence_telemetry(view: &studio_engine::agent::EvidenceView) -> serde_json::Value {
    serde_json::json!({
        "state": format!("{:?}", view.state),
        "revision": view.revision,
        "artifacts": view.artifacts.iter().map(|a| serde_json::json!({
            "label": a.label, "frames": a.frames, "bytes": a.bytes, "sha256": a.sha256[..12.min(a.sha256.len())],
        })).collect::<Vec<_>>(),
    })
}

/// Redacted observation of a transcript row: its kind and size, never its text.
fn row_telemetry(kind: &RowKind) -> serde_json::Value {
    use serde_json::json;
    match kind {
        RowKind::User { text, .. } => json!({"kind": "user", "chars": text.chars().count()}),
        RowKind::Agent {
            text, streaming, ..
        } => json!({"kind": "agent", "chars": text.chars().count(), "streaming": streaming}),
        RowKind::Thought { .. } => json!({"kind": "thought"}),
        RowKind::Tool(_) => json!({"kind": "tool"}),
        RowKind::Permission(card) => {
            json!({"kind": "permission", "state": format!("{:?}", card.state)})
        }
        RowKind::Notice { .. } => json!({"kind": "notice"}),
        RowKind::Error(error) => json!({"kind": "error", "code": error.code}),
        RowKind::Validation(_) => json!({"kind": "validation"}),
        RowKind::Changes(_) => json!({"kind": "changes"}),
        RowKind::Outcome(card) => json!({"kind": "outcome", "outcome": format!("{:?}", card.kind)}),
    }
}

/// One line per evidence set: its state, the immutable revision it was rendered from and
/// the actual frames rendered. Never predicted pixels.
fn evidence_summary(name: &str, view: &studio_engine::agent::EvidenceView) -> String {
    use studio_engine::agent::EvidenceState;
    let state = match view.state {
        EvidenceState::Pending => "rendering…".to_owned(),
        EvidenceState::Ready => {
            let frames: Vec<String> = view
                .artifacts
                .iter()
                .map(|a| format!("{} {:?}", a.label, a.frames))
                .collect();
            format!("{} · rendered {}", view.artifacts.len(), frames.join("; "))
        }
        EvidenceState::Unavailable => format!(
            "unavailable: {}",
            view.note.as_deref().unwrap_or("not rendered")
        ),
        EvidenceState::Released if !view.artifacts.is_empty() => {
            let frames: Vec<String> = view
                .artifacts
                .iter()
                .map(|a| format!("{} {:?}", a.label, a.frames))
                .collect();
            format!(
                "{} · rendered {} · artifact files released with task",
                view.artifacts.len(),
                frames.join("; ")
            )
        }
        EvidenceState::Released => "released with its task".to_owned(),
    };
    format!("{name} evidence (revision {}): {state}", view.revision)
}

fn evidence_image_key(task: &studio_engine::AgentTaskId, artifact_id: &str) -> String {
    format!("{}:{artifact_id}", task.0)
}

use crate::{
    design_system::{self, TextStyle, text_style},
    ui::{
        self, Badge, BoundsProbe, Button, ButtonVariant, Collapsible, FieldRow, Icon, IconButton,
        IconButtonSize, Menu, MenuState, Notice, Select, Toggle, Tone as UiTone, icon,
    },
};
use controls::{
    Controls, InputKey, KeyMods, ReplyTracker, ReviewControls, SaveTracker, SubmitKind,
    UndoControl, derive_controls, guidance, route_input_key,
};
use gpui::{
    AnyElement, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, FollowMode,
    InteractiveElement, IntoElement, ListAlignment, ListState, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, canvas, div, list, px,
};
use parking_lot::Mutex;
use rows::{ListOp, OVERDRAW_PX, ResidentLimits, RowKey, Viewport};
use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use studio_engine::agent::{
    AdapterReadiness, AgentWorkflow, ChangeCard, ConflictView, HandoffState, HistoryEntry,
    NoticeLevel, OutcomeKind, PermissionAnswer, PermissionCard, PermissionRef, PermissionState,
    Row, RowKind, StructuredError, TaskPhase, ToolCard, UndoView, UserSource, ValidationCard,
    WorkflowSnapshot,
};
use studio_engine::containment::{OwnershipPolicy, Resolution};
use studio_engine::providers::host::{self, AdapterFields, AdapterFile, McpChoice};
use studio_engine::providers::provider_profiles;
use studio_engine::{DraftState, ReviewPolicy, TaskScope, app_paths::AppPaths};
use studio_harness::driver::{ConfigKind, OptionValue, ToolStatus};
use studio_theme::Theme;
use studio_ui::kit::{TextInput, TextInputEvent, TextInputState};

/// Height of the composer container and the inset of its contents.
const COMPOSER_H: f32 = 64.;
/// Height of a reasoning row's header, with or without the "Thinking…" badge.
const THOUGHT_HEADER_H: f32 = 20.;
/// Height of the header bar holding the tabs.
const HEADER_H: f32 = 46.;
const COMPOSER_INSET: f32 = 4.;

/// Events the shell reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelEvent {
    /// Escape in an input: focus returns to the shell.
    Leave,
}

impl EventEmitter<PanelEvent> for ConversationPanel {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Chat,
    Project,
    Setup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Normal,
    Primary,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Export {
    Candidate,
    Draft,
}

struct Attached {
    workflow: Arc<AgentWorkflow>,
    #[allow(dead_code)]
    paths: AppPaths,
    serial: u64,
}

/// Everything the shell hands the panel when a project's workflow opens.
pub struct Attachment {
    pub workflow: Arc<AgentWorkflow>,
    pub paths: AppPaths,
    /// The shell's open serial (hand-offs and history pages carry it).
    pub serial: u64,
    /// The saved adapter description (what the Setup tab starts with).
    pub adapter: Option<AdapterFile>,
    /// The app-local provider registry; defaults are shown when no registry exists yet.
    pub registry: Option<provider_profiles::ProviderRegistry>,
    /// Display-only provider statuses read from the installed M6 ledger.
    pub qualification: provider_profiles::QualificationSnapshot,
    pub settings_error: Option<String>,
    /// How the saved adapter's writer containment was derived.
    pub resolution: Option<Resolution>,
    pub policy: OwnershipPolicy,
}

/// (attachment serial, chat generation at request time, the page).
type PageResult = (u64, u64, Result<Vec<Arc<Row>>, String>);

/// The open menu of a select-type agent option: the names it shows and, row for row, the
/// values they commit. Both are built together, so a click commits the value whose name the
/// reader saw even when the agent changed its options since.
struct ConfigMenu {
    state: MenuState,
    values: Vec<String>,
}

impl ConfigMenu {
    /// A menu listing the values of the select-type option `id`, the current one checked.
    fn of(config: &[studio_harness::driver::ConfigOption], id: &str) -> Option<Self> {
        let option = config.iter().find(|o| o.id == id)?;
        let ConfigKind::Select { current, values } = &option.kind else {
            return None;
        };
        let names = values.iter().map(|v| v.name.clone().into()).collect();
        Some(Self {
            state: MenuState::new(names, values.iter().position(|v| &v.value == current)),
            values: values.iter().map(|v| v.value.clone()).collect(),
        })
    }

    fn empty() -> Self {
        Self {
            state: MenuState::new(Vec::new(), None),
            values: Vec::new(),
        }
    }
}

#[derive(Default)]
struct PageSlot {
    done: Mutex<Option<PageResult>>,
    loading: AtomicBool,
    /// Tells the shell a page finished.
    wake: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

struct SetupForm {
    provider: Entity<TextInputState>,
    executable: Entity<TextInputState>,
    args: Entity<TextInputState>,
    env_names: Entity<TextInputState>,
}

pub struct ConversationPanel {
    attached: Option<Attached>,
    snapshot: Option<Arc<WorkflowSnapshot>>,
    tab: Tab,
    prompt: Entity<TextInputState>,
    /// Refreshed at the top of every render.
    theme: Theme,
    prompt_kind: Option<SubmitKind>,
    setup: SetupForm,
    saved_adapter: Option<AdapterFile>,
    registry: provider_profiles::ProviderRegistry,
    qualification: provider_profiles::QualificationSnapshot,
    settings_error: Option<String>,
    /// How the saved adapter's writer containment was derived (shown read-only).
    containment: Option<Resolution>,
    /// Who may say a writer is contained (production: validated evidence only).
    policy: OwnershipPolicy,
    saves: SaveTracker,
    mcp_choice: McpChoice,
    /// Detect agents, Install adapter and the turn-on of a Select or Install click.
    agent_setup: provider_setup::AgentSetup,
    list: ListState,
    view_rows: Vec<Arc<Row>>,
    view_keys: Vec<RowKey>,
    older: Vec<Arc<Row>>,
    older_exhausted: bool,
    hidden_newer: usize,
    page: Arc<PageSlot>,
    viewport: Rc<Cell<Option<usize>>>,
    replies: ReplyTracker,
    notice: Option<String>,
    sdk_ready: bool,
    task_scope: Option<TaskScope>,
    scope_error: Option<String>,
    /// Measured bounds of the buttons a native driver must click (qualification only).
    button_bounds: std::collections::HashMap<String, [f32; 4]>,
    /// Current prompt input bounds, used by the native shell qualification harness.
    prompt_bounds: Option<[f32; 4]>,
    /// Current-task thumbnails are local presentation state, never workflow snapshot data.
    /// Each entry keeps the workflow's decoded thumbnail it was converted from.
    evidence_images: HashMap<String, (Arc<image::RgbaImage>, Arc<gpui::RenderImage>)>,
    /// Images removed from the current task; released from GPUI on the next render.
    retired_evidence_images: Vec<Arc<gpui::RenderImage>>,
    /// Reasoning rows the reader expanded (presentation state only).
    thoughts_open: std::collections::HashSet<u64>,
    /// The workflow's chat generation the shown rows belong to.
    chat: u64,
    /// One menu per select-type agent option, kept across frames so each trigger
    /// reports its own bounds.
    config_menus: HashMap<String, ConfigMenu>,
    /// The option whose menu is open.
    open_config: Option<String>,
    config_focus: FocusHandle,
}

/// A single-line field. Return in the prompt sends it; Escape in any field hands the
/// keyboard back to the shell.
fn input(
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<ConversationPanel>,
) -> Entity<TextInputState> {
    let placeholder = placeholder.to_owned();
    let input = cx.new(|cx| {
        let mut input = TextInputState::single_line(window, cx);
        input.set_placeholder(placeholder);
        input
    });
    cx.subscribe(
        &input,
        |panel, input, event: &TextInputEvent, cx| match event {
            TextInputEvent::Confirmed if input == panel.prompt => panel.submit(cx),
            TextInputEvent::Cancelled => cx.emit(PanelEvent::Leave),
            _ => (),
        },
    )
    .detach();
    input
}

impl ConversationPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let list = ListState::new(0, ListAlignment::Top, px(OVERDRAW_PX));
        list.set_follow_mode(FollowMode::Tail);
        let viewport = Rc::new(Cell::new(None));
        let seen = viewport.clone();
        let panel = cx.entity().downgrade();
        list.set_scroll_handler(move |event, _, cx| {
            seen.set(Some(event.visible_range.start));
            // Scrolled towards the top: page in earlier history.
            let panel = panel.clone();
            cx.defer(move |cx| {
                let _ = panel.update(cx, |panel, _| panel.maybe_request_page());
            });
        });
        Self {
            attached: None,
            snapshot: None,
            tab: Tab::Chat,
            prompt: input("Describe the change you want", window, cx),
            theme: Theme::for_window(window, cx),
            prompt_kind: None,
            setup: SetupForm {
                provider: input("Label, for example my-adapter", window, cx),
                executable: input("Absolute path of the ACP adapter", window, cx),
                args: input("Arguments, for example --acp", window, cx),
                env_names: input("e.g. ANTHROPIC_API_KEY", window, cx),
            },
            saved_adapter: None,
            registry: provider_profiles::ProviderRegistry::default(),
            qualification: provider_profiles::QualificationSnapshot::default(),
            settings_error: None,
            containment: None,
            policy: OwnershipPolicy::Validated,
            saves: SaveTracker::default(),
            mcp_choice: McpChoice::Baseline,
            agent_setup: provider_setup::AgentSetup::default(),
            list,
            view_rows: Vec::new(),
            view_keys: Vec::new(),
            older: Vec::new(),
            older_exhausted: false,
            hidden_newer: 0,
            page: Arc::new(PageSlot::default()),
            viewport,
            replies: ReplyTracker::default(),
            notice: None,
            sdk_ready: false,
            task_scope: None,
            scope_error: None,
            button_bounds: std::collections::HashMap::new(),
            prompt_bounds: None,
            evidence_images: HashMap::new(),
            retired_evidence_images: Vec::new(),
            thoughts_open: std::collections::HashSet::new(),
            chat: 0,
            config_menus: HashMap::new(),
            open_config: None,
            config_focus: cx.focus_handle(),
        }
    }

    // ---- shell interface ---------------------------------------------------------------------

    /// Binds the panel to a freshly opened workflow.
    pub fn attach(&mut self, attachment: Attachment, cx: &mut Context<Self>) {
        let Attachment {
            workflow,
            paths,
            serial,
            adapter,
            registry,
            qualification,
            settings_error,
            resolution,
            policy,
        } = attachment;
        self.detach(cx);
        self.attached = Some(Attached {
            workflow,
            paths,
            serial,
        });
        self.mcp_choice = adapter.as_ref().map_or(McpChoice::Baseline, |a| a.mcp);
        self.registry = registry.unwrap_or_default();
        self.qualification = qualification;
        self.fill_setup(adapter.as_ref(), cx);
        self.saved_adapter = adapter;
        self.settings_error = settings_error;
        self.containment = resolution;
        self.policy = policy;
        self.tab = Tab::Chat;
        self.poll(cx);
        cx.notify();
    }

    /// Forgets the workflow (the shell closes it).
    pub fn detach(&mut self, cx: &mut Context<Self>) {
        self.attached = None;
        self.snapshot = None;
        self.qualification = provider_profiles::QualificationSnapshot::default();
        self.task_scope = None;
        self.scope_error = None;
        self.reset_transcript();
        self.open_config = None;
        self.notice = None;
        self.prompt_kind = None;
        self.saves.cancel();
        // Stops a running install and drops every in-flight scan, install and turn-on.
        self.agent_setup.reset();
        cx.notify();
    }

    /// Forgets every shown and paged-in row and the state keyed by their ids.
    fn reset_transcript(&mut self) {
        self.older.clear();
        self.older_exhausted = false;
        self.hidden_newer = 0;
        self.view_rows.clear();
        self.view_keys.clear();
        self.list.reset(0);
        self.viewport.set(None);
        self.page.loading.store(false, Ordering::Release);
        self.page.done.lock().take();
        self.replies = ReplyTracker::default();
        self.thoughts_open.clear();
    }

    pub fn workflow(&self) -> Option<&Arc<AgentWorkflow>> {
        self.attached.as_ref().map(|a| &a.workflow)
    }

    pub fn attached_serial(&self) -> Option<u64> {
        self.attached.as_ref().map(|a| a.serial)
    }

    pub fn set_sdk_ready(&mut self, ready: bool, cx: &mut Context<Self>) {
        if self.sdk_ready != ready {
            self.sdk_ready = ready;
            cx.notify();
        }
    }

    /// Receives the latest immutable selection snapshot from the compiled timeline.
    pub fn set_task_scope(
        &mut self,
        scope: Result<Option<TaskScope>, String>,
        cx: &mut Context<Self>,
    ) {
        match scope {
            Ok(scope) => {
                self.task_scope = scope;
                self.scope_error = None;
            }
            Err(error) => {
                self.task_scope = None;
                self.scope_error = Some(error);
            }
        }
        cx.notify();
    }

    /// True while the prompt (or a setup field) holds keyboard focus.
    pub fn has_text_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        use gpui::Focusable;
        let focused = |input: &Entity<TextInputState>| input.focus_handle(cx).is_focused(window);
        focused(&self.prompt)
            || focused(&self.setup.provider)
            || focused(&self.setup.executable)
            || focused(&self.setup.args)
            || focused(&self.setup.env_names)
    }

    /// Redacted observation of the panel for the M3 telemetry file: counts and states,
    /// never prompt, brief or message text.
    pub fn telemetry(&self) -> serde_json::Value {
        let Some(snapshot) = &self.snapshot else {
            return serde_json::json!({"attached": false, "setup": self.setup_telemetry()});
        };
        // Unique `Arc<Row>` allocations the panel itself holds (paged history plus what it
        // lays out); the workflow's own live window is shared with it and bounded by the
        // workflow, so it is reported separately instead of being counted twice.
        let (retained_rows, retained_bytes) =
            rows::unique_retained(&[&self.older, &self.view_rows]);
        let bytes: usize = self.view_rows.iter().map(|r| r.estimated_bytes()).sum();
        serde_json::json!({
            "attached": true,
            "revision": snapshot.revision,
            "phase": snapshot.task.as_ref().map(|t| t.phase.label()),
            "repair_used": snapshot.task.as_ref().map(|t| t.repair.used),
            "queue": snapshot.queue.len(),
            "queue_stale": snapshot.queue.iter().filter(|q| q.stale_scope).count(),
            "error_codes": snapshot.rows.iter().filter_map(|r| match &r.kind {
                RowKind::Error(e) => Some(e.code.clone()),
                _ => None,
            }).take(20).collect::<Vec<_>>(),
            "review_policy": format!("{:?}", snapshot.review_policy),
            "list_items": self.list.item_count(),
            "view_rows": self.view_rows.len(),
            "view_bytes": bytes,
            "retained_rows": retained_rows,
            "retained_bytes": retained_bytes,
            "live_rows": snapshot.rows.len(),
            "older_loaded": self.older.len(),
            "hidden_newer": self.hidden_newer,
            "open_permissions": snapshot.resources.open_permissions,
            "owned_processes": snapshot.resources.owned_processes,
            "broker_grants": snapshot.resources.broker_grants,
            "handoff": snapshot.handoff.as_ref().map(|h| format!("{:?}", h.state)),
            "buttons": self.button_bounds,
            "prompt_bounds": self.prompt_bounds,
            "scope_error": self.scope_error,
            "submit_scope": self.task_scope.as_ref().map(|scope| scope.label()),
            "task_scope": snapshot.task.as_ref().map(|t| t.scope.label()),
            "image_limitation": snapshot.task.as_ref().and_then(|t| t.image_limitation.clone()),
            "evidence_preview_count": self.evidence_images.len(),
            "before_evidence": snapshot.task.as_ref().and_then(|t| t.before.as_ref()).map(evidence_telemetry),
            "after_evidence": snapshot.task.as_ref().and_then(|t| t.after.as_ref()).map(evidence_telemetry),
            "undo": match &snapshot.undo {
                UndoView::Available { .. } => "available",
                UndoView::Unavailable { .. } => "unavailable",
                UndoView::InProgress { .. } => "in_progress",
            },
            "setup": self.setup_telemetry(),
            "apply_blocked": snapshot.task.as_ref().and_then(|t| t.review.as_ref()).and_then(|r| r.apply_blocked.clone()),
            "rows": snapshot.rows.iter().skip(snapshot.rows.len().saturating_sub(64)).map(|row| row_telemetry(&row.kind)).collect::<Vec<_>>(),
        })
    }

    /// Called on a workflow thread when a history page finished; the shell then calls
    /// [`Self::poll`].
    pub fn set_wake(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        *self.page.wake.lock() = Some(Arc::new(wake));
    }

    /// Takes the newest snapshot and finished history pages. Cheap; the shell calls it
    /// whenever the workflow changed or a page finished. Returns whether anything changed.
    pub fn poll(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(attached) = &self.attached else {
            return false;
        };
        let mut changed = false;
        let snapshot = attached.workflow.snapshot();
        let serial = attached.serial;
        if self
            .snapshot
            .as_ref()
            .is_none_or(|current| current.revision != snapshot.revision)
        {
            if snapshot.chat != self.chat {
                self.chat = snapshot.chat;
                self.reset_transcript();
            }
            self.snapshot = Some(snapshot);
            self.after_snapshot(cx);
            changed = true;
        }
        changed |= self.sync_evidence_images();
        let finished = self.page.done.lock().take();
        if let Some((page_serial, page_chat, result)) = finished {
            self.page.loading.store(false, Ordering::Release);
            if page_serial == serial && page_chat == self.chat {
                match result {
                    Ok(page) => {
                        let wanted = rows::PAGE_ROWS;
                        self.older_exhausted = page.len() < wanted;
                        rows::prepend_page(&mut self.older, page, ResidentLimits::default());
                        self.recompose(cx);
                        // The next page waits for the reader to scroll again; a stale
                        // "at the top" would otherwise page the whole log in.
                        self.viewport.set(None);
                    }
                    Err(error) => {
                        self.notice = Some(format!("Earlier messages unavailable: {error}"))
                    }
                }
                changed = true;
            }
        }
        self.maybe_request_page();
        if changed {
            cx.notify();
        }
        changed
    }

    fn after_snapshot(&mut self, cx: &mut Context<Self>) {
        self.recompose(cx);
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let controls = derive_controls(&snapshot, self.prompt_is_empty(cx), self.sdk_ready);
        if self.prompt_kind.as_ref() != Some(&controls.submit.kind) {
            let placeholder = match controls.submit.kind {
                SubmitKind::Brief => "Describe the change you want",
                SubmitKind::Queue => "Queue a follow-up; it runs after the current task",
                SubmitKind::Clarify(_) => "Answer the agent's question",
            };
            self.prompt.update(cx, |input, cx| {
                input.set_placeholder(placeholder);
                cx.notify();
            });
            self.prompt_kind = Some(controls.submit.kind);
        }
    }

    /// Copies only decoded UI handles for the current task out of the workflow's bounded
    /// presentation cache, and retires no-longer-visible images for GPUI cleanup.
    fn sync_evidence_images(&mut self) -> bool {
        let mut desired = HashMap::new();
        if let (Some(attached), Some(snapshot)) = (&self.attached, &self.snapshot)
            && let Some(task) = &snapshot.task
        {
            for view in [task.before.as_ref(), task.after.as_ref()]
                .into_iter()
                .flatten()
            {
                for artifact in &view.artifacts {
                    if let Some(image) = attached.workflow.evidence_image(&task.id, &artifact.id) {
                        desired.insert(evidence_image_key(&task.id, &artifact.id), image);
                    }
                }
            }
        }

        let retired: Vec<String> = self
            .evidence_images
            .keys()
            .filter(|key| !desired.contains_key(*key))
            .cloned()
            .collect();
        let mut changed = false;
        for key in retired {
            if let Some((_, image)) = self.evidence_images.remove(&key) {
                self.retired_evidence_images.push(image);
                changed = true;
            }
        }
        for (key, thumbnail) in desired {
            if self
                .evidence_images
                .get(&key)
                .is_none_or(|(current, _)| !Arc::ptr_eq(current, &thumbnail))
            {
                let image = Arc::new(gpui::RenderImage::new(smallvec::SmallVec::from_const([
                    image::Frame::new(thumbnail.as_ref().clone()),
                ])));
                self.evidence_images.insert(key, (thumbnail, image));
                changed = true;
            }
        }
        changed
    }

    fn recompose(&mut self, _cx: &mut Context<Self>) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let view = rows::compose_view(&self.older, &snapshot.rows, ResidentLimits::default());
        let keys: Vec<RowKey> = view.rows.iter().map(RowKey::of).collect();
        for op in rows::diff_rows(&self.view_keys, &keys) {
            match op {
                ListOp::Splice { at, old, new } => self.list.splice(at..at + old, new),
                ListOp::Remeasure { at, len } => self.list.remeasure_items(at..at + len),
                ListOp::Reset { len } => self.list.reset(len),
            }
        }
        self.view_rows = view.rows;
        self.view_keys = keys;
        self.hidden_newer = view.hidden_newer;
    }

    fn log_has_older(&self) -> bool {
        if self.older.is_empty() {
            self.snapshot.as_ref().is_some_and(|s| s.older_rows) && !self.older_exhausted
        } else {
            !self.older_exhausted
        }
    }

    fn maybe_request_page(&mut self) {
        let Some(first_visible) = self.viewport.get() else {
            return;
        };
        if let Some((before, limit)) = rows::page_request(
            &self.view_rows,
            Viewport { first_visible },
            self.log_has_older(),
            self.page.loading.load(Ordering::Acquire),
        ) {
            self.request_page(before, limit);
        }
    }

    fn request_page(&mut self, before: u64, limit: usize) {
        let Some(attached) = &self.attached else {
            return;
        };
        if self.page.loading.swap(true, Ordering::AcqRel) {
            return;
        }
        let slot = self.page.clone();
        let serial = attached.serial;
        let chat = self.chat;
        let started = attached.workflow.history_page_with(
            studio_engine::agent::RowId(before),
            limit,
            move |result| {
                *slot.done.lock() = Some((serial, chat, result.map_err(|e| e.to_string())));
                if let Some(wake) = slot.wake.lock().clone() {
                    wake();
                }
            },
        );
        if let Err(error) = started {
            self.page.loading.store(false, Ordering::Release);
            self.notice = Some(error.to_string());
        }
    }

    fn jump_to_latest(&mut self, cx: &mut Context<Self>) {
        self.older.clear();
        self.older_exhausted = false;
        self.recompose(cx);
        self.list.set_follow_mode(FollowMode::Tail);
        self.list.scroll_to_end();
        cx.notify();
    }

    fn prompt_is_empty(&self, cx: &gpui::App) -> bool {
        self.prompt.read(cx).text().trim().is_empty()
    }

    fn fill_setup(&mut self, adapter: Option<&AdapterFile>, cx: &mut Context<Self>) {
        let fields = adapter.map(AdapterFile::fields).unwrap_or_default();
        let set = |input: &Entity<TextInputState>, text: String, cx: &mut Context<Self>| {
            input.update(cx, |input, cx| input.set_text(text, cx));
        };
        set(&self.setup.provider, fields.provider, cx);
        set(&self.setup.executable, fields.executable, cx);
        set(&self.setup.args, fields.args, cx);
        set(&self.setup.env_names, fields.env_names, cx);
    }

    fn setup_fields(&self, cx: &gpui::App) -> AdapterFields {
        let text = |input: &Entity<TextInputState>| input.read(cx).text().to_owned();
        AdapterFields {
            provider: text(&self.setup.provider),
            executable: text(&self.setup.executable),
            args: text(&self.setup.args),
            env_names: text(&self.setup.env_names),
        }
    }

    // ---- commands ------------------------------------------------------------------------------

    fn say(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.notice = Some(message.into());
        cx.notify();
    }

    fn command<T>(
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

    fn stop(&mut self, cx: &mut Context<Self>) {
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

    fn answer_permission(
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

    /// Saves the Setup fields. The qualification of the adapter (hashing its executable and
    /// reading the ledger) and the file write both run on background threads; at most one
    /// save is in flight (a second click is refused until it completes), the running
    /// workflow is only reconfigured while idle, and only the in-flight save's completion
    /// is reported. The Save button never starts the adapter.
    fn save_adapter(&mut self, cx: &mut Context<Self>) {
        self.save_adapter_with(None, cx);
    }

    /// [`Self::save_adapter`]. With `turn_on` (the profile being turned on and the
    /// description its launch was filled into) the save writes that profile, found by its
    /// id, and runs one connection test once it is written.
    fn save_adapter_with(
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
            let (file, paths) = (file.clone(), paths.clone());
            cx.background_executor()
                .spawn(async move { file.map(|file| file.resolve(&paths, &policy)) })
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
    fn apply_saved(
        &mut self,
        ticket: u64,
        target: Option<String>,
        file: Option<AdapterFile>,
        resolved: Option<(studio_engine::agent::AdapterSettings, Resolution)>,
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
        let (settings, resolution) = match resolved {
            Some((settings, resolution)) => (Some(settings), Some(resolution)),
            None => (None, None),
        };
        let applied = attached.workflow.set_adapter(settings);
        if self.command(applied, cx).is_none() {
            self.saves.finish(ticket);
            self.agent_setup.turn_on.finish(ticket, false);
            return;
        }
        self.saved_adapter = file.clone();
        self.containment = resolution;
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

    fn export(&mut self, kind: Export, cx: &mut Context<Self>) {
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

    /// Key routing for one text field. The composition state is the field's own, read at
    /// the moment of the key: the prompt and every Setup field share this one path.
    fn key_in_field(
        &mut self,
        field: &Entity<TextInputState>,
        event: &gpui::KeyDownEvent,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let composing = field.read(cx).is_composing();
        self.key_in_input(event, composing, submit, window, cx);
    }

    fn key_in_input(
        &mut self,
        event: &gpui::KeyDownEvent,
        composing: bool,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = &event.keystroke.modifiers;
        let mods = KeyMods {
            shift: modifiers.shift,
            alt: modifiers.alt,
            secondary: modifiers.control || modifiers.platform,
        };
        match route_input_key(event.keystroke.key.as_str(), mods, composing) {
            InputKey::Submit => {
                if submit {
                    self.submit(cx);
                }
                cx.stop_propagation();
            }
            InputKey::FocusNext => {
                window.focus_next(cx);
                cx.stop_propagation();
            }
            InputKey::FocusPrev => {
                window.focus_prev(cx);
                cx.stop_propagation();
            }
            InputKey::Leave => {
                cx.emit(PanelEvent::Leave);
                cx.stop_propagation();
            }
            InputKey::Composition => cx.stop_propagation(),
            // Typing, IME text, Space and the arrows belong to the input.
            InputKey::Text => (),
        }
    }

    // ---- building blocks -----------------------------------------------------------------------

    fn activate(
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl Fn(&mut Window, &mut gpui::App) + 'static {
        let entity = cx.entity();
        move |window, app| {
            entity.update(app, |this, cx| action(this, window, cx));
        }
    }

    /// Reports the bounds of the buttons and tabs native qualification clicks.
    fn bounds_probe(id: &ElementId, cx: &mut Context<Self>) -> Option<BoundsProbe> {
        let name = probed_button(id)?.to_owned();
        let entity = cx.entity();
        Some(BoundsProbe::new(move |bounds, cx| {
            entity.update(cx, |panel, _| {
                panel.button_bounds.insert(name.clone(), bounds);
            });
        }))
    }

    fn button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        enabled: bool,
        tone: Tone,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Button {
        let id: ElementId = id.into();
        let probe = Self::bounds_probe(&id, cx);
        Button::new(id, label)
            .variant(match tone {
                Tone::Normal => ButtonVariant::Editor,
                Tone::Primary => ButtonVariant::Primary,
                Tone::Danger => ButtonVariant::Destructive,
            })
            .disabled(!enabled)
            .probe(probe)
            .on_click(Self::activate(cx, action))
    }

    /// An icon-only button; `label` is its tooltip and accessible name.
    fn icon_button(
        &self,
        id: impl Into<ElementId>,
        glyph: Icon,
        label: &'static str,
        variant: ButtonVariant,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> IconButton {
        IconButton::new(id, glyph)
            .size(IconButtonSize::Small)
            .variant(variant)
            .label(label)
            .on_click(Self::activate(cx, action))
    }

    /// One item of a segmented rail: the selected item is highlighted and inert.
    fn segment_button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        selected: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Button {
        let id: ElementId = id.into();
        let probe = Self::bounds_probe(&id, cx);
        let button = Button::new(id, label)
            .variant(ButtonVariant::Ghost)
            .selected(selected)
            .probe(probe);
        if selected {
            button
        } else {
            button.on_click(Self::activate(cx, action))
        }
    }

    fn segment_rail() -> gpui::Div {
        let tokens = design_system::tokens();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .p(px(2.))
            .rounded(px(tokens.metrics.radius_control + 2.))
            .bg(tokens.palette.ctl)
    }

    fn tab_button(
        &self,
        tab: Tab,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.tab == tab;
        self.segment_button(
            format!("agent-tab-{label}"),
            label,
            active,
            cx,
            move |this, _, cx| {
                this.tab = tab;
                cx.notify();
            },
        )
    }

    /// A titled group on a nested card.
    fn section(title: &str) -> gpui::Div {
        let tokens = design_system::tokens();
        ui::card_2()
            .flex()
            .flex_col()
            .gap(px(tokens.metrics.space_2))
            .p(px(tokens.metrics.space_3))
            .child(
                text_style(div(), TextStyle::Label)
                    .text_color(tokens.palette.text_1)
                    .child(title.to_owned()),
            )
    }

    fn line(text: impl Into<SharedString>) -> gpui::Div {
        text_style(div(), TextStyle::Small)
            .text_color(design_system::tokens().palette.text_2)
            .child(text.into())
    }

    // ---- rows ------------------------------------------------------------------------------------

    /// Rows get their spacing as padding inside the measured element, so the list's
    /// height measurement includes it and stays stable while a row streams.
    fn row_shell() -> gpui::Div {
        div()
            .w_full()
            .pb(px(design_system::tokens().metrics.space_2))
    }

    fn caption(text: &str) -> gpui::Div {
        text_style(div(), TextStyle::Small)
            .text_color(design_system::tokens().palette.text_3)
            .child(text.to_owned())
    }

    fn row_id(row: &Row) -> ElementId {
        ElementId::from(("agent-row", row.id.0))
    }

    fn row_element(&mut self, row: &Arc<Row>, cx: &mut Context<Self>) -> AnyElement {
        let palette = design_system::tokens().palette;
        match &row.kind {
            RowKind::User { text, source } => Self::row_shell()
                .child(
                    ui::card_2()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .p(px(10.))
                        .child(Self::caption(match source {
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
                        .child(Self::caption(if *streaming {
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
                        Collapsible::new(("agent-thought", id), open)
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
                                    this.toggle_thought(id, next);
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
    fn toggle_thought(&mut self, id: u64, open: bool) {
        if open {
            self.thoughts_open.insert(id);
        } else {
            self.thoughts_open.remove(&id);
        }
        if let Some(ix) = self.view_rows.iter().position(|row| row.id.0 == id) {
            self.list.remeasure_items(ix..ix + 1);
        }
    }

    fn tool_card(&self, card: &ToolCard) -> AnyElement {
        let palette = design_system::tokens().palette;
        let (status, tone) = tool_badge(card.status);
        Self::row_shell()
            .child(
                ui::card_2()
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
                                ui::label(format!(
                                    "Tool · {}",
                                    card.kind.as_deref().unwrap_or("call")
                                ))
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
                        (card.updates > 1).then(|| Self::line(format!("{} updates", card.updates))),
                    ),
            )
            .into_any_element()
    }

    fn permission_card(&mut self, card: &PermissionCard, cx: &mut Context<Self>) -> AnyElement {
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
                notice = notice.action(self.button(
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
            notice = notice.action(self.button(
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

    fn error_card(&self, row: &Row, error: &StructuredError) -> AnyElement {
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

    fn validation_card(&self, row: &Row, card: &ValidationCard) -> AnyElement {
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

    fn changes_card(&self, card: &ChangeCard) -> AnyElement {
        use studio_engine::candidate_validation::ChangeKind;
        let palette = design_system::tokens().palette;
        let mut body = ui::card_2()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(10.))
            .child(ui::label(format!(
                "{} changed file{}",
                card.total,
                if card.total == 1 { "" } else { "s" }
            )));
        for entry in &card.entries {
            let (mark, color) = match entry.change {
                ChangeKind::Added => ("A", palette.success),
                ChangeKind::Modified => ("M", palette.text_2),
                ChangeKind::Deleted => ("D", palette.danger),
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
            body = body.child(Self::line(format!(
                "…and {} more",
                card.total.saturating_sub(card.entries.len())
            )));
        }
        Self::row_shell().child(body).into_any_element()
    }

    // ---- tabs ----------------------------------------------------------------------------------------

    fn banner(
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

    fn review_buttons(&mut self, review: &ReviewControls, cx: &mut Context<Self>) -> Vec<Button> {
        vec![
            self.button(
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
            self.button(
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
            self.button(
                "agent-export-candidate",
                "Export candidate…",
                true,
                Tone::Normal,
                cx,
                |this, _, cx| this.export(Export::Candidate, cx),
            ),
        ]
    }

    fn conflict_block(&self, conflict: &ConflictView) -> gpui::Div {
        let list = |title: &str, paths: &[String]| {
            let mut block = div().flex().flex_col();
            if !paths.is_empty() {
                block = block.child(Self::line(title.to_owned()));
                for path in paths.iter().take(8) {
                    block = block.child(Self::line(format!("  {path}")));
                }
                if paths.len() > 8 {
                    block = block.child(Self::line(format!("  …and {} more", paths.len() - 8)));
                }
            }
            block
        };
        ui::card_2()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(10.))
            .child(ui::label(format!("Conflict: {}", conflict.reason)))
            .child(list(
                "Changed in the project meanwhile:",
                &conflict.external_paths,
            ))
            .child(list("The candidate changes:", &conflict.candidate_paths))
            .child(list("Changed on both sides:", &conflict.overlapping))
            .children((!conflict.retained_variants.is_empty()).then(|| {
                let mut block = div().flex().flex_col();
                block = block.child(Self::line("Preserved variants (nothing was overwritten):"));
                for variant in conflict.retained_variants.iter().take(8) {
                    block =
                        block.child(Self::line(format!("  {} · {}", variant.path, variant.role)));
                }
                block
            }))
    }

    fn options_bar(
        &mut self,
        snapshot: &WorkflowSnapshot,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Div> {
        if !controls.modes && !controls.config_options {
            return controls
                .options_note
                .as_ref()
                .map(|note| div().child(Self::line(note.clone())));
        }
        let mut bar = div().flex().flex_col().items_start().gap(px(8.));
        if controls.modes {
            let mut rail = Self::segment_rail();
            for mode in &snapshot.options.modes {
                let selected = snapshot.options.current_mode.as_deref() == Some(mode.id.as_str());
                let id = mode.id.clone();
                rail = rail.child(self.segment_button(
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
                    .child(Self::line("Mode"))
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
                        bar =
                            bar.child(div().w_full().child(FieldRow::new(
                                label.clone(),
                                Toggle::new(id, *current).label(label).on_change(
                                    move |_, _, app| {
                                        entity.update(app, |this, cx| {
                                            if let Some(w) = this.workflow().cloned() {
                                                let r =
                                                    w.set_config_option(&option_id, next.clone());
                                                this.command(r, cx);
                                            }
                                        });
                                    },
                                ),
                            )));
                    }
                    ConfigKind::Select { current, values } => {
                        let name = values
                            .iter()
                            .find(|v| &v.value == current)
                            .map_or(current.as_str(), |v| v.name.as_str());
                        let menu = self
                            .config_menus
                            .entry(option.id.clone())
                            .or_insert_with(ConfigMenu::empty);
                        let entity = cx.entity();
                        bar = bar.child(div().w_full().child(FieldRow::new(
                            option.name.clone(),
                            div().min_w(px(160.)).child(
                                Select::new(id, name.to_owned()).menu(&menu.state).on_open(
                                    move |window, app| {
                                        entity.update(app, |this, cx| {
                                            this.toggle_config_menu(&option_id, window, cx);
                                        });
                                    },
                                ),
                            ),
                        )));
                    }
                }
            }
        }
        Some(bar)
    }

    fn toggle_config_menu(&mut self, option_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_config.as_deref() == Some(option_id) {
            self.close_config_menu(window, cx);
            return;
        }
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let Some(menu) = ConfigMenu::of(&snapshot.options.config, option_id) else {
            return;
        };
        // The trigger reports into the old state's anchor until its next paint.
        if let Some(old) = self.config_menus.get(option_id) {
            menu.state.anchor_cell().set(old.state.anchor_cell().get());
        }
        self.config_menus.insert(option_id.to_owned(), menu);
        self.open_config = Some(option_id.to_owned());
        window.focus(&self.config_focus, cx);
        cx.notify();
    }

    fn close_config_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    fn commit_config(
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
    fn sync_config_menu(&mut self) {
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
        if menu.values != fresh.values
            || menu.state.items() != fresh.state.items()
            || menu.state.selected() != fresh.state.selected()
        {
            fresh
                .state
                .anchor_cell()
                .set(menu.state.anchor_cell().get());
            *menu = fresh;
        }
    }

    fn render_config_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
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
            Menu::new("agent-config-menu", &menu.state, &self.config_focus)
                .on_highlight(move |index, _, cx| {
                    highlight.update(cx, |this, cx| {
                        if let Some(menu) = this.config_menus.get_mut(&key) {
                            menu.state.set_highlighted(index);
                        }
                        cx.notify();
                    });
                })
                .on_select(move |index, window, cx| {
                    let value = shown.get(index).cloned().map(OptionValue::Value);
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

    fn chat_tab(
        &mut self,
        snapshot: Arc<WorkflowSnapshot>,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut column = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(12.));
        let g = guidance(
            &snapshot.adapter.readiness,
            snapshot.adapter.provider.is_some(),
        );
        if let Some(banner) = self.banner(&snapshot, controls, cx) {
            column = column.child(banner);
        } else if !g.usable || snapshot.rows.is_empty() {
            column = column.child(
                Self::section(&g.headline)
                    .children(g.steps.iter().take(3).map(|s| Self::line(s.clone()))),
            );
        }
        if let Some(bar) = self.options_bar(&snapshot, controls, cx) {
            column = column.child(bar);
        }
        if let Some(error) = &self.scope_error {
            column = column.child(Self::line(format!("Timeline scope unavailable: {error}")));
        } else if let Some(scope) = &self.task_scope {
            let same_as_task = snapshot
                .task
                .as_ref()
                .filter(|task| !task.phase.is_terminal())
                .is_some_and(|task| task.scope.label() == scope.label());
            if !same_as_task {
                column = column.child(Self::line(format!("Submit scope: {}", scope.label())));
            }
        }
        if let Some(task) = &snapshot.task {
            let label = if task.phase.is_terminal() {
                "Last task scope"
            } else {
                "Frozen task scope"
            };
            column = column.child(Self::line(format!("{label}: {}", task.scope.label())));
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
                                .child(
                                    text_style(div(), TextStyle::Label)
                                        .child(format!("{name} · revision {}", view.revision)),
                                )
                                .child(gpui::img(image.clone()).w(px(128.)).h(px(72.)))
                                .child(
                                    text_style(div(), TextStyle::Label)
                                        .text_color(design_system::tokens().palette.text_2)
                                        .child(format!(
                                            "{} · {} frames{}",
                                            artifact.label,
                                            artifact.frames.len(),
                                            if boundaries.is_empty() {
                                                String::new()
                                            } else {
                                                format!(" · edge {boundaries}")
                                            }
                                        )),
                                ),
                        );
                    } else {
                        column = column.child(Self::line(evidence_summary(name, view)));
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
                column = column.child(Self::line(summary.to_owned()));
            }
        }
        if self.log_has_older() {
            let loading = self.page.loading.load(Ordering::Acquire);
            column = column.child(self.button(
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
                            .rounded(px(design_system::tokens().metrics.radius_control))
                            .bg(design_system::tokens().palette.card)
                            .child(self.button(
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
            let mut queue = Self::section(&format!("Queued ({})", snapshot.queue.len()));
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
                                .child(
                                    text_style(div(), TextStyle::Label).child(item.summary.clone()),
                                )
                                .children(item.scope.as_ref().map(|scope| {
                                    Self::line(format!(
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
                            ButtonVariant::Ghost,
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
        column
            .child(self.prompt_form(controls, cx))
            .into_any_element()
    }

    fn prompt_form(&mut self, controls: &Controls, cx: &mut Context<Self>) -> gpui::Div {
        let tokens = design_system::tokens();
        let enabled = controls.submit.blocked.is_none();
        let label = controls.submit.label();
        let hint = controls.submit.blocked.clone();
        let entity = cx.entity();
        // An icon button: the submit label is its tooltip and accessible name.
        let send = IconButton::new("agent-send", Icon::ArrowUp)
            .size(IconButtonSize::Small)
            .variant(ButtonVariant::Primary)
            .round(true)
            .label(label)
            .disabled(!enabled)
            .on_click(Self::activate(cx, |this, _, cx| this.submit(cx)))
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
            .rounded(px(tokens.metrics.radius_card))
            .bg(tokens.palette.ctl)
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
                                    panel.prompt_bounds = Some(ui::bounds_to_record(bounds));
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
            .children(hint.map(Self::line))
    }

    fn stop_button(&mut self, cx: &mut Context<Self>) -> IconButton {
        self.icon_button(
            "agent-stop",
            Icon::Square,
            "Stop",
            ButtonVariant::Destructive,
            cx,
            |this, _, cx| this.stop(cx),
        )
        .round(true)
    }

    fn project_tab(
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
        let mut review = Self::section("Review")
            .child(text_style(div(), TextStyle::Label).child(match policy {
                ReviewPolicy::AutoApply => "Validated changes are applied automatically.",
                ReviewPolicy::ManualReview => "Validated changes wait for your Apply or Discard.",
            }))
            .child(Self::line(WorkflowSnapshot::REVIEW_POLICY_NOTICE))
            .child(Self::line(
                "A change affects tasks started afterwards; a running task keeps the policy it started with.",
            ));
        if let Some(task) = snapshot.task.as_ref().filter(|t| !t.phase.is_terminal()) {
            review = review.child(Self::line(format!(
                "The running task uses: {}",
                match task.review_policy {
                    ReviewPolicy::AutoApply => "automatic Apply",
                    ReviewPolicy::ManualReview => "manual review",
                }
            )));
        }
        review = review.child(FieldRow::new(
            "Later tasks",
            self.button(
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
        let mut undo = Self::section("Undo");
        match &controls.undo {
            UndoControl::Enabled { summary, .. } => {
                undo = undo
                    .child(Self::line(format!("Newest agent edit: {summary}")))
                    .child(FieldRow::new(
                        "Undo",
                        self.button(
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
                undo = undo.child(Self::line(reason.clone())).child(FieldRow::new(
                    "Undo",
                    self.button(
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
            undo = undo.child(text_style(div(), TextStyle::Label).child(match &handoff.state {
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
            let mut section = Self::section("Recovery");
            if let Some(reason) = &recovery.suspended {
                section = section.child(text_style(div(), TextStyle::Label).child(format!(
                    "Source changes are suspended until the project is reopened: {reason}"
                )));
            }
            if let Some(notice) = &recovery.notice {
                section = section.child(Self::line(notice.clone()));
            }
            for id in recovery.rolled_back.iter().take(5) {
                section = section.child(Self::line(format!(
                    "An unfinished edit ({}) was rolled back when the project opened; your files are as they were.",
                    &id[..id.len().min(8)]
                )));
            }
            for conflict in &recovery.unresolved_conflicts {
                section = section.child(self.conflict_block(conflict));
            }
            if let Some(draft) = recovery.draft.as_ref().filter(|d| draft_needs_attention(d)) {
                section = section.child(text_style(div(), TextStyle::Label).child(match &draft.state {
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
                section = section.child(Self::line(draft.draft.display().to_string()));
                if controls.acknowledge_writer_gone {
                    section = section
                        .child(Self::line(
                            "Confirm only if you are sure no agent process is still writing there.",
                        ))
                        .child(FieldRow::new(
                            "Writer",
                            self.button(
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
                section = section.child(self.button(
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
        let mut history = Self::section(&format!(
            "Accepted agent edits ({})",
            snapshot.history.len()
        ));
        if snapshot.history.is_empty() {
            history = history.child(Self::line(
                "No agent edit has been accepted in this project yet.",
            ));
        }
        for entry in snapshot.history.iter().rev().take(10) {
            history = history.child(history_line(entry));
        }
        column = column.child(history);

        // Resources.
        let r = snapshot.resources;
        column = column.child(Self::line(format!(
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

    fn setup_tab(
        &mut self,
        snapshot: Arc<WorkflowSnapshot>,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let configured = snapshot.adapter.provider.is_some();
        let g = guidance(&snapshot.adapter.readiness, configured);
        let mut column = div()
            .id("agent-setup-tab")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(12.));
        column = column.child(
            Self::section(&g.headline).children(g.steps.iter().map(|s| Self::line(s.clone()))),
        );
        if let Some(error) = &self.settings_error {
            column = column.child(Notice::new(
                "agent-settings-error",
                UiTone::Danger,
                error.clone(),
            ));
        }
        let switch_pending = snapshot.switch_pending.is_some();
        let selection_busy = snapshot
            .task
            .as_ref()
            .is_some_and(|task| !task.phase.is_terminal())
            || matches!(&snapshot.undo, UndoView::InProgress { .. });
        column = column.child(self.providers_section(switch_pending, selection_busy, cx));
        column = column.child(self.detect_section(
            &snapshot.adapter.readiness,
            controls.settings_editable,
            cx,
        ));

        if let Some(choice) = &controls.switch_choice {
            let mut handoff = Self::section("Stopped-writer handoff")
                .child(Self::line(choice.note.clone()))
                .child(Self::line(format!(
                    "Outgoing task: {}",
                    snapshot
                        .task
                        .as_ref()
                        .map(|task| task.brief.as_str())
                        .unwrap_or("no active task")
                )))
                .child(Self::line(format!(
                    "Draft revision: {} · current source: {} · accepted: {}",
                    choice.draft_revision.as_deref().unwrap_or("unavailable"),
                    choice.source_revision,
                    choice.accepted_revision
                )));
            if choice.retained_queue_count > 0 {
                handoff = handoff.child(Self::line(format!(
                    "{} queued brief(s) remain with the outgoing provider until you explicitly transfer or clear them.",
                    choice.retained_queue_count
                )));
            }
            handoff = handoff.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.button(
                        "agent-handoff-continue-draft",
                        "Continue captured draft",
                        true,
                        Tone::Primary,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(
                                    workflow.choose_handoff_continuation(
                                        studio_engine::agent::model::HandoffChoice::ContinueDraft,
                                    ),
                                    cx,
                                );
                            }
                        },
                    ))
                    .child(self.button(
                        "agent-handoff-restart-accepted",
                        "Restart from accepted",
                        true,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(
                                    workflow.choose_handoff_continuation(
                                        studio_engine::agent::model::HandoffChoice::RestartFromAccepted,
                                    ),
                                    cx,
                                );
                            }
                        },
                    ))
                    .child(self.button(
                        "agent-handoff-cancel",
                        "Cancel switch",
                        true,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(workflow.cancel_switch(), cx);
                            }
                        },
                    )),
            );
            column = column.child(handoff);
        }
        if controls.retained_queue.is_some() {
            let mut retained = Self::section("Queued work held for the previous provider");
            for brief in &snapshot.retained_queue {
                retained = retained.child(Self::line(format!("• {}", brief.summary)));
            }
            retained = retained.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.button(
                        "agent-transfer-retained-queue",
                        "Explicitly transfer queued briefs",
                        !selection_busy && !switch_pending,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(workflow.transfer_retained_queue(), cx);
                            }
                        },
                    ))
                    .child(self.button(
                        "agent-clear-retained-queue",
                        "Clear held briefs",
                        !selection_busy && !switch_pending,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(workflow.clear_retained_queue(), cx);
                            }
                        },
                    )),
            );
            column = column.child(retained);
        }
        if let Some(restore) = &controls.session_restore {
            let mut session =
                Self::section("Previous provider session").child(Self::line(format!(
                    "{} · session {} · {}",
                    restore.provider_id,
                    restore.redacted_session_id,
                    restore
                        .notice
                        .as_deref()
                        .unwrap_or("Native conversation memory stays with its provider.")
                )));
            if restore.is_resumable {
                session = session.child(self.button(
                    "agent-restore-session",
                    "Use restored session for next task",
                    !selection_busy && !switch_pending,
                    Tone::Normal,
                    cx,
                    |this, _, cx| {
                        if let Some(workflow) = this.workflow().cloned() {
                            this.command(workflow.restore_session(), cx);
                        }
                    },
                ));
            }
            session = session.child(self.button(
                "agent-dismiss-session",
                "Dismiss saved session",
                !selection_busy && !switch_pending,
                Tone::Normal,
                cx,
                |this, _, cx| {
                    if let Some(workflow) = this.workflow().cloned() {
                        this.command(workflow.dismiss_restoration(), cx);
                    }
                },
            ));
            column = column.child(session);
        }
        let field = |this: &Self,
                     label: &'static str,
                     input: &Entity<TextInputState>,
                     cx: &mut Context<Self>| {
            // The handler reads THIS field's composition state: an active IME composition
            // keeps Enter/Tab/Escape (candidate selection and cancellation) in the field.
            let field = input.clone();
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(Self::line(label))
                .child(
                    div()
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                this.key_in_field(&field, event, false, window, cx);
                            },
                        ))
                        .child(TextInput::plain(
                            &this.theme,
                            SharedString::from(format!("setup-{label}")),
                            input,
                        )),
                )
        };
        let editable = controls.settings_editable;
        let mut form = Self::section("Adapter");
        form = form
            .child(field(self, "Label", &self.setup.provider.clone(), cx))
            .child(field(
                self,
                "Executable",
                &self.setup.executable.clone(),
                cx,
            ))
            .child(field(self, "Arguments", &self.setup.args.clone(), cx))
            .child(field(
                self,
                "Sign-in variable names (values are never stored)",
                &self.setup.env_names.clone(),
                cx,
            ));
        form = form
            .child(Self::line(
                "Writer containment is not a setting: it is derived from the qualification ledger installed in the app data folder, and only for this exact adapter and platform.",
            ))
            .child(Self::line(
                self.containment.as_ref().map_or_else(
                    || "Writer containment: no adapter configured.".to_owned(),
                    Resolution::summary,
                ),
            ))
            .child({
                let entity = cx.entity();
                let label = match self.mcp_choice {
                    McpChoice::Baseline => "Project tools: MCP and command line",
                    McpChoice::Unsupported => "Project tools: command line only",
                };
                FieldRow::new(
                    label,
                    Toggle::new("agent-mcp-toggle", self.mcp_choice == McpChoice::Baseline)
                        .label(label)
                        .disabled(!editable)
                        .on_change(move |_, _, app| {
                            entity.update(app, |this, cx| {
                                this.mcp_choice = match this.mcp_choice {
                                    McpChoice::Baseline => McpChoice::Unsupported,
                                    McpChoice::Unsupported => McpChoice::Baseline,
                                };
                                cx.notify();
                            });
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.button(
                        "agent-save-adapter",
                        "Save",
                        editable && !self.saves.is_pending(),
                        Tone::Primary,
                        cx,
                        |this, _, cx| this.save_adapter(cx),
                    ))
                    .child(self.button(
                        "agent-check-adapter",
                        "Check adapter",
                        controls.check_adapter,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(w) = this.workflow().cloned() {
                                let r = w.check_adapter();
                                this.command(r, cx);
                            }
                        },
                    )),
            );
        if !editable {
            form = form.child(Self::line(
                "Settings can change after the running task finishes or is stopped.",
            ));
        }
        column = column.child(form);

        let mut status = Self::section("Status");
        status = status
            .child(Self::line(format!(
                "Adapter: {}",
                snapshot
                    .adapter
                    .executable
                    .clone()
                    .unwrap_or_else(|| "not configured".into())
            )))
            .child(Self::line(format!(
                "Writer process model: {}",
                snapshot
                    .adapter
                    .writer_ownership
                    .clone()
                    .unwrap_or_else(|| "n/a".into())
            )));
        if snapshot
            .adapter
            .writer_ownership
            .as_deref()
            .is_some_and(|label| label != "process-group-contained")
        {
            status = status.child(Self::line(
                "This adapter is not qualified for this exact launch and platform, so its candidates cannot be captured or applied until a passed writer-containment qualification of it is installed.",
            ));
        }
        if let AdapterReadiness::Checked { report, .. } = &snapshot.adapter.readiness
            && let Some(info) = &report.initialized
        {
            status = status.child(Self::line(format!(
                "Adapter reports {} {} · ACP v{}",
                info.agent_name, info.agent_version, info.protocol_version
            )));
        }
        if let Some(caps) = &snapshot.capabilities {
            let mut unsupported = Vec::new();
            if !caps.prompt_image {
                unsupported.push("images");
            }
            if !caps.prompt_audio {
                unsupported.push("audio");
            }
            if !caps.load_session && !caps.resume_session {
                unsupported.push("resuming a session");
            }
            if !unsupported.is_empty() {
                status = status.child(Self::line(format!(
                    "Not supported by this agent (prompts are text only): {}",
                    unsupported.join(", ")
                )));
            }
        }
        let mcp = &snapshot.mcp;
        status = status.child(Self::line(format!(
            "Project tools: MCP {} · command line {}{}",
            if mcp.policy_enabled && mcp.binary_available {
                "offered"
            } else {
                "not offered"
            },
            if mcp.cli_active {
                "offered"
            } else {
                "when a task runs"
            },
            mcp.note
                .as_ref()
                .map(|n| format!(" · {n}"))
                .unwrap_or_default()
        )));
        column = column.child(status);
        column.into_any_element()
    }
}

/// The buttons whose bounds native qualification reads.
fn probed_button(id: &ElementId) -> Option<&str> {
    match id {
        ElementId::Name(name)
            if matches!(
                name.as_ref(),
                "agent-policy"
                    | "agent-apply"
                    | "agent-undo"
                    | "agent-tab-Chat"
                    | "agent-tab-Project"
            ) =>
        {
            Some(name.as_ref())
        }
        _ => None,
    }
}

fn banner_tone(phase: TaskPhase) -> UiTone {
    match phase {
        TaskPhase::Accepted => UiTone::Success,
        TaskPhase::Failed | TaskPhase::Conflict => UiTone::Danger,
        TaskPhase::AwaitingReview
        | TaskPhase::AwaitingEvidence
        | TaskPhase::WaitingPermission
        | TaskPhase::WaitingClarification
        | TaskPhase::Interrupted => UiTone::Warning,
        _ => UiTone::Info,
    }
}

fn outcome_style(kind: OutcomeKind) -> (UiTone, &'static str) {
    match kind {
        OutcomeKind::Accepted => (UiTone::Success, "Accepted"),
        OutcomeKind::Undone => (UiTone::Success, "Undone"),
        OutcomeKind::AwaitingReview => (UiTone::Warning, "Waiting for your review"),
        OutcomeKind::Conflict => (UiTone::Danger, "Conflict"),
        OutcomeKind::Failed => (UiTone::Danger, "Failed"),
        OutcomeKind::Cancelled => (UiTone::Neutral, "Stopped"),
        OutcomeKind::Interrupted => (UiTone::Warning, "Interrupted"),
        OutcomeKind::Discarded => (UiTone::Neutral, "Discarded"),
    }
}

fn tool_badge(status: Option<ToolStatus>) -> (&'static str, UiTone) {
    match status {
        Some(ToolStatus::Pending) => ("pending", UiTone::Neutral),
        Some(ToolStatus::InProgress) => ("running", UiTone::Info),
        Some(ToolStatus::Completed) => ("done", UiTone::Success),
        Some(ToolStatus::Failed) => ("failed", UiTone::Danger),
        None => ("…", UiTone::Neutral),
    }
}

/// Only a locked or retained working copy needs the user's attention; one whose bytes
/// were published (or that a running task owns) is routine.
fn draft_needs_attention(draft: &studio_engine::agent::DraftNotice) -> bool {
    matches!(
        draft.state,
        DraftState::UnsafeWriter { .. } | DraftState::Retained { .. }
    )
}

fn history_line(entry: &HistoryEntry) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .child(text_style(div(), TextStyle::Label).child(format!(
            "{} · {} file{} · {}{}",
            entry.kind,
            entry.files,
            if entry.files == 1 { "" } else { "s" },
            &entry.published[..entry.published.len().min(12)],
            entry
                .undoes
                .as_ref()
                .map(|u| format!(" · undoes {}", &u[..u.len().min(8)]))
                .unwrap_or_default()
        )))
        .child(ConversationPanel::line(entry.summary.clone()))
}

impl Render for ConversationPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme.refresh(window, cx);
        self.sync_evidence_images();
        for image in self.retired_evidence_images.drain(..) {
            let _ = window.drop_image(image);
        }
        let tokens = design_system::tokens();
        let mut root = div()
            .id("agent-panel")
            .role(gpui::Role::Group)
            .aria_label("Agent")
            .size_full()
            .flex()
            .flex_col()
            .text_color(tokens.palette.text_1);
        let Some(snapshot) = self.snapshot.clone() else {
            return root
                .p(px(tokens.metrics.space_4))
                .gap(px(tokens.metrics.space_2))
                .child(ui::heading("Agent"))
                .child(Self::line(
                    "Open or create a project to work with an agent.",
                ))
                .child(Self::line(controls::NO_AGENT_LAUNCHED))
                .into_any_element();
        };
        let controls = derive_controls(&snapshot, self.prompt_is_empty(cx), self.sdk_ready);
        let g = guidance(
            &snapshot.adapter.readiness,
            snapshot.adapter.provider.is_some(),
        );
        self.sync_config_menu();
        let rail = Self::segment_rail()
            .child(self.tab_button(Tab::Chat, "Chat", cx))
            .child(self.tab_button(Tab::Project, "Project", cx))
            .child(self.tab_button(Tab::Setup, "Setup", cx));
        let new_chat = self
            .icon_button(
                "agent-new-chat",
                Icon::Plus,
                "New chat",
                ButtonVariant::Ghost,
                cx,
                |this, _, cx| {
                    if let Some(w) = this.workflow().cloned() {
                        let r = w.new_chat();
                        this.command(r, cx);
                    }
                    this.tab = Tab::Chat;
                    cx.notify();
                },
            )
            .disabled(!controls.new_chat);
        let mut header = div()
            .flex()
            .flex_col()
            .flex_none()
            .border_b_1()
            .border_color(tokens.palette.line)
            .child(
                div()
                    .h(px(HEADER_H))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.))
                    .child(
                        text_style(div(), TextStyle::Title)
                            .text_color(tokens.palette.text_1)
                            .child("Agent"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(new_chat)
                            .child(rail),
                    ),
            )
            .child(
                text_style(div(), TextStyle::Small)
                    .px(px(12.))
                    .pb(px(8.))
                    .text_color(if g.usable {
                        tokens.palette.text_2
                    } else {
                        tokens.palette.accent
                    })
                    .child(g.headline.clone()),
            );
        if let Some(notice) = self.notice.clone() {
            header = header.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(4.))
                    .px(px(8.))
                    .pb(px(8.))
                    .child(div().flex_1().min_w_0().child(Notice::new(
                        "agent-notice",
                        UiTone::Warning,
                        notice,
                    )))
                    .child(self.icon_button(
                        "agent-dismiss-notice",
                        Icon::X,
                        "×",
                        ButtonVariant::Ghost,
                        cx,
                        |this, _, cx| {
                            this.notice = None;
                            cx.notify();
                        },
                    )),
            );
        }
        root = root.child(header);
        let body = match self.tab {
            Tab::Chat => self.chat_tab(snapshot, &controls, cx),
            Tab::Project => self.project_tab(snapshot, &controls, cx),
            Tab::Setup => self.setup_tab(snapshot, &controls, cx),
        };
        root.child(body)
            .children(self.render_config_menu(cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;

    #[test]
    fn only_policy_and_apply_report_bounds() {
        assert_eq!(probed_button(&"agent-apply".into()), Some("agent-apply"));
        assert_eq!(probed_button(&"agent-policy".into()), Some("agent-policy"));
        assert_eq!(probed_button(&"agent-undo".into()), Some("agent-undo"));
        assert_eq!(
            probed_button(&"agent-tab-Project".into()),
            Some("agent-tab-Project")
        );
        assert_eq!(probed_button(&"agent-tab-Setup".into()), None);
        assert_eq!(probed_button(&"agent-discard".into()), None);
        assert_eq!(
            probed_button(&ElementId::from(("agent-apply", 1usize))),
            None
        );
    }

    #[test]
    fn a_config_menu_lists_the_values_and_commits_the_chosen_one() {
        use studio_harness::driver::{ConfigOption, ConfigValue};
        let value = |value: &str, name: &str| ConfigValue {
            value: value.into(),
            name: name.into(),
            group: None,
        };
        let config = vec![
            ConfigOption {
                id: "model".into(),
                name: "Model".into(),
                description: None,
                kind: ConfigKind::Select {
                    current: "sonnet".into(),
                    values: vec![value("opus", "Opus"), value("sonnet", "Sonnet")],
                },
            },
            ConfigOption {
                id: "fast".into(),
                name: "Fast".into(),
                description: None,
                kind: ConfigKind::Boolean { current: false },
            },
        ];
        let menu = ConfigMenu::of(&config, "model").unwrap();
        let items: Vec<&str> = menu.state.items().iter().map(|s| s.as_ref()).collect();
        assert_eq!(items, ["Opus", "Sonnet"]);
        assert_eq!(menu.state.selected(), Some(1));
        assert_eq!(menu.values, ["opus", "sonnet"]);
        assert!(ConfigMenu::of(&config, "fast").is_none());
        assert!(ConfigMenu::of(&config, "gone").is_none());
    }

    #[test]
    fn a_config_menu_commits_the_value_it_showed_after_the_options_change() {
        use studio_harness::driver::{ConfigOption, ConfigValue};
        let select = |values: &[(&str, &str)]| {
            vec![ConfigOption {
                id: "model".into(),
                name: "Model".into(),
                description: None,
                kind: ConfigKind::Select {
                    current: values[0].0.into(),
                    values: values
                        .iter()
                        .map(|(value, name)| ConfigValue {
                            value: (*value).into(),
                            name: (*name).into(),
                            group: None,
                        })
                        .collect(),
                },
            }]
        };
        let shown =
            ConfigMenu::of(&select(&[("opus", "Opus"), ("sonnet", "Sonnet")]), "model").unwrap();
        // The agent reorders its options between the paint and the click.
        let now =
            ConfigMenu::of(&select(&[("sonnet", "Sonnet"), ("opus", "Opus")]), "model").unwrap();
        // Each menu pairs its names with their values row for row, so the row the reader
        // clicked commits the value they saw.
        assert_eq!(shown.state.items()[1].as_ref(), "Sonnet");
        assert_eq!(shown.values[1], "sonnet");
        assert_eq!(now.state.items()[1].as_ref(), "Opus");
        assert_eq!(now.values[1], "opus");
    }

    #[test]
    fn banner_tone_follows_the_task_phase() {
        assert_eq!(banner_tone(TaskPhase::Accepted), UiTone::Success);
        assert_eq!(banner_tone(TaskPhase::Failed), UiTone::Danger);
        assert_eq!(banner_tone(TaskPhase::Conflict), UiTone::Danger);
        assert_eq!(banner_tone(TaskPhase::AwaitingReview), UiTone::Warning);
        assert_eq!(banner_tone(TaskPhase::WaitingPermission), UiTone::Warning);
    }

    #[test]
    fn outcomes_keep_their_titles_and_tones() {
        assert_eq!(
            outcome_style(OutcomeKind::Accepted),
            (UiTone::Success, "Accepted")
        );
        assert_eq!(
            outcome_style(OutcomeKind::Conflict),
            (UiTone::Danger, "Conflict")
        );
        assert_eq!(
            outcome_style(OutcomeKind::Cancelled),
            (UiTone::Neutral, "Stopped")
        );
        assert_eq!(
            outcome_style(OutcomeKind::AwaitingReview),
            (UiTone::Warning, "Waiting for your review")
        );
    }

    #[test]
    fn tool_status_badges_keep_their_words() {
        assert_eq!(
            tool_badge(Some(ToolStatus::InProgress)),
            ("running", UiTone::Info)
        );
        assert_eq!(
            tool_badge(Some(ToolStatus::Completed)),
            ("done", UiTone::Success)
        );
        assert_eq!(
            tool_badge(Some(ToolStatus::Failed)),
            ("failed", UiTone::Danger)
        );
        assert_eq!(tool_badge(None), ("…", UiTone::Neutral));
    }
}
