mod engine_events;
mod header;
mod keymap;
mod layout;
mod left_column;
mod overflow;
mod player;
mod qualification;
mod status_strip;
mod transport;

use gpui::{AppContext, Context, Window};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use studio_engine::agent::AgentWorkflow;
use studio_engine::containment::OwnershipPolicy;
use studio_engine::engine::backend::StudioEnvironment;
use studio_engine::engine::backend::{Command, Presentation};
use studio_engine::engine::handle::EngineHandle;
use studio_engine::providers::host::HostInbox;
use studio_engine::services::audio_service::{AudioService, OutputDevice, OutputHandle};
use studio_engine::services::build_service::BuildService;
use studio_engine::services::export_service::{ExportControl, ExportProgress};
use studio_engine::services::preview_coordinator::PreviewCoordinator;
use studio_engine::services::thumbnail_cache::ThumbnailKey;
use studio_project::ProjectId;
use studio_proto::CommandId;
use studio_theme::Theme;

use crate::app_state::{AppEvent, AppState};
use crate::kit::{EditorButton, Icon, IconButton, IconButtonSize, MenuState};
use crate::panels::conversation::{ConversationPanel, PanelEvent};
use crate::{
    canvas_view::CanvasViewState,
    frame_image::ImagePresentationManager,
    preset_panel::PresetPanel,
    timeline_view::{TimelineEvent, TimelineView},
};
use left_column::LeftTab;

enum Picker {
    Create,
    Restore,
    Open,
    Import,
    Asset,
    Sdk,
    SdkBundle,
    Locate(ProjectId),
    PresetImport,
    PresetCss,
    PresetExport(String),
    Export,
}

struct PreparedInstall {
    ready: Arc<studio_engine::ReadyPreview>,
    clock: studio_engine::PlaybackClock,
    audio: Option<OutputHandle>,
    audio_ready: bool,
}

pub struct StudioShell {
    /// The theme of the window, refreshed at the start of every render.
    theme: Theme,
    /// The running app's version, which SDKs must be compatible with.
    app_version: &'static str,
    engine: EngineHandle,
    app: gpui::Entity<AppState>,
    _app_subscription: gpui::Subscription,
    /// The environment's build service, read here so the UI thread never waits on the
    /// engine for it.
    builds: BuildService,
    closed: Arc<AtomicBool>,
    focus: gpui::FocusHandle,
    presentation: Presentation,
    /// A command is in flight or a native picker is open.
    busy: bool,
    /// The command in flight and whether it is a refresh (which keeps the error shown).
    in_flight: Option<(CommandId, bool)>,
    error: Option<String>,
    failed_open: Option<PathBuf>,
    left_tab: LeftTab,
    sdk_open: bool,
    recents_open: bool,
    status_open: bool,
    preview: Option<Arc<PreviewCoordinator>>,
    preview_session: Option<studio_engine::OpenSession>,
    preview_state: studio_engine::PreviewState,
    displayed: Option<Arc<studio_engine::ReadyPreview>>,
    pending_ready: Option<PreparedInstall>,
    pending_frame: Option<studio_engine::PreviewFrame>,
    canvas: CanvasViewState,
    /// The preview surface's focus, which its keys need.
    preview_focus: gpui::FocusHandle,
    canvas_origin: (f64, f64),
    canvas_extent: (f64, f64),
    canvas_pan_position: Option<(f64, f64)>,
    images: ImagePresentationManager,
    preview_extent: (u32, u32),
    clear_images: bool,
    preview_gate_pending: bool,
    /// Woken by the preview coordinator when it has news.
    preview_wake: async_channel::Sender<()>,
    /// The thumbnails last asked of the coordinator; `None` asks again.
    thumbnails_sent: Option<Vec<ThumbnailKey>>,
    timeline: gpui::Entity<TimelineView>,
    _timeline_subscription: gpui::Subscription,
    transport: studio_engine::PlaybackClock,
    audio: Arc<AudioService>,
    output: Option<OutputHandle>,
    audio_ready_epoch: Option<u64>,
    audio_note: String,
    output_device: OutputDevice,
    muted: bool,
    resume_install: bool,
    requested_clock_epoch: u64,
    presented_serial: u64,
    clock_start: Instant,
    resume_scrub: bool,
    qualifying: bool,
    button_bounds: HashMap<String, [f32; 4]>,
    player_width: f32,
    overflow_open: bool,
    overflow_menu: MenuState,
    /// Where the overflow trigger was last painted; the menu opens against it.
    overflow_anchor: std::rc::Rc<std::cell::Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
    overflow_focus: gpui::FocusHandle,
    painted_frame: Option<usize>,
    /// Set while the accepted source is ahead of what the preview shows: a handoff that
    /// failed (or has not finished) keeps the old preview playing under this label.
    awaiting_preview: Option<String>,
    panel: gpui::Entity<ConversationPanel>,
    _panel_subscription: gpui::Subscription,
    preset_panel: gpui::Entity<PresetPanel>,
    _preset_subscription: gpui::Subscription,
    agent: AgentHost,
    /// A promotion changed the source: take a fresh presentation (without cancelling the
    /// staged preview that was just adopted for exactly that source).
    refresh_wanted: bool,
    presentation_refresh_pending: bool,
    /// When the accepted source started waiting for a preview that nothing is preparing.
    awaiting_since: Option<Instant>,
    export_status: Option<ExportProgress>,
    export_cancelled: Option<Arc<ExportControl>>,
    export_processes: Option<studio_bootstrap::ProcessTreeManager>,
}

/// The shell's side of the per-project agent workflow: which project it belongs to, how
/// it was opened and what must still be reported to it. Workflow threads reach the UI only
/// through [`HostInbox`], which the frame loop drains.
struct AgentHost {
    inbox: Arc<HostInbox>,
    workflow: Option<Arc<AgentWorkflow>>,
    session: Option<studio_engine::OpenSession>,
    /// Serial of the open in flight, if any; a result with another serial is closed.
    opening: Option<u64>,
    /// Serial of the workflow currently attached (hand-offs carry it).
    attached: Option<u64>,
    serial: u64,
    /// The SDK directory the workflow's build settings were last set to.
    applied_sdk: Option<PathBuf>,
    playhead: usize,
    /// A revision the preview now displays, still to be reported.
    displayed: Option<String>,
    /// Who may say a writer is contained: validated qualification evidence only, except
    /// for the explicit test injection of the `qualify-m3` observation entry.
    ownership: OwnershipPolicy,
}

impl AgentHost {
    fn new(ownership: OwnershipPolicy) -> Self {
        Self {
            inbox: Arc::new(HostInbox::default()),
            workflow: None,
            session: None,
            opening: None,
            attached: None,
            serial: 0,
            applied_sdk: None,
            playhead: 0,
            displayed: None,
            ownership,
        }
    }
}

impl StudioShell {
    /// `app_version` is the running app's version, which SDKs must be compatible with.
    pub fn new(app_version: &'static str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_environment(StudioEnvironment::system(), app_version, window, cx)
    }

    pub fn with_environment(
        environment: StudioEnvironment,
        app_version: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle().tab_stop(false);
        window.focus(&focus, cx);
        let timeline = cx.new(TimelineView::new);
        let timeline_subscription =
            cx.subscribe(&timeline, |shell, _, event, cx| match event.clone() {
                TimelineEvent::Seek(frame) => shell.seek_preview(frame, cx),
                TimelineEvent::Step(delta) => shell.step_preview(delta, cx),
                TimelineEvent::TogglePlayback => shell.toggle_playback(cx),
                TimelineEvent::ToggleMute => shell.toggle_mute(cx),
                TimelineEvent::BeginScrub => {
                    shell.resume_scrub = shell.transport.playing() || shell.resume_install;
                    shell.resume_install = false;
                    shell.pause_playback(cx);
                }
                TimelineEvent::EndScrub => {
                    if shell.resume_scrub
                        && !shell.transport.playing()
                        && shell
                            .displayed
                            .as_ref()
                            .is_some_and(|d| shell.transport.position() < d.timeline.total_frames)
                    {
                        shell.toggle_playback(cx);
                    }
                    shell.resume_scrub = false;
                }
                TimelineEvent::ScopeChanged(scope) => {
                    let scope = scope.map(|scope| scope.map(|scope| *scope));
                    shell.install_task_scope(scope, cx);
                }
            });
        let panel = cx.new(|cx| ConversationPanel::new(window, cx));
        let panel_subscription =
            cx.subscribe_in(&panel, window, |shell, _, event, window, cx| match event {
                PanelEvent::Leave => window.focus(&shell.focus, cx),
            });
        let preset_panel = cx.new(|cx| PresetPanel::new(window, cx));
        let preset_subscription =
            cx.subscribe_in(&preset_panel, window, |shell, _, event, window, cx| {
                shell.preset_event(event.clone(), window, cx)
            });
        let (engine, events) = EngineHandle::new(environment.clone(), app_version);
        let app = cx.new(|cx| AppState::new(engine.clone(), events, cx));
        let app_subscription = cx.subscribe(&app, |shell, _, event: &AppEvent, cx| {
            if let Some(event) = event.take() {
                shell.engine_event(event, cx);
            }
        });
        let closed = engine.closed().clone();
        let owner = engine.clone();
        let shutdown = closed.clone();
        let clock_start = Instant::now();
        let audio = Arc::new(AudioService::new(clock_start));
        let quit_audio = audio.clone();
        let builds = environment.builds.clone();
        cx.on_app_quit(move |shell, cx| {
            shutdown.store(true, Ordering::Release);
            shell.cancel_export(cx);
            quit_audio.shutdown();
            let audio = quit_audio.clone();
            let preview = shell.preview.take();
            // The workflow reaps its task, broker, compilers and workers before the
            // controller they use is closed.
            let workflow = shell.agent.workflow.take();
            let displayed = shell.displayed.take();
            let pending = shell.pending_ready.take();
            shell.output = None;
            let owner = owner.clone();
            let builds = builds.clone();
            cx.background_executor().spawn(async move {
                if let Some(workflow) = workflow {
                    workflow.close();
                }
                owner.processes().shutdown(Duration::ZERO);
                // Compiles are owned by the build service, not the app process scope.
                builds.close();
                audio.join();
                if let Some(preview) = preview {
                    preview.close();
                    drop(preview);
                }
                drop((displayed, pending));
                // A hash/copy may still own the serialized queue. Do not wait
                // unboundedly on it during quit; durable jobs recover as interrupted.
                owner.shutdown(Duration::from_millis(50));
            })
        })
        .detach();
        let agent = AgentHost::new(environment.ownership.clone());
        let (agent_wake, agent_wakes) = async_channel::bounded(1);
        agent.inbox.set_listener(move || {
            let _ = agent_wake.try_send(());
        });
        let inbox = agent.inbox.clone();
        panel.update(cx, |panel, _| panel.set_wake(move || inbox.wake()));
        cx.spawn(async move |this, cx| {
            while agent_wakes.recv().await.is_ok() {
                if this.update(cx, |shell, cx| shell.agent_woken(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let (preview_wake, preview_wakes) = async_channel::bounded(1);
        cx.spawn(async move |this, cx| {
            while preview_wakes.recv().await.is_ok() {
                if this
                    .update(cx, |shell, cx| shell.drain_preview(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let mut shell = Self {
            theme: Theme::for_window(window, cx),
            app_version,
            engine,
            app,
            _app_subscription: app_subscription,
            builds: environment.builds.clone(),
            closed,
            focus,
            presentation: Presentation::default(),
            busy: false,
            in_flight: None,
            error: None,
            failed_open: None,
            left_tab: LeftTab::Project,
            sdk_open: true,
            recents_open: true,
            status_open: false,
            preview: None,
            preview_session: None,
            preview_state: studio_engine::PreviewState::default(),
            displayed: None,
            pending_ready: None,
            pending_frame: None,
            canvas: CanvasViewState::default(),
            preview_focus: cx.focus_handle().tab_index(0).tab_stop(true),
            canvas_origin: (0.0, 0.0),
            canvas_extent: (0.0, 0.0),
            canvas_pan_position: None,
            images: ImagePresentationManager::new(),
            preview_extent: (1280, 720),
            clear_images: false,
            preview_gate_pending: false,
            preview_wake,
            thumbnails_sent: None,
            timeline,
            _timeline_subscription: timeline_subscription,
            transport: studio_engine::PlaybackClock::install(30, 0, 0, 0.)
                .expect("valid empty clock"),
            audio,
            output: None,
            audio_ready_epoch: None,
            audio_note: "Audio output not prepared".into(),
            awaiting_preview: None,
            panel,
            _panel_subscription: panel_subscription,
            preset_panel,
            _preset_subscription: preset_subscription,
            agent,
            refresh_wanted: false,
            presentation_refresh_pending: false,
            awaiting_since: None,
            export_status: None,
            export_cancelled: None,
            export_processes: None,
            output_device: OutputDevice::Default,
            muted: false,
            resume_install: false,
            requested_clock_epoch: 0,
            presented_serial: 0,
            clock_start,
            resume_scrub: false,
            qualifying: false,
            button_bounds: HashMap::new(),
            player_width: f32::INFINITY,
            overflow_open: false,
            overflow_menu: MenuState::new(gpui::Point::default(), &[]),
            overflow_anchor: Default::default(),
            overflow_focus: cx.focus_handle(),
            painted_frame: None,
        };
        shell.dispatch(Command::Initialize, cx);
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let Ok(keep_running) = this.update(cx, |shell, cx| {
                    if shell.closed.load(Ordering::Acquire) {
                        return false;
                    }
                    shell.tick_playback(cx);
                    shell.request_thumbnails(cx);
                    shell.forward_playhead();
                    shell.rebuild_stranded_acceptance(cx);
                    true
                }) else {
                    break;
                };
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
        shell
    }
}

impl StudioShell {
    /// An editor button wired to a shell action, with the qualification bounds probe.
    /// `primary` paints it as the view's main action.
    #[allow(clippy::too_many_arguments)]
    fn action_button(
        &self,
        id: &'static str,
        label: &'static str,
        enabled: bool,
        primary: bool,
        glyph: Option<Icon>,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> EditorButton {
        let theme = self.theme;
        let entity = cx.entity();
        let probe = self.bounds_probe(&gpui::ElementId::Name(id.into()), cx);
        let mut button = EditorButton::plain(&theme, id)
            .label(label)
            .disabled(!enabled)
            .on_click(move |_, _, cx| {
                entity.update(cx, |shell, cx| action(shell, cx));
            });
        button = if primary {
            button.accent(&theme)
        } else {
            button.filled(&theme)
        };
        if let Some(glyph) = glyph {
            button = button.left_icon(glyph.path());
        }
        if let Some(probe) = probe {
            button = button.on_bounds(probe.into_bounds_handler());
        }
        button
    }

    /// An icon button wired to a shell action, with the qualification bounds probe.
    /// The label is the accessible name and the tooltip.
    #[allow(clippy::too_many_arguments)]
    fn action_icon_button(
        &self,
        id: &'static str,
        label: &'static str,
        glyph: Icon,
        size: IconButtonSize,
        enabled: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> IconButton {
        let theme = self.theme;
        let entity = cx.entity();
        let probe = self.bounds_probe(&gpui::ElementId::Name(id.into()), cx);
        let button = IconButton::editor(&theme, id, glyph.path())
            .editor_size(size)
            .label(label)
            .tooltip(&theme, label)
            .disabled(!enabled)
            .on_click(move |_, _, cx| {
                entity.update(cx, |shell, cx| action(shell, cx));
            });
        match probe {
            Some(probe) => button.on_bounds(probe.into_bounds_handler()),
            None => button,
        }
    }
}
