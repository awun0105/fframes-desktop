use crate::conversation_panel::{Attachment, ConversationPanel, PanelEvent};
use crate::{
    actions,
    canvas_view::{CanvasViewState, StageLayout},
    frame_image::{ImagePresentationManager, create_render_image},
    preset_panel::{PresetEvent, PresetPanel, PresetPick},
    timeline_view::{TimelineEvent, TimelineView},
    ui::BoundsProbe,
};
use gpui::{
    AppContext, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, canvas, div, px,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use studio_engine::TaskScope;
use studio_engine::agent::{AgentWorkflow, BuildSettings};
use studio_engine::containment::OwnershipPolicy;
use studio_engine::engine::backend::StudioEnvironment;
use studio_engine::engine::backend::{AgentSdk, Command, Presentation};
use studio_engine::engine::fence::{PresentationVerdict, presentation_verdict};
use studio_engine::engine::handle::{
    CommandFinished, EngineEvent, EngineHandle, PreviewGated, PreviewNews, WorkflowOpen,
};
use studio_engine::presets::{PresetCommand, PresetReport};
use studio_engine::providers::host::HostInbox;
use studio_engine::services::audio_service::{
    AudioEvent, AudioService, OutputDevice, OutputHandle,
};
use studio_engine::services::build_service::BuildService;
use studio_engine::services::export_service::{ExportControl, ExportProgress};
use studio_engine::services::preview_coordinator::{PreviewCoordinator, SeekIntent};
use studio_engine::services::thumbnail_cache::ThumbnailKey;
use studio_project::ProjectId;
use studio_proto::CommandId;
use studio_theme::{METRICS, Shadow, TextStyle, Theme, shadow_for, text_style};
use studio_ui::app_state::{AppEvent, AppState};
use studio_ui::kit::{
    Badge, Card, CollapsibleSection, DISABLED_OPACITY, EditorButton, Handler, Icon, IconButton,
    IconButtonSize, Menu, MenuItem, MenuState, Notice, StatusDot, TabRail, TabRailItem, Tone,
    divider, focus_ring, focusable, heading, icon, muted, on_activate, section_title, with_tooltip,
};

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

/// The export's state and its detail (a failure's reason, the published file's name).
fn export_telemetry(status: Option<&ExportProgress>) -> serde_json::Value {
    let (state, detail) = match status {
        None => ("idle", None),
        Some(ExportProgress::Started { .. }) => ("started", None),
        Some(ExportProgress::Rendering { done, total }) => {
            ("rendering", Some(format!("{done}/{total}")))
        }
        Some(ExportProgress::Audio) => ("audio", None),
        Some(ExportProgress::Warning(warning)) => ("warning", Some(warning.clone())),
        Some(ExportProgress::Verifying) => ("verifying", None),
        Some(ExportProgress::Complete { destination, .. }) => (
            "complete",
            destination
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
        ),
        Some(ExportProgress::Failed(reason)) => ("failed", Some(reason.clone())),
        Some(ExportProgress::Cancelled) => ("cancelled", None),
    };
    serde_json::json!({ "state": state, "detail": detail })
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

fn agent_build(sdk: &AgentSdk, service: &BuildService) -> BuildSettings {
    BuildSettings {
        service: service.clone(),
        sdk: sdk.dir.clone(),
        compatibility: sdk.compatibility.clone(),
    }
}

/// Closes a workflow away from the UI thread (cancels its task, reaps its scopes).
fn close_workflow_detached(workflow: Arc<AgentWorkflow>) {
    std::thread::spawn(move || workflow.close());
}

impl StudioShell {
    /// The text shown while the accepted source has no matching preview yet.
    pub fn awaiting_preview(&self) -> Option<&str> {
        self.awaiting_preview.as_deref()
    }

    /// Guarded handoff of a just-published task revision to playback.
    ///
    /// `staged` is the preview Stage 2's validation prepared for the candidate; it is
    /// adopted under the engine's fresh authorization and re-primed at the latest
    /// playhead before the usual matching video/audio commit (with a new audio epoch)
    /// installs it. If there is no staged preview or authorization, or adoption fails,
    /// the old preview keeps playing, the accepted source is labelled as awaiting its
    /// preview, and an ordinary build of the accepted source is queued. Committed
    /// source is never undone implicitly.
    ///
    /// Returns `Ok(())` when the coordinator adopted the staged preview, otherwise the
    /// reason the accepted revision is left awaiting an ordinary preview build.
    pub fn adopt_promotion(
        &mut self,
        staged: Option<studio_engine::services::preview_coordinator::StagedPreview>,
        promotion: &studio_engine::Promotion,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let published = promotion.record.published.as_str();
        self.awaiting_preview = Some(format!(
            "Accepted source {} is awaiting its preview; the preview still shows the previous revision.",
            &published[..12]
        ));
        let handoff = match (
            staged,
            promotion.authorization.as_ref(),
            self.preview.as_ref(),
        ) {
            (Some(staged), Some(authorization), Some(preview)) => {
                let latest = SeekIntent {
                    identity: self
                        .preview_state
                        .displayed()
                        .cloned()
                        .unwrap_or_else(|| staged.ready().identity().clone()),
                    serial: self.preview_state.serial(),
                    position: self.preview_state.position(),
                    scale: self.preview_state.scale(),
                };
                preview
                    .adopt(staged, authorization, latest)
                    .map(|()| authorization.clone())
                    .map_err(|e| e.to_string())
            }
            (_, None, _) => {
                Err("the source changed again before the preview could be handed off".into())
            }
            _ => Err("no staged preview is available".into()),
        };
        // The published bytes changed the source: the presentation catches up once idle.
        self.refresh_presentation();
        let outcome = match handoff {
            Ok(authorization) => {
                // Preview-state bookkeeping follows the adoption so a rejected adoption
                // leaves nothing half-begun.
                self.preview_state.begin_promotion(&authorization);
                Ok(())
            }
            Err(reason) => {
                self.error = Some(format!(
                    "Preview handoff failed ({reason}); rebuilding the accepted source."
                ));
                self.dispatch(Command::Build, cx);
                Err(reason)
            }
        };
        cx.notify();
        outcome
    }

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
    /// Observe the production shell while the external harness sends native input.
    /// File serialization/writes stay off the UI thread and are awaited (no queue).
    pub fn start_qualification(
        &mut self,
        project: PathBuf,
        output: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.qualifying = true;
        cx.spawn(async move |this, cx| {
            let mut stage = 0;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
                let Ok(value) = this.update(cx, |shell, cx| {
                    if !shell.busy && stage == 0 {
                        shell.dispatch(Command::Open(project.clone()), cx);
                        stage = 1;
                    } else if !shell.busy && stage == 1 && shell.presentation.project.is_some() {
                        shell.dispatch(Command::Build, cx);
                        stage = 2;
                    }
                    shell.qualification_snapshot(cx)
                }) else {
                    break;
                };
                let path = output.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let temporary = path.with_extension("tmp");
                        std::fs::write(
                            &temporary,
                            serde_json::to_vec(&value).map_err(std::io::Error::other)?,
                        )?;
                        std::fs::rename(temporary, path)
                    })
                    .await;
                if let Err(error) = result {
                    let _ = this.update(cx, |shell, cx| {
                        shell.error = Some(format!("Qualification telemetry: {error}"));
                        cx.notify();
                    });
                    break;
                }
            }
        })
        .detach();
    }
    /// M3 observation entry: opens `project` and writes the agent panel's redacted state
    /// (phase, counts, bounds; never prompts or message text) to `output` every 250 ms.
    /// Nothing is launched and no command is sent; an operator drives the real panel.
    ///
    /// `test_ownership` is the explicit test-only injection for fixtures: with it the
    /// adapter's writers count as that ownership WITHOUT any qualification evidence (the
    /// panel says so); without it only validated evidence can qualify an adapter, exactly
    /// as in the product.
    pub fn start_agent_telemetry(
        &mut self,
        project: PathBuf,
        output: PathBuf,
        test_ownership: Option<studio_bootstrap::WriterOwnership>,
        cx: &mut Context<Self>,
    ) {
        if let Some(ownership) = test_ownership {
            self.agent.ownership = OwnershipPolicy::TestInjected(ownership);
        }
        // Observation only: measured bounds of the named transport buttons go to the
        // telemetry so a driver clicks what the shell actually laid out.
        self.qualifying = true;
        cx.spawn(async move |this, cx| {
            let mut opened = false;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                let Ok(value) = this.update(cx, |shell, cx| {
                    if !opened && !shell.busy {
                        shell.dispatch(Command::Open(project.clone()), cx);
                        opened = true;
                    }
                    serde_json::json!({
                        "project_open": shell.presentation.project.is_some(),
                        "sdk_ready_for_agent": shell.presentation.agent_sdk.is_some(),
                        "agent_workflow_open": shell.agent.workflow.is_some(),
                        "awaiting_preview": shell.awaiting_preview.is_some(),
                        "displayed_revision": shell
                            .displayed
                            .as_ref()
                            .map(|d| d.identity().source_revision.chars().take(12).collect::<String>()),
                        "displayed_identity": shell.displayed.as_ref().map(|d| serde_json::json!({
                            "open_session_len": d.identity().open_session.len(),
                            "worker_generation": d.identity().worker_generation,
                            "timeline_open_session_len": d.timeline.envelope.identity.open_session.len(),
                            "timeline_worker_generation": d.timeline.envelope.identity.worker_generation,
                        })),
                        "panel": shell.panel.read(cx).telemetry(),
                        "preview": shell.preview_telemetry(),
                        "canvas": shell.canvas_telemetry(),
                        "buttons": serde_json::to_value(&shell.button_bounds)
                            .unwrap_or(serde_json::Value::Null),
                        "ruler_bounds": shell.timeline.read(cx).qualification_metrics()["ruler_bounds"].clone(),
                        "timeline_selection": shell.timeline.read(cx).qualification_metrics()["selection"].clone(),
                    })
                }) else {
                    break;
                };
                let path = output.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let temporary = path.with_extension("tmp");
                        std::fs::write(
                            &temporary,
                            serde_json::to_vec(&value).map_err(std::io::Error::other)?,
                        )?;
                        std::fs::rename(temporary, path)
                    })
                    .await;
                if let Err(error) = result {
                    let _ = this.update(cx, |shell, cx| {
                        shell.error = Some(format!("Agent telemetry: {error}"));
                        cx.notify();
                    });
                    break;
                }
            }
        })
        .detach();
    }
    /// Redacted playback identity for the M3 observation: revisions are 12-hex prefixes,
    /// the audio digest is the prepared mix's hash prefix; nothing here is project text.
    fn preview_telemetry(&self) -> serde_json::Value {
        let prefix = |text: &str| text.chars().take(12).collect::<String>();
        serde_json::json!({
            "status": match &self.preview_state.status {
                studio_engine::PreviewStatus::Absent => "absent",
                studio_engine::PreviewStatus::Building => "building",
                studio_engine::PreviewStatus::Preparing => "preparing",
                studio_engine::PreviewStatus::Ready => "ready",
                studio_engine::PreviewStatus::Displayed => "displayed",
                studio_engine::PreviewStatus::Error(_) => "error",
                studio_engine::PreviewStatus::Closed => "closed",
            },
            "error": match &self.preview_state.status {
                studio_engine::PreviewStatus::Error(e) => Some(format!("{e:?}").chars().take(4000).collect::<String>()),
                _ => None,
            },
            "transport_epoch": self.transport.epoch(),
            "playing": self.transport.playing(),
            "seek_serial": self.preview_state.serial(),
            "position": self.preview_state.position(),
            "audio_output": self.output.is_some(),
            "painted_frame": self.painted_frame,
            "presented_serial": self.presented_serial,
            "displayed": self.displayed.as_ref().map(|d| serde_json::json!({
                "video_revision": prefix(&d.identity().source_revision),
                "audio_revision": prefix(&d.audio.envelope.identity.source_revision),
                "audio_sha256": prefix(&d.audio.sha256),
                "audio_samples": d.audio.sample_count,
                "sample_rate": d.audio.sample_rate,
                "fps": d.timeline.fps,
                "pcm_start_sample": d.pcm_start_sample,
                "pcm_samples": d.pcm.len() / 8,
                "frame_index": d.frame.as_ref().map(|f| f.response.frame_index),
                "frame_serial": d.frame.as_ref().map(|f| f.response.seek_serial),
                "total_frames": d.timeline.total_frames,
                "position": d.position,
                "seek_serial": d.seek_serial,
            })),
        })
    }
    /// Bounded native-selection observations for the opt-in qualification telemetry.
    /// Only semantic keys and geometry are recorded; authored source and image bytes are not.
    fn canvas_telemetry(&self) -> serde_json::Value {
        let metadata = self
            .canvas
            .displayed
            .as_ref()
            .and_then(|displayed| displayed.metadata.as_ref());
        let objects: Vec<_> = metadata
            .into_iter()
            .flat_map(|metadata| metadata.objects.iter())
            .take(64)
            .map(|object| {
                serde_json::json!({
                    "identity": object.identity,
                    "bounds": object.bounds,
                    "paint_order": object.paint_order,
                    "support": format!("{:?}", object.support),
                })
            })
            .collect();
        let selection = self.canvas.selection.as_ref().map(|selection| {
            serde_json::json!({
                "identity": selection.identity,
                "bounds": selection.bounds,
                "support": format!("{:?}", selection.support),
            })
        });
        let viewport = self.canvas.viewport.as_ref();
        let image_bounds = viewport.map(|viewport| {
            let bounds = viewport.image_bounds();
            serde_json::json!({
                "x": bounds.x,
                "y": bounds.y,
                "width": bounds.width,
                "height": bounds.height,
            })
        });
        let rectangle = self.canvas.rectangle_scope.map(|rectangle| {
            serde_json::json!({
                "x": rectangle.x,
                "y": rectangle.y,
                "width": rectangle.width,
                "height": rectangle.height,
            })
        });
        let rendered_image = self.images.current_image();
        let rendered_image_size = rendered_image.as_ref().map(|image| image.size(0));
        let rendered_bright_pixels = rendered_image
            .as_ref()
            .and_then(|image| image.as_bytes(0))
            .map(|pixels| {
                pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|pixel| pixel[..3].iter().all(|channel| *channel > 180) && pixel[3] > 0)
                    .count()
            });
        serde_json::json!({
            "frame_index": self.canvas.displayed.as_ref().map(|frame| frame.frame_index),
            "seek_serial": self.canvas.displayed.as_ref().map(|frame| frame.seek_serial),
            "metadata_status": metadata.map(|metadata| format!("{:?}", metadata.status)),
            "object_count": metadata.map_or(0, |metadata| metadata.objects.len()),
            "objects_truncated": metadata.is_some_and(|metadata| metadata.objects.len() > 64),
            "objects": objects,
            "selection": selection,
            "rectangle": rectangle,
            "drawing_rectangle": self.canvas.is_drawing_rectangle(),
            "video_dimensions": viewport.map(|viewport| viewport.video_dimensions()),
            "image_bounds": image_bounds,
            "canvas_origin": self.canvas_origin,
            "canvas_extent": self.canvas_extent,
            "rendered_image_size": rendered_image_size.map(|size| (size.width.0, size.height.0)),
            "rendered_bright_pixels": rendered_bright_pixels,
            "message": self.canvas.message,
        })
    }
    /// The shell's redacted state for hosts that observe the app (native qualification,
    /// harnesses): the qualification snapshot plus the preview, canvas, timeline, agent
    /// panel and job state. Revisions are prefixes; no project text or image bytes.
    pub fn telemetry(&self, cx: &gpui::App) -> serde_json::Value {
        let mut value = self.qualification_snapshot(cx);
        value["project_open"] = self.presentation.project.is_some().into();
        if let Some(project) = &self.presentation.project {
            value["accepted_revision"] = project.accepted.as_str().into();
            value["checkpoint_path"] = project.checkpoint.to_string_lossy().into_owned().into();
            value["job"] = project.job.clone().into();
        }
        value["app_version"] = self.app_version.into();
        value["sdk"] = self.presentation.sdk.clone().into();
        value["sdk_ready_for_agent"] = self.presentation.agent_sdk.is_some().into();
        value["awaiting_preview"] = self.awaiting_preview.is_some().into();
        value["preview"] = self.preview_telemetry();
        value["canvas"] = self.canvas_telemetry();
        value["timeline"] = self.timeline.read(cx).qualification_metrics();
        value["panel"] = self.panel.read(cx).telemetry();
        value["export"] = export_telemetry(self.export_status.as_ref());
        value["presets"] = self.preset_panel.read(cx).telemetry();
        value["output_devices"] = self.audio.devices().into();
        value
    }
    fn qualification_snapshot(&self, cx: &gpui::App) -> serde_json::Value {
        let thumbnails = self.timeline.read(cx).qualification_metrics();
        let mut buttons = serde_json::to_value(&self.button_bounds).expect("finite geometry");
        buttons["timeline-fit"] = thumbnails["fit_bounds"].clone();
        serde_json::json!({
            "pid": std::process::id(), "busy": self.busy,
            "project_path": self.presentation.project.as_ref().map(|p| &p.root), "error": self.error,
            "current_source_revision": self.presentation.project.as_ref().map(|p| p.source.as_str()),
            "preview_status": format!("{:?}", self.preview_state.status),
            "displayed_generation": self.displayed.as_ref().map(|d| d.identity().worker_generation),
            "displayed_revision": self.displayed.as_ref().map(|d| &d.identity().source_revision),
            "position": self.transport.position(), "playing": self.transport.playing() || self.resume_install,
            "total_frames": self.displayed.as_ref().map(|d| d.timeline.total_frames),
            "fps": self.displayed.as_ref().map(|d| d.timeline.fps),
            "clock_epoch": self.transport.epoch(), "seek_serial": self.preview_state.serial(),
            "paint_serial": self.presented_serial, "paint_frame": self.painted_frame,
            "preview_extent": self.preview_extent, "preview_scale": self.preview_state.scale(),
            "audio_ready": self.output.is_some(),
            "audio_identity_matches": self.output.as_ref().is_none_or(|o| Some(&o.identity) == self.preview_state.displayed() && o.epoch == self.transport.epoch()),
            "audio_note": self.audio_note, "output_device": format!("{:?}", self.output_device), "muted": self.muted,
            "audio_metrics": self.output.as_ref().map(OutputHandle::metrics),
            "thumbnail_metrics": thumbnails,
            "image_metrics": { "resident": self.images.resident_count(), "queued": self.images.queue_depth(), "dropped": self.images.dropped_count(), "release_failures": self.images.release_failures() },
            "pump_metrics": self.preview.as_ref().map(|p| p.metrics()), "buttons": buttons,
            "ruler_bounds": thumbnails["ruler_bounds"],
        })
    }
    fn discard_install(&mut self, cx: &mut Context<Self>) {
        if self.pending_ready.take().is_some() {
            self.audio.stop(self.transport.epoch());
            self.output = None;
            self.audio_ready_epoch = None;
        }
        if self.resume_install {
            self.resume_install = false;
            self.toggle_playback(cx);
        }
    }
    fn dispatch(&mut self, command: Command, cx: &mut Context<Self>) {
        if self.busy || self.closed.load(Ordering::Acquire) {
            return;
        }
        if matches!(
            &command,
            Command::Close
                | Command::Create(_)
                | Command::Import(_)
                | Command::Open(_)
                | Command::Locate(_, _)
        ) {
            self.cancel_export(cx);
        }
        if let Command::Open(path) | Command::Locate(path, _) = &command {
            self.failed_open = Some(path.clone());
        }
        if matches!(command, Command::CancelBuild | Command::Build) {
            if let Some(preview) = &self.preview {
                preview.cancel_build();
            }
            self.preview_state.cancel_build();
            self.discard_install(cx);
        }
        if matches!(command, Command::Close) {
            self.detach_preview(cx);
        }
        // A command that replaces the open project closes the project's agent workflow
        // first (cancelling its task and reaping its scopes), on the background thread and
        // before the controller it shares is closed.
        let closing = self.take_workflow_for(&command, cx);
        let leaves_session = closing.is_some() || self.leaves_session(&command);
        let refresh = matches!(command, Command::Refresh);
        self.busy = true;
        let id = self.engine.send(command, closing);
        self.in_flight = Some((id, refresh));
        self.app.update(cx, |app, _| app.sent(id, leaves_session));
        cx.notify();
    }

    /// Whether `command` closes or replaces the open project.
    fn leaves_session(&self, command: &Command) -> bool {
        let current = self.presentation.project.as_ref().map(|p| &p.root);
        match command {
            Command::Close | Command::Import(_) | Command::Create(_) => true,
            Command::Open(path) | Command::Locate(path, _) | Command::Independent(path) => {
                current != Some(path)
            }
            _ => false,
        }
    }

    fn engine_event(&mut self, event: EngineEvent, cx: &mut Context<Self>) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        match event {
            EngineEvent::CommandFinished(finished) => {
                self.command_finished(*finished, cx);
                self.refresh_if_wanted();
                // Preview news held back while the command ran.
                self.drain_preview(cx);
            }
            EngineEvent::PreviewGated(gated) => {
                self.preview_gated(*gated, cx);
                self.drain_preview(cx);
            }
            EngineEvent::Refreshed { presentation, .. } => {
                self.presentation_refresh_pending = false;
                if let Some(presentation) = presentation {
                    self.presentation = *presentation;
                    self.push_preset_view(cx);
                }
                self.refresh_if_wanted();
                cx.notify();
            }
            EngineEvent::SourceChanged { .. } => {
                // A native picker or a command defers it; the engine says it again.
                if !self.busy {
                    self.dispatch(Command::Refresh, cx);
                }
            }
            EngineEvent::CleanupProblem(report) => {
                // Background teardown only ever reports problems; show the newest one.
                self.error = Some(format!(
                    "Background cleanup ({}): {}",
                    report.label, report.problem
                ));
                cx.notify();
            }
        }
    }

    fn command_finished(&mut self, finished: CommandFinished, cx: &mut Context<Self>) {
        let CommandFinished {
            id,
            result,
            presentation,
            build: spec,
            preset: report,
            ..
        } = finished;
        let Some((_, refresh)) = self.in_flight.take_if(|(sent, _)| *sent == id) else {
            return;
        };
        // Jobs run in order: a refresh or preview news sent before this command has been
        // answered, or dropped with the session this command left.
        self.presentation_refresh_pending = false;
        self.preview_gate_pending = false;
        self.busy = false;
        if let Some(presentation) = presentation {
            let changed_session =
                self.preview_session.as_ref() != presentation.state.as_ref().map(|s| s.session());
            if changed_session {
                self.detach_preview(cx);
                if let (Some(state), Some(owner)) = (&presentation.state, &presentation.owner) {
                    let preview = PreviewCoordinator::new(owner.sub_manager());
                    let wake = self.preview_wake.clone();
                    preview.set_listener(move || {
                        let _ = wake.try_send(());
                    });
                    self.preview = Some(Arc::new(preview));
                    self.thumbnails_sent = None;
                    self.preview_session = Some(state.session().clone());
                    self.preview_state = studio_engine::PreviewState::default();
                }
            }
            let verdict = if changed_session {
                PresentationVerdict::CancelPreparation
            } else {
                presentation_verdict(
                    &self.preview_state,
                    self.presentation.state.as_ref().map(|s| s.generation()),
                    presentation.state.as_ref(),
                )
            };
            match verdict {
                PresentationVerdict::Older => {
                    // Captured before a publication this shell already knows: it
                    // replaces nothing and cancels nothing; a fresh one is taken.
                    self.refresh_presentation();
                }
                verdict => {
                    if verdict == PresentationVerdict::CancelPreparation {
                        if let Some(preview) = &self.preview {
                            preview.cancel_build();
                        }
                        self.preview_state.cancel_build();
                        self.discard_install(cx);
                    }
                    self.presentation = presentation;
                    self.sync_agent(cx);
                    self.push_preset_view(cx);
                }
            }
        }
        if let Some(spec) = spec {
            self.preview_state.begin(spec.tag.clone());
            if let Some(preview) = &self.preview {
                preview.build(spec);
            }
        }
        match result {
            Ok(()) => {
                if !refresh {
                    self.error = None;
                    self.failed_open = None;
                }
            }
            Err(error) => self.error = Some(error),
        }
        if let Some(report) = report {
            self.adopt_preset_report(report, cx);
        }
        cx.notify();
    }

    /// Shows a preset command's outcome. Only a committed revision asks for a matching
    /// preview: the previous playable preview stays displayed under the awaiting label
    /// until it is ready (or for good if its build fails), and the source is never
    /// rolled back implicitly.
    fn adopt_preset_report(&mut self, report: PresetReport, cx: &mut Context<Self>) {
        let request = report.preview_request();
        let catalog_changed = report.catalog_changed;
        self.preset_panel
            .update(cx, |panel, cx| panel.report(report, cx));
        if catalog_changed {
            self.refresh_presentation();
        }
        if let Some(request) = request {
            self.awaiting_preview = Some(request.label);
            self.refresh_presentation();
            self.dispatch(Command::Build, cx);
        }
    }

    fn push_preset_view(&mut self, cx: &mut Context<Self>) {
        let view = self.presentation.preset.clone();
        self.preset_panel
            .update(cx, |panel, cx| panel.set_view(view, cx));
    }

    fn run_preset(&mut self, command: PresetCommand, cx: &mut Context<Self>) {
        // The fence is the revision this window last showed.
        let expected = self
            .presentation
            .state
            .as_ref()
            .map(|state| state.source().clone());
        self.dispatch(Command::Preset(command, expected), cx);
    }

    fn preset_event(&mut self, event: PresetEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            PresetEvent::Leave => window.focus(&self.focus, cx),
            PresetEvent::Run(command) => self.run_preset(command, cx),
            PresetEvent::Pick(PresetPick::ImportFolder) => self.pick(Picker::PresetImport, cx),
            PresetEvent::Pick(PresetPick::ImportCss) => self.pick(Picker::PresetCss, cx),
            PresetEvent::Pick(PresetPick::Export { key }) => {
                self.pick(Picker::PresetExport(key), cx)
            }
        }
    }
    /// The workflow a command must close before it may run: every command that closes or
    /// replaces the open project. Re-selecting the same folder keeps the workflow.
    fn take_workflow_for(
        &mut self,
        command: &Command,
        cx: &mut Context<Self>,
    ) -> Option<Arc<AgentWorkflow>> {
        if !self.leaves_session(command) {
            return None;
        }
        self.agent.opening = None;
        self.agent.session = None;
        self.agent.attached = None;
        self.agent.applied_sdk = None;
        self.panel.update(cx, |panel, cx| panel.detach(cx));
        self.agent.workflow.take()
    }

    /// Keeps the agent workflow in step with the open project: one workflow per open
    /// session, opened on its own thread (it replays the conversation log), never
    /// launching an agent. Also forwards SDK readiness, which stays separate from the
    /// agent's.
    fn sync_agent(&mut self, cx: &mut Context<Self>) {
        let session = self
            .presentation
            .state
            .as_ref()
            .map(|state| state.session().clone());
        let sdk_ready = self.presentation.agent_sdk.is_some();
        self.panel
            .update(cx, |panel, cx| panel.set_sdk_ready(sdk_ready, cx));
        let Some(session) = session else {
            if let Some(workflow) = self.agent.workflow.take() {
                close_workflow_detached(workflow);
            }
            self.agent.session = None;
            self.agent.opening = None;
            self.agent.attached = None;
            self.panel.update(cx, |panel, cx| panel.detach(cx));
            return;
        };
        if self.agent.session.as_ref() != Some(&session) {
            // A different open session (the project was replaced some other way).
            if let Some(workflow) = self.agent.workflow.take() {
                close_workflow_detached(workflow);
            }
            self.agent.attached = None;
            self.agent.opening = None;
            self.panel.update(cx, |panel, cx| panel.detach(cx));
            self.agent.session = Some(session);
        }
        if self.agent.workflow.is_none() && self.agent.opening.is_none() {
            self.start_agent_open();
        }
        self.sync_agent_build();
    }

    fn start_agent_open(&mut self) {
        let Some(session) = self.presentation.session else {
            return;
        };
        self.agent.serial += 1;
        let serial = self.agent.serial;
        self.agent.opening = Some(serial);
        let service = self.builds.clone();
        let build = self
            .presentation
            .agent_sdk
            .as_ref()
            .map(|sdk| agent_build(sdk, &service));
        self.agent.applied_sdk = self.presentation.agent_sdk.as_ref().map(|s| s.dir.clone());
        self.engine.open_workflow(WorkflowOpen {
            session,
            serial,
            build,
            inbox: self.agent.inbox.clone(),
            policy: self.agent.ownership.clone(),
        });
    }

    /// Hands a changed SDK selection to the workflow (it refuses while a task runs; the
    /// task ending wakes the agent and it is tried again).
    fn sync_agent_build(&mut self) {
        let Some(workflow) = &self.agent.workflow else {
            return;
        };
        let desired = self.presentation.agent_sdk.as_ref().map(|s| s.dir.clone());
        if desired == self.agent.applied_sdk {
            return;
        }
        let service = self.builds.clone();
        if workflow
            .set_build(
                self.presentation
                    .agent_sdk
                    .as_ref()
                    .map(|sdk| agent_build(sdk, &service)),
            )
            .is_ok()
        {
            self.agent.applied_sdk = desired;
        }
    }

    /// The agent half of the window, run whenever the workflow, its open or a history page
    /// woke it: adopt a finished open, adopt committed promotions on this thread (the only
    /// place the preview coordinator is touched), retry a refused build selection (a task
    /// ending wakes it) and refresh the panel.
    fn agent_woken(&mut self, cx: &mut Context<Self>) {
        let inbox = self.agent.inbox.clone();
        inbox.take_wake();
        for opened in inbox.take_opened() {
            self.adopt_opened(opened, cx);
        }
        for queued in inbox.take_handoffs() {
            if self.agent.attached != Some(queued.serial) {
                // Not this project's workflow any more: the staged worker is reaped by
                // dropping it; nothing is adopted.
                continue;
            }
            let published = queued
                .handoff
                .promotion
                .record
                .published
                .as_str()
                .to_owned();
            let outcome =
                self.adopt_promotion(queued.handoff.staged, &queued.handoff.promotion, cx);
            if let Some(workflow) = &self.agent.workflow {
                let _ = workflow.report_handoff(&published, outcome);
            }
        }
        self.report_displayed();
        self.sync_agent_build();
        self.panel.update(cx, |panel, cx| {
            panel.poll(cx);
        });
    }

    /// Tells the workflow which revision the preview now displays.
    fn report_displayed(&mut self) {
        if let Some(workflow) = &self.agent.workflow
            && let Some(revision) = self.agent.displayed.take()
        {
            let _ = workflow.preview_displayed(&revision);
        }
    }

    /// Forwards the playhead to the workflow when it moved.
    fn forward_playhead(&mut self) {
        let Some(workflow) = &self.agent.workflow else {
            return;
        };
        let position = self.preview_state.position();
        if position != self.agent.playhead {
            self.agent.playhead = position;
            workflow.set_playhead(position);
        }
    }

    /// Replaces the presentation with the controller's current one. Unlike a command's
    /// completion this never cancels preparation: a promotion advances the generation on
    /// purpose and the staged preview adopted for it must survive.
    fn refresh_presentation(&mut self) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        if self.presentation_refresh_pending {
            self.refresh_wanted = true;
            return;
        }
        self.presentation_refresh_pending = true;
        self.engine.refresh();
    }

    /// Sends the refresh wanted while another was pending.
    fn refresh_if_wanted(&mut self) {
        if std::mem::take(&mut self.refresh_wanted) {
            self.refresh_presentation();
        }
    }

    /// Safety net for an accepted source whose handoff never produced a preview (a
    /// refresh or a failed staging cancelled it): after a grace period with nothing being
    /// prepared, build the accepted source the ordinary way. Never undoes anything.
    fn rebuild_stranded_acceptance(&mut self, cx: &mut Context<Self>) {
        if self.awaiting_preview.is_none() {
            self.awaiting_since = None;
            return;
        }
        let since = *self.awaiting_since.get_or_insert_with(Instant::now);
        let preparing = matches!(
            self.preview_state.status,
            studio_engine::PreviewStatus::Building | studio_engine::PreviewStatus::Preparing
        ) || self.pending_ready.is_some();
        if preparing {
            self.awaiting_since = Some(Instant::now());
        } else if since.elapsed() > Duration::from_secs(8)
            && !self.busy
            && self.preview.is_some()
            && self
                .presentation
                .project
                .as_ref()
                .is_some_and(|p| p.worker_available)
        {
            self.awaiting_since = Some(Instant::now());
            self.dispatch(Command::Build, cx);
        }
    }

    fn adopt_opened(
        &mut self,
        opened: studio_engine::providers::host::Opened,
        cx: &mut Context<Self>,
    ) {
        let wanted = self.agent.opening == Some(opened.serial);
        let workflow = match opened.result {
            Ok(workflow) => workflow,
            Err(error) => {
                if wanted {
                    self.agent.opening = None;
                    self.error = Some(format!("Agent workflow unavailable: {error}"));
                    cx.notify();
                }
                return;
            }
        };
        if !wanted {
            // Opened for a project the shell has left.
            std::thread::spawn(move || workflow.close());
            return;
        }
        self.agent.opening = None;
        let workflow = Arc::new(workflow);
        let preview_identity = self
            .displayed
            .as_ref()
            .map(|preview| preview.identity().clone());
        let _ = workflow.set_displayed_preview_identity(preview_identity);
        self.agent.workflow = Some(workflow.clone());
        self.agent.attached = Some(opened.serial);
        self.agent.playhead = usize::MAX;
        if let Some(paths) = self.presentation.paths.clone() {
            let scope = self.timeline.read(cx).selected_scope();
            self.panel.update(cx, |panel, cx| {
                panel.attach(
                    Attachment {
                        workflow,
                        paths,
                        serial: opened.serial,
                        adapter: opened.adapter,
                        registry: opened.registry,
                        qualification: opened.qualification,
                        settings_error: opened.settings_error,
                        resolution: opened.resolution,
                        policy: opened.policy,
                    },
                    cx,
                );
            });
            self.install_task_scope(scope, cx);
        }
        self.sync_agent_build();
        cx.notify();
    }

    fn detach_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(workflow) = &self.agent.workflow {
            let _ = workflow.set_displayed_preview_identity(None);
        }
        if let Some(preview) = self.preview.take() {
            // Teardown stays off the UI thread; the root token rejects any late spawn.
            std::thread::spawn(move || preview.close());
        }
        self.preview_session = None;
        self.awaiting_preview = None;
        self.preview_state.close();
        self.displayed = None;
        self.pending_ready = None;
        self.pending_frame = None;
        self.canvas.clear();
        self.canvas_pan_position = None;
        self.clear_images = true;
        let _ = self
            .transport
            .reinstall(30, 0, 0, self.clock_start.elapsed().as_secs_f64());
        self.audio.stop(self.transport.epoch());
        self.output = None;
        self.audio_ready_epoch = None;
        self.resume_install = false;
        self.audio_note = "Audio output not prepared".into();
        self.resume_scrub = false;
        self.timeline.update(cx, |v, cx| v.clear(cx));
    }
    /// Takes the preview coordinator's news. Held back while a command runs or earlier
    /// news is with the engine; both drain again when they finish.
    fn drain_preview(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.preview_gate_pending {
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let events = preview.events();
        if let Some((key, frame)) = events.thumbnail {
            self.timeline
                .update(cx, |v, cx| v.thumbnail(key, frame, cx));
            self.thumbnails_sent = None;
        }
        if let Some(frame) = events.frame
            && self.preview_state.accepts_frame(&frame)
            && self.requested_clock_epoch == self.transport.epoch()
        {
            self.pending_frame = Some(frame);
            cx.notify();
        }
        if events.compiled.is_none() && events.ready.is_none() && events.error.is_none() {
            return;
        }
        self.preview_gate_pending = true;
        self.engine.preview_news(PreviewNews {
            compiled: events.compiled,
            error: events.error,
            ready: events.ready,
        });
    }

    /// Asks the coordinator for the timeline's visible thumbnails when they changed;
    /// nothing while playing.
    fn request_thumbnails(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = &self.preview else {
            return;
        };
        let requests = if self.transport.playing() {
            vec![]
        } else {
            self.timeline.read(cx).thumbnail_requests()
        };
        if self.thumbnails_sent.as_ref() != Some(&requests) {
            preview.thumbnails(requests.clone());
            self.thumbnails_sent = Some(requests);
        }
    }

    fn preview_gated(&mut self, gated: PreviewGated, cx: &mut Context<Self>) {
        let PreviewGated {
            ready,
            state,
            error,
            presentation,
            ..
        } = gated;
        self.preview_gate_pending = false;
        if let Some(p) = presentation {
            self.presentation = p;
            self.push_preset_view(cx);
        }
        if let Some((tag, e)) = error {
            self.preview_state.fail(&tag, e.clone());
            self.discard_install(cx);
            self.error = Some(e);
        } else if let (Some(ready), Some(state)) = (ready, state) {
            // A staged preview of just-published bytes carries the promotion
            // tag (base source = the published revision, the task's own
            // generation); it is installable only under the engine's current
            // authorization, never as a build of the task base.
            let promoted = state.promotion().is_some_and(|p| p.tag() == ready.tag());
            if !promoted
                && (ready.tag().generation != state.generation()
                    || ready.tag().base_source != *state.source())
            {
                if let Some(preview) = &self.preview {
                    preview.cancel_build();
                }
                self.preview_state.cancel_build();
            } else if self.transport.playing()
                && (promoted
                    || matches!(state.job(), studio_engine::JobState::Succeeded(tag) if tag == ready.tag()))
            {
                // Re-evaluate the audible position, then re-prime the candidate
                // under the newly frozen seek serial. Old playback served the
                // entire compilation/preparation, not the compiler's cursor.
                self.resume_install = true;
                self.pause_playback(cx);
            } else if self.preview_state.can_install(&ready, &state).is_ok() {
                let mut clock = self.transport.clone();
                if let Err(e) = clock.reinstall(
                    ready.timeline.fps,
                    ready.timeline.total_frames,
                    ready.position,
                    self.clock_start.elapsed().as_secs_f64(),
                ) {
                    self.error = Some(e.to_string());
                } else {
                    // Reserve the staged epoch in the active clock as well.
                    // A seek during preparation must advance past this epoch,
                    // never collide with a callback for another revision.
                    let _ = self.transport.wait_for_output();
                    self.audio.stage(
                        ready.clone(),
                        clock.epoch(),
                        ready.position,
                        self.output_device.clone(),
                    );
                    self.output = None;
                    self.audio_ready_epoch = None;
                    self.pending_ready = Some(PreparedInstall {
                        ready,
                        clock,
                        audio: None,
                        audio_ready: false,
                    });
                    self.audio_note = "Priming matching audio and first frame".into();
                }
            }
        }
        cx.notify();
    }
    fn seek_preview(&mut self, position: usize, cx: &mut Context<Self>) {
        let resume = std::mem::take(&mut self.resume_install);
        self.pending_ready = None;
        if let Err(e) = self
            .transport
            .seek(position, self.clock_start.elapsed().as_secs_f64())
        {
            self.error = Some(e.to_string());
            return;
        }
        self.audio.stop(self.transport.epoch());
        self.output = None;
        self.audio_ready_epoch = None;
        self.request_frame(self.transport.position(), cx);
        if resume && !self.transport.playing() {
            self.toggle_playback(cx);
        }
        if self.transport.playing() {
            self.prime_playback(cx);
        }
    }
    fn request_frame(&mut self, position: usize, cx: &mut Context<Self>) {
        let Some(id) = self.preview_state.displayed().cloned() else {
            return;
        };
        let scale = self.displayed.as_ref().map_or(1., |d| {
            crate::frame_image::preview_scale(
                d.timeline.width,
                d.timeline.height,
                self.preview_extent,
            )
        });
        let Ok(serial) = self.preview_state.seek(position, scale) else {
            return;
        };
        self.requested_clock_epoch = self.transport.epoch();
        if let Some(preview) = &self.preview {
            preview.seek(SeekIntent {
                identity: id,
                serial,
                position: self.preview_state.position(),
                scale: self.preview_state.scale(),
            });
        }
        self.timeline.update(cx, |v, cx| {
            v.set_position(self.preview_state.position(), cx)
        });
        cx.notify();
    }
    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if self.transport.playing() || self.resume_install {
            self.resume_install = false;
            self.pause_playback(cx);
        } else {
            self.pending_ready = None;
            if let Err(e) = self
                .transport
                .toggle(self.clock_start.elapsed().as_secs_f64())
            {
                self.error = Some(e.to_string());
            } else if self.transport.playing() {
                // Every start passes through a matching first-frame/PCM epoch.
                let _ = self.transport.wait_for_output();
                self.request_frame(self.transport.position(), cx);
                self.prime_playback(cx);
            }
        }
        cx.notify();
    }
    fn pause_playback(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self
            .transport
            .pause(self.clock_start.elapsed().as_secs_f64())
        {
            self.error = Some(e.to_string());
        }
        self.pending_ready = None;
        self.audio.stop(self.transport.epoch());
        self.output = None;
        self.audio_ready_epoch = None;
        self.request_frame(self.transport.position(), cx);
    }

    fn pause_at_painted_frame(&mut self, cx: &mut Context<Self>) {
        if !self.transport.playing() {
            return;
        }
        let frame = self
            .painted_frame
            .unwrap_or_else(|| self.transport.position());
        self.pause_playback(cx);
        self.seek_preview(frame, cx);
    }

    fn install_task_scope(
        &mut self,
        scope: Result<Option<TaskScope>, String>,
        cx: &mut Context<Self>,
    ) {
        let scope = scope.and_then(|scope| match scope {
            Some(mut scope) => {
                scope.canvas_selection = self.canvas.task_scope_selection();
                scope.validate().map_err(|error| error.to_string())?;
                Ok(Some(scope))
            }
            None if self.canvas.task_scope_selection().is_some() => {
                Err("canvas selection has no matching compiled timeline".into())
            }
            None => Ok(None),
        });
        self.panel
            .update(cx, |panel, cx| panel.set_task_scope(scope, cx));
    }

    fn canvas_point(&self, position: gpui::Point<gpui::Pixels>) -> (f64, f64) {
        (
            f64::from(position.x) - self.canvas_origin.0,
            f64::from(position.y) - self.canvas_origin.1,
        )
    }

    fn canvas_pointer_down(&mut self, event: &gpui::MouseDownEvent, cx: &mut Context<Self>) {
        let point = self.canvas_point(event.position);
        if event.button != MouseButton::Middle {
            self.canvas_pan_position = None;
        }
        if event.button == MouseButton::Left {
            self.pause_at_painted_frame(cx);
        }
        match event.button {
            MouseButton::Middle => self.canvas_pan_position = Some(point),
            MouseButton::Left if event.modifiers.shift => {
                if !self.canvas.begin_rectangle(point.0, point.1) {
                    self.canvas.message =
                        Some("Start a rectangle drag inside the painted video image.".into());
                }
            }
            MouseButton::Left => {
                return self.select_canvas(point, event.modifiers.alt, cx);
            }
            _ => return,
        }
        self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
        cx.notify();
    }

    /// Selects the object at canvas `point` (`cycle`: the next one under it) and makes the
    /// selection the agent's task scope.
    fn select_canvas(&mut self, point: (f64, f64), cycle: bool, cx: &mut Context<Self>) {
        if let Err(error) = self.canvas.select_at(point.0, point.1, cycle) {
            self.canvas.message = Some(error.to_string());
        }
        self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
        cx.notify();
    }

    /// Where video pixel (`x`, `y`) is painted on the canvas, if the video is laid out.
    fn video_to_canvas(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let viewport = self.canvas.viewport.as_ref()?;
        let image = viewport.image_bounds();
        let (width, height) = viewport.video_dimensions();
        Some((
            image.x + x * image.width / f64::from(width),
            image.y + y * image.height / f64::from(height),
        ))
    }

    fn canvas_pointer_move(&mut self, event: &gpui::MouseMoveEvent, cx: &mut Context<Self>) {
        let point = self.canvas_point(event.position);
        if self.canvas.is_drawing_rectangle() {
            self.canvas.update_rectangle(point.0, point.1);
            cx.notify();
        } else if event.pressed_button == Some(MouseButton::Middle)
            && let Some(previous) = self.canvas_pan_position.replace(point)
            && let Some(viewport) = &mut self.canvas.viewport
        {
            let _ = viewport.pan_by(point.0 - previous.0, point.1 - previous.1);
            cx.notify();
        }
    }

    fn canvas_pointer_up(&mut self, button: MouseButton, cx: &mut Context<Self>) {
        if button == MouseButton::Middle {
            self.canvas_pan_position = None;
        } else if button == MouseButton::Left && self.canvas.is_drawing_rectangle() {
            if let Some(rect) = self.canvas.end_rectangle() {
                self.canvas.message = Some(format!(
                    "Rectangle scope · {:.0}×{:.0} video pixels · source identity is not inferred.",
                    rect.width, rect.height
                ));
            }
            self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
            cx.notify();
        }
    }

    fn canvas_scroll(&mut self, event: &gpui::ScrollWheelEvent, cx: &mut Context<Self>) {
        let point = self.canvas_point(event.position);
        let delta = match event.delta {
            gpui::ScrollDelta::Pixels(pixels) => f64::from(pixels.y),
            gpui::ScrollDelta::Lines(lines) => f64::from(lines.y * 32.0),
        };
        if event.modifiers.control || event.modifiers.platform {
            let factor = if delta > 0.0 {
                1.0 + delta.abs() * 0.01
            } else {
                1.0 / (1.0 + delta.abs() * 0.01)
            };
            if let Some(viewport) = &mut self.canvas.viewport {
                let _ = viewport.zoom_at(point.0, point.1, factor);
            }
        } else if let Some(viewport) = &mut self.canvas.viewport {
            let _ = viewport.pan_by(-delta, 0.0);
        }
        cx.notify();
    }

    fn cycle_canvas_selection(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.canvas.cycle_last_hit() {
            self.canvas.message = Some(error.to_string());
        }
        cx.notify();
    }

    fn reset_canvas_view(&mut self, cx: &mut Context<Self>) {
        if let Some(viewport) = &mut self.canvas.viewport {
            viewport.reset_to_fit();
        }
        cx.notify();
    }

    fn prime_playback(&mut self, cx: &mut Context<Self>) {
        if let Some(ready) = &self.displayed {
            self.output = None;
            self.audio_ready_epoch = None;
            self.audio.stage(
                ready.clone(),
                self.transport.epoch(),
                self.transport.position(),
                self.output_device.clone(),
            );
            self.audio_note = "Buffering · waiting for matching first frame and PCM".into();
            cx.notify();
        }
    }
    fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.muted = !self.muted;
        self.audio.mute(self.muted);
        cx.notify();
    }
    fn change_output(&mut self, next: OutputDevice, cx: &mut Context<Self>) {
        self.output_device = next;
        self.seek_preview(self.transport.position(), cx);
        if !self.transport.playing() {
            self.prime_playback(cx);
        }
    }
    fn cycle_output(&mut self, cx: &mut Context<Self>) {
        let next = self.output_device.next(&self.audio.devices());
        self.change_output(next, cx);
    }
    fn poll_audio(&mut self, cx: &mut Context<Self>) {
        match self.audio.event() {
            Some(AudioEvent::Staged {
                epoch,
                identity,
                output,
                unavailable,
            }) => {
                if let Some(pending) = &mut self.pending_ready
                    && pending.clock.epoch() == epoch
                    && pending.ready.identity() == &identity
                {
                    pending.audio = output;
                    pending.audio_ready = true;
                    self.audio_note =
                        unavailable.unwrap_or_else(|| "Matching audio prepared".into());
                } else if epoch == self.transport.epoch()
                    && self.preview_state.displayed() == Some(&identity)
                {
                    self.output = output;
                    self.audio_ready_epoch = Some(epoch);
                    self.audio_note = unavailable.unwrap_or_else(|| "Audio output prepared".into());
                }
                cx.notify();
            }
            Some(AudioEvent::Failed { epoch, error }) => {
                if self
                    .pending_ready
                    .as_ref()
                    .is_some_and(|p| p.clock.epoch() == epoch)
                {
                    let pending = self.pending_ready.take().unwrap();
                    self.preview_state.fail(pending.ready.tag(), error.clone());
                    if let Some(preview) = &self.preview {
                        preview.cancel_build();
                    }
                    self.error = Some(format!(
                        "Audio preparation failed; prior preview retained: {error}"
                    ));
                    if self.resume_install {
                        self.resume_install = false;
                        self.toggle_playback(cx);
                    }
                } else if epoch == self.transport.epoch() {
                    self.output = None;
                    self.audio_ready_epoch = Some(epoch);
                    self.audio_note = format!("Audio unavailable · {error}");
                }
                cx.notify();
            }
            Some(AudioEvent::Lost { epoch }) if epoch == self.transport.epoch() => {
                self.output = None;
                self.audio_ready_epoch = None;
                let _ = self
                    .transport
                    .fallback(self.clock_start.elapsed().as_secs_f64());
                self.audio.stop(self.transport.epoch());
                self.request_frame(self.transport.position(), cx);
                self.audio_note =
                    "Audio device lost · monotonic fallback · Retry audio to reconnect".into();
            }
            _ => {}
        }
    }
    fn activate_audio(&mut self, cx: &mut Context<Self>) {
        let epoch = self.transport.epoch();
        if self.audio_ready_epoch == Some(epoch)
            && self.presented_serial == self.preview_state.serial()
        {
            self.audio_ready_epoch = None;
            if self.output.is_some() {
                self.audio.commit(epoch, self.transport.playing());
            } else {
                let _ = self
                    .transport
                    .fallback(self.clock_start.elapsed().as_secs_f64());
                self.audio.stop(self.transport.epoch());
                self.requested_clock_epoch = self.transport.epoch();
                self.audio_note = "Audio unavailable · monotonic fallback".into();
            }
            cx.notify();
        }
    }
    fn step_preview(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.resume_install = false;
        self.pause_playback(cx);
        self.seek_preview(self.transport.position().saturating_add_signed(delta), cx);
    }
    fn tick_playback(&mut self, cx: &mut Context<Self>) {
        self.poll_audio(cx);
        if let Some(output) = &self.output
            && let Some(snapshot) = output.snapshot()
        {
            self.transport.accept_output(snapshot);
        }
        let was_playing = self.transport.playing();
        let position = self
            .transport
            .tick(self.clock_start.elapsed().as_secs_f64());
        if position != self.preview_state.position() {
            self.request_frame(position, cx);
        }
        if was_playing != self.transport.playing() {
            if !self.transport.playing() {
                self.audio.stop(self.transport.epoch());
                self.output = None;
                self.audio_ready_epoch = None;
            }
            cx.notify();
        }
    }
    fn pick(&mut self, kind: Picker, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        // A watcher refresh must not discard the result of an active native dialog.
        self.busy = true;
        cx.notify();
        if matches!(
            kind,
            Picker::Create | Picker::Restore | Picker::PresetExport(_) | Picker::Export
        ) {
            let home = std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            let picker = cx.prompt_for_new_path(
                &home,
                Some(match kind {
                    Picker::Create => "New video",
                    Picker::PresetExport(_) => "Exported preset",
                    Picker::Export => "Exported video.mp4",
                    _ => "Recovered video",
                }),
            );
            // Export the checkpoint selected before the picker opened, even if
            // current source is degraded. No source installation occurs here.
            let accepted = self
                .presentation
                .project
                .as_ref()
                .map(|p| p.accepted.clone());
            cx.spawn(async move |this, cx| {
                let result = picker.await;
                let _ = this.update(cx, |shell, cx| {
                    if shell.closed.load(Ordering::Acquire) {
                        return;
                    }
                    shell.busy = false;
                    // Preview news held back while the dialog was open.
                    shell.drain_preview(cx);
                    match result {
                        Ok(Ok(Some(path))) => {
                            if let Picker::PresetExport(key) = kind {
                                shell.run_preset(PresetCommand::Export { key, dest: path }, cx);
                            } else if matches!(kind, Picker::Create) {
                                shell.dispatch(Command::Create(path), cx);
                            } else if matches!(kind, Picker::Export) {
                                shell.start_export(path, cx);
                            } else if let Some(accepted) = accepted {
                                shell.dispatch(Command::Restore(path, accepted), cx);
                            }
                        }
                        Ok(Ok(None)) => (),
                        error => shell.error = Some(format!("Native picker failed: {error:?}; check the desktop portal/file chooser and retry")),
                    }
                    cx.notify();
                });
            }).detach();
        } else {
            let label = match &kind {
                Picker::Asset => "Asset",
                Picker::Import => "Import",
                Picker::Sdk => "SDK",
                Picker::SdkBundle => "Offline SDK bundle folder",
                Picker::Locate(_) => "Locate",
                Picker::PresetImport => "Import preset folder",
                Picker::PresetCss => "CSS report",
                _ => "Open",
            };
            let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
                files: matches!(kind, Picker::Asset | Picker::Import | Picker::PresetCss),
                directories: !matches!(kind, Picker::Asset | Picker::Import | Picker::PresetCss),
                multiple: false,
                prompt: Some(format!("{label}…").into()),
            });
            cx.spawn(async move |this, cx| {
                let result = picker.await;
                let _ = this.update(cx, |shell, cx| {
                    if shell.closed.load(Ordering::Acquire) {
                        return;
                    }
                    shell.busy = false;
                    // Preview news held back while the dialog was open.
                    shell.drain_preview(cx);
                    match result {
                        Ok(Ok(Some(paths))) => if let Some(path) = paths.into_iter().next() {
                            let command = match kind {
                                Picker::Asset => Command::Asset(path),
                                Picker::Import => Command::Import(path),
                                Picker::Sdk => Command::SelectSdk(path),
                                Picker::SdkBundle => Command::ImportSdkBundle(path),
                                Picker::Locate(id) => Command::Locate(path, id),
                                Picker::PresetImport => {
                                    return shell
                                        .run_preset(PresetCommand::ImportFolder(path), cx);
                                }
                                Picker::PresetCss => {
                                    return shell.run_preset(PresetCommand::ImportCss(path), cx);
                                }
                                _ => Command::Open(path),
                            };
                            shell.dispatch(command, cx);
                        },
                        Ok(Ok(None)) => (),
                        error => shell.error = Some(format!("Native picker failed: {error:?}; check the desktop portal/file chooser and retry")),
                    }
                    cx.notify();
                });
            }).detach();
        }
    }

    fn start_export(&mut self, destination: PathBuf, cx: &mut Context<Self>) {
        if self.export_cancelled.is_some() {
            return;
        }
        let (Some(session), Some(sdk)) = (
            self.presentation.session,
            self.presentation
                .agent_sdk
                .as_ref()
                .map(|sdk| sdk.dir.clone()),
        ) else {
            self.export_status = Some(ExportProgress::Failed(
                "Select a compatible installed SDK and open a project before exporting".into(),
            ));
            cx.notify();
            return;
        };
        let job = match self
            .engine
            .start_export(session, destination, sdk, self.builds.clone())
        {
            Ok(job) => job,
            Err(error) => {
                self.export_status = Some(ExportProgress::Failed(error));
                cx.notify();
                return;
            }
        };
        let control = job.control.clone();
        self.export_cancelled = Some(control.clone());
        self.export_processes = Some(job.processes.clone());
        self.export_status = Some(ExportProgress::Started {
            revision: "capturing validated revision".into(),
            label: "Preparing immutable project snapshot".into(),
        });
        cx.spawn(async move |this, cx| {
            while let Ok(event) = job.progress.recv().await {
                let terminal = is_terminal(&event);
                let shown = this.update(cx, |shell, cx| {
                    shell.export_progress(&control, Some(event), cx)
                });
                if terminal || shown.is_err() {
                    return;
                }
            }
            let _ = this.update(cx, |shell, cx| shell.export_progress(&control, None, cx));
        })
        .detach();
        cx.notify();
    }

    /// Shows the progress of the export `control` runs; `None` when its thread stopped
    /// without a terminal event. A replaced export's progress is ignored.
    fn export_progress(
        &mut self,
        control: &Arc<ExportControl>,
        event: Option<ExportProgress>,
        cx: &mut Context<Self>,
    ) {
        if !self
            .export_cancelled
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, control))
        {
            return;
        }
        let event = event.unwrap_or_else(|| {
            if control.is_cancelled() {
                ExportProgress::Cancelled
            } else {
                ExportProgress::Failed("Export worker stopped without a completion result".into())
            }
        });
        if is_terminal(&event) {
            self.export_cancelled = None;
            self.export_processes = None;
        }
        self.export_status = Some(event);
        cx.notify();
    }

    fn cancel_export(&mut self, cx: &mut Context<Self>) {
        let Some(cancelled) = &self.export_cancelled else {
            return;
        };
        if !cancelled.request_cancel() {
            if cancelled.is_publishing() {
                self.export_status = Some(ExportProgress::Verifying);
                cx.notify();
            }
            return;
        }
        if let Some(processes) = self.export_processes.clone() {
            cx.background_executor()
                .spawn(async move {
                    processes.shutdown(Duration::ZERO);
                })
                .detach();
        }
        self.export_status = Some(ExportProgress::Cancelled);
        cx.notify();
    }

    /// Bounds reporter for the buttons native qualification clicks; `None` outside
    /// qualification and for every other button.
    fn bounds_probe(&self, id: &gpui::ElementId, cx: &mut Context<Self>) -> Option<BoundsProbe> {
        let name = match id {
            gpui::ElementId::Name(name)
                if self.qualifying
                    && matches!(
                        name.as_ref(),
                        "build-preview"
                            | "cancel-preview"
                            | "play-pause"
                            | "mute-audio"
                            | "next-audio-output"
                            | "retry-audio-output"
                            | "no-audio-output"
                            | "previous-frame"
                            | "next-frame"
                            | "close"
                    ) =>
            {
                Some(name.to_string())
            }
            _ => None,
        };
        let entity = cx.entity();
        name.map(|name| {
            BoundsProbe::new(move |bounds, cx| {
                entity.update(cx, |shell, _| {
                    shell.button_bounds.insert(name.clone(), bounds);
                });
            })
        })
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

    fn render_header(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let enabled = !self.busy;
        let project = self.presentation.project.clone();
        let mut actions = div().flex().items_center().gap(px(METRICS.space_2));
        for id in header_actions(project.is_some()) {
            let button = match id {
                "create" => self.action_button(
                    "create",
                    "Create…",
                    enabled,
                    false,
                    Some(Icon::Plus),
                    cx,
                    |s, cx| s.pick(Picker::Create, cx),
                ),
                "open" => self.action_button(
                    "open",
                    "Open…",
                    enabled,
                    false,
                    Some(Icon::FolderOpen),
                    cx,
                    |s, cx| s.pick(Picker::Open, cx),
                ),
                "import" => self.action_button(
                    "import",
                    "Import Rust…",
                    enabled,
                    false,
                    Some(Icon::Upload),
                    cx,
                    |s, cx| s.pick(Picker::Import, cx),
                ),
                "export-mp4" => self.action_button(
                    "export-mp4",
                    "Export MP4…",
                    enabled
                        && project.is_some()
                        && self.presentation.agent_sdk.is_some()
                        && self.export_cancelled.is_none(),
                    true,
                    Some(Icon::Download),
                    cx,
                    |s, cx| s.pick(Picker::Export, cx),
                ),
                _ => self.action_button(
                    "close",
                    "Close project",
                    enabled && project.is_some(),
                    false,
                    None,
                    cx,
                    |s, cx| s.dispatch(Command::Close, cx),
                ),
            };
            actions = actions.child(button);
        }
        let title = project
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "fframes Studio".into());
        let mut text = div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(text_style(div().truncate(), TextStyle::Title).child(title));
        if let Some(p) = &project {
            let path = p.root.display().to_string();
            text = text.child(with_tooltip(
                text_style(div().id("project-path").truncate(), TextStyle::Small)
                    .text_color(theme.editor.text_2)
                    .child(path.clone()),
                &theme,
                Some(path.into()),
            ));
        }
        Card::editor(&self.theme)
            .h(px(METRICS.header_h))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(METRICS.space_4))
            .px(px(METRICS.space_4))
            .child(
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap(px(METRICS.space_3))
                    .child(icon(Icon::Layers, 20., theme.editor.accent))
                    .child(text),
            )
            .child(actions)
            .into_any_element()
    }

    /// Warning, error, export and recovery notices; empty when there is nothing to say.
    fn render_notices(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let enabled = !self.busy;
        let mut notices = Vec::new();
        if let Some(label) = &self.awaiting_preview {
            notices.push(
                Notice::new("awaiting-preview", Tone::Warning, label.clone()).into_any_element(),
            );
        }
        if let Some(error) = &self.error {
            let (title, details) = error_title_details(error);
            let mut notice = Notice::new("error", Tone::Danger, title);
            if let Some(details) = details {
                notice = notice.details(details);
            }
            if let Some(path) = self.failed_open.clone().filter(|_| {
                error.contains("Duplicate project ID")
                    || error.contains("Relocated project content differs")
            }) {
                notice = notice.action(self.action_button(
                    "independent",
                    "Use this folder as an independent project",
                    enabled,
                    false,
                    None,
                    cx,
                    move |s, cx| s.dispatch(Command::Independent(path.clone()), cx),
                ));
            }
            notices.push(notice.into_any_element());
        }
        let Some(p) = &self.presentation.project else {
            return notices;
        };
        if let Some(status) = &self.export_status {
            let (tone, message) = match status {
                ExportProgress::Started { revision, label } => (
                    Tone::Info,
                    format!("Export snapshot · {label} · {revision}"),
                ),
                ExportProgress::Rendering { done, total } => {
                    (Tone::Info, format!("Rendering MP4 · {done}/{total} frames"))
                }
                ExportProgress::Audio => (Tone::Info, "Writing and verifying audio".into()),
                ExportProgress::Warning(message) => {
                    (Tone::Warning, format!("Export warning · {message}"))
                }
                ExportProgress::Verifying => {
                    (Tone::Info, "Verifying MP4 before publication".into())
                }
                ExportProgress::Complete {
                    destination,
                    revision,
                } => (
                    Tone::Success,
                    format!("Exported {} · revision {}", destination.display(), revision),
                ),
                ExportProgress::Failed(message) => {
                    (Tone::Danger, format!("Export failed · {message}"))
                }
                ExportProgress::Cancelled => (
                    Tone::Info,
                    "Export cancelled · partial output removed".into(),
                ),
            };
            let mut notice = Notice::new("export-status", tone, message);
            if self.export_cancelled.is_some() {
                notice = notice.action(self.action_button(
                    "cancel-export",
                    "Cancel export",
                    true,
                    false,
                    None,
                    cx,
                    |shell, cx| shell.cancel_export(cx),
                ));
            }
            notices.push(notice.into_any_element());
        }
        if p.interrupted {
            notices.push(
                Notice::new(
                    "interrupted-job",
                    Tone::Warning,
                    if p.draft.is_some() {
                        "Interrupted job · retained draft available"
                    } else {
                        "Interrupted before draft publication · source and saved checkpoint are safe"
                    },
                )
                .into_any_element(),
            );
        }
        if let Some(text) = &p.recovery_notice {
            notices.push(
                Notice::new("recovery-notice", Tone::Warning, text.clone()).into_any_element(),
            );
        }
        notices
    }

    /// Source, checkpoint and job on one line; the other status lines when expanded.
    fn render_status_strip(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let p = self.presentation.project.as_ref()?;
        let theme = self.theme;
        let expanded = status_strip_expanded(&p.job, self.error.is_some(), self.status_open);
        let tone = if p.source == p.accepted {
            Tone::Success
        } else {
            Tone::Warning
        };
        let summary = format!(
            "Source {} · Checkpoint {} · Job {}",
            &p.source.as_str()[..12],
            &p.accepted.as_str()[..12],
            p.job
        );
        let line = |text: &'static str| {
            text_style(div(), TextStyle::Small)
                .text_color(theme.editor.text_2)
                .child(text)
        };
        let lines = div()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_1))
            .pt(px(METRICS.space_1))
            .child(line(if p.source == p.accepted {
                "Current source matches the saved checkpoint"
            } else {
                "Source changed · saved checkpoint remains available"
            }))
            .child(line(if p.history_available {
                "Local history available"
            } else {
                "Previous machine history unavailable · source baseline saved locally"
            }))
            .child(line(if p.worker_available {
                "Worker bridge available · build an immutable preview"
            } else {
                "Worker bridge missing · source remains navigable"
            }));
        let entity = cx.entity();
        Some(
            Card::editor(&self.theme)
                .id("status-strip")
                .flex_none()
                .px(px(METRICS.space_3))
                .py(px(METRICS.space_2))
                .child(
                    CollapsibleSection::new("status-details", expanded)
                        .header(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(METRICS.space_2))
                                .child(StatusDot::new(tone))
                                .child(
                                    text_style(div(), TextStyle::Small)
                                        .text_color(theme.editor.text_2)
                                        .child(summary),
                                ),
                        )
                        .content(lines)
                        .on_toggle(move |open, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.status_open = *open;
                                cx.notify();
                            });
                        }),
                )
                .into_any_element(),
        )
    }

    /// Tab rail and the open tab's content.
    fn render_left_column(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let entity = cx.entity();
        let rail = TabRail::editor(
            &theme,
            "left-tabs",
            theme.editor.card.into(),
            LeftTab::ALL
                .iter()
                .map(|tab| {
                    TabRailItem::new(tab.icon().path(), tab.name(), *tab == self.left_tab, false)
                })
                .collect(),
        )
        .on_select(move |index, _, cx| {
            entity.update(cx, |shell, cx| {
                shell.left_tab = LeftTab::from_index(*index);
                cx.notify();
            });
        });
        let content = match left_tab_content(self.left_tab) {
            LeftContent::Files => self.render_project_tab(cx),
            LeftContent::Assets => self.render_assets_tab(cx),
            LeftContent::Presets => self.render_style_tab(cx),
        };
        Card::editor(&self.theme)
            .w(px(METRICS.left_column_w))
            .flex_none()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_3))
            .p(px(METRICS.space_4))
            .child(rail)
            .child(content)
            .into_any_element()
    }

    /// A 28 px file row: icon, truncated name, hover fill. The full name is the tooltip.
    fn list_row(
        &self,
        id: String,
        label: String,
        glyph: Icon,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let entity = cx.entity();
        let handler: Handler = std::rc::Rc::new(move |_, cx| {
            entity.update(cx, |shell, cx| action(shell, cx));
        });
        let row = div()
            .id(id)
            .role(gpui::Role::Button)
            .aria_label(label.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            .h(px(LIST_ROW_H))
            .px(px(METRICS.space_2))
            .min_w_0()
            .rounded(px(METRICS.radius_item))
            .hover(move |style| style.bg(theme.editor.ctl_hover))
            .child(icon(glyph, METRICS.icon_sm, theme.editor.text_2))
            .child(
                text_style(div().min_w_0().flex_1().truncate(), TextStyle::Body)
                    .text_color(theme.editor.text_1)
                    .child(label.clone()),
            );
        let row = on_activate(
            focusable(row, &theme, false).cursor_pointer(),
            false,
            Some(handler),
        );
        with_tooltip(row, &theme, Some(label.into()))
    }

    /// The file list of a project tab, or the hint shown while no project is open.
    fn file_list(
        &self,
        files: Option<&[String]>,
        root: Option<PathBuf>,
        glyph: Icon,
        empty: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let mut list = div()
            .id("inventory")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll();
        let (Some(files), Some(root)) = (files, root) else {
            return list.child(
                text_style(div(), TextStyle::Body)
                    .text_color(theme.editor.text_2)
                    .child("Create or open a project to browse its files."),
            );
        };
        if files.is_empty() {
            list = list.child(
                text_style(div(), TextStyle::Body)
                    .text_color(theme.editor.text_2)
                    .child(empty),
            );
        }
        for file in files {
            let path = root.join(file);
            list = list.child(self.list_row(
                format!("file-{file}"),
                file.clone(),
                glyph,
                cx,
                move |_, cx| cx.reveal_path(&path),
            ));
        }
        list
    }

    fn render_project_tab(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let enabled = !self.busy;
        let project = self.presentation.project.clone();
        let mut title = section_title(&theme, "Files");
        if let Some(p) = &project {
            title = title.child(Badge::new(p.file_count.to_string()));
        }
        let files = self.file_list(
            project.as_ref().map(|p| p.files.as_slice()),
            project.as_ref().map(|p| p.root.clone()),
            Icon::File,
            "No files to show",
            cx,
        );
        let mut tab = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .child(title)
            .child(files);
        if let Some(p) = &project
            && p.file_count > p.files.len()
        {
            tab = tab.child(
                text_style(div(), TextStyle::Small)
                    .flex_none()
                    .text_color(theme.editor.text_3)
                    .child(format!(
                        "Showing {} of {} files",
                        p.files.len(),
                        p.file_count
                    )),
            );
        }

        let sdk_ready = self.presentation.agent_sdk.is_some();
        let status = if self.busy {
            "Operation in progress…".to_owned()
        } else {
            self.presentation.sdk.clone()
        };
        let entity = cx.entity();
        let sdk = CollapsibleSection::new("sdk-section", self.sdk_open)
            .header(section_title(&self.theme, "Managed SDK"))
            .content(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(METRICS.space_2))
                    .pt(px(METRICS.space_2))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(METRICS.space_2))
                            .child(
                                div()
                                    .pt(px(5.))
                                    .child(StatusDot::new(sdk_status_tone(self.busy, sdk_ready))),
                            )
                            .child(
                                text_style(div().min_w_0().flex_1(), TextStyle::Small)
                                    .text_color(theme.editor.text_2)
                                    .child(status),
                            ),
                    )
                    .child(self.action_button(
                        "install",
                        "Install managed SDK",
                        enabled,
                        false,
                        None,
                        cx,
                        |s, cx| s.dispatch(Command::InstallSdk, cx),
                    ))
                    .child(self.action_button(
                        "import-sdk-bundle",
                        "Import offline SDK bundle…",
                        enabled,
                        false,
                        None,
                        cx,
                        |s, cx| s.pick(Picker::SdkBundle, cx),
                    ))
                    .child(self.action_button(
                        "sdk",
                        "Select installed SDK…",
                        enabled && project.is_some(),
                        false,
                        None,
                        cx,
                        |s, cx| s.pick(Picker::Sdk, cx),
                    )),
            )
            .on_toggle(move |open, _, cx| {
                entity.update(cx, |shell, cx| {
                    shell.sdk_open = *open;
                    cx.notify();
                });
            });
        let mut library = div()
            .id("left-library")
            .flex_none()
            .max_h(px(360.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_3))
            .child(divider(&self.theme))
            .child(sdk);
        if project.is_some() {
            let entity = cx.entity();
            library = library.child(
                CollapsibleSection::new("recents-section", self.recents_open)
                    .header(section_title(&self.theme, "Recent projects"))
                    .content(div().pt(px(METRICS.space_2)).child(self.render_recents(cx)))
                    .on_toggle(move |open, _, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.recents_open = *open;
                            cx.notify();
                        });
                    }),
            );
        }
        tab.child(library).into_any_element()
    }

    fn render_assets_tab(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let project = self.presentation.project.clone();
        let files = self.file_list(
            project.as_ref().map(|p| p.assets.as_slice()),
            project.as_ref().map(|p| p.root.clone()),
            Icon::Image,
            "No assets imported",
            cx,
        );
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .child(section_title(&self.theme, "Assets"))
            .child(files)
            .child(self.action_button(
                "asset",
                "Copy asset…",
                !self.busy && project.is_some(),
                false,
                Some(Icon::Plus),
                cx,
                |s, cx| s.pick(Picker::Asset, cx),
            ))
            .into_any_element()
    }

    fn render_style_tab(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let mut tab = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2));
        if let Some(p) = &self.presentation.project {
            if p.styles.is_empty() {
                tab = tab.child(
                    text_style(div(), TextStyle::Small)
                        .text_color(theme.editor.text_3)
                        .child("No style preset configured"),
                );
            }
            for file in &p.styles {
                let path = p.root.join(file);
                tab = tab.child(self.list_row(
                    format!("file-{file}"),
                    file.clone(),
                    Icon::File,
                    cx,
                    move |_, cx| cx.reveal_path(&path),
                ));
            }
        }
        tab.child(
            div()
                .id("preset-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(self.preset_panel.clone()),
        )
        .into_any_element()
    }

    /// The recent-projects list. Rendered once: in the left column while a project is
    /// open, in the empty state otherwise, so its element ids stay unique.
    fn render_recents(&self, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let enabled = !self.busy;
        let mut recents = div()
            .id("recents")
            .flex()
            .flex_col()
            .gap(px(METRICS.space_1));
        if self.presentation.recents.is_empty() {
            recents = recents.child(
                text_style(div(), TextStyle::Body)
                    .text_color(theme.editor.text_2)
                    .child("No recent projects"),
            );
        }
        for recent in self.presentation.recents.clone() {
            let id = String::from(recent.id.clone());
            let path = recent.path.clone();
            let project_id = recent.id.clone();
            let group = format!("recent-group-{id}");
            let label = format!(
                "{}{}",
                recent.name,
                if recent.missing { " · Missing" } else { "" }
            );
            let open_enabled = enabled && !recent.missing;
            let entity = cx.entity();
            let open: Handler = std::rc::Rc::new(move |_, cx| {
                entity.update(cx, |shell, cx| {
                    shell.dispatch(Command::Open(path.clone()), cx)
                });
            });
            let mut name_line = div().flex().items_center().gap(px(METRICS.space_2)).child(
                text_style(div().min_w_0().flex_1().truncate(), TextStyle::Body)
                    .text_color(theme.editor.text_1)
                    .child(recent.name.clone()),
            );
            if recent.missing {
                name_line = name_line.child(Badge::new("Missing").tone(Tone::Warning));
            }
            let main = div()
                .id(format!("recent-{id}"))
                .role(gpui::Role::Button)
                .aria_label(label)
                .flex()
                .flex_col()
                .w_full()
                .min_w_0()
                .px(px(METRICS.space_2))
                .py(px(METRICS.space_1))
                .rounded(px(METRICS.radius_item))
                .child(name_line)
                .child(
                    text_style(div().truncate(), TextStyle::Small)
                        .text_color(theme.editor.text_3)
                        .child(recent.path.display().to_string()),
                );
            let main = if open_enabled {
                main.hover(move |style| style.bg(theme.editor.ctl_hover))
                    .cursor_pointer()
            } else {
                main.opacity(DISABLED_OPACITY)
            };
            let main = on_activate(
                focusable(main, &theme, !open_enabled),
                !open_enabled,
                Some(open),
            );
            let main = with_tooltip(main, &theme, Some(recent.path.display().to_string().into()));

            // Row actions appear on pointer hover and while keyboard focus is inside.
            let mut actions = div()
                .id(format!("recent-actions-{id}"))
                .focusable()
                .absolute()
                .top(px(METRICS.space_1))
                .right(px(METRICS.space_1))
                .flex()
                .items_center()
                .gap(px(METRICS.space_1))
                .rounded(px(METRICS.radius_item))
                .bg(theme.editor.ctl_hover)
                .opacity(0.)
                .group_hover(group.clone(), |style| style.opacity(1.))
                .in_focus(|style| style.opacity(1.));
            for action in recent_row_actions(recent.missing) {
                let entity = cx.entity();
                let target = project_id.clone();
                actions = actions.child(match action {
                    "locate" => EditorButton::plain(&theme, format!("locate-{id}"))
                        .label("Locate…")
                        .filled(&theme)
                        .disabled(!enabled)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.pick(Picker::Locate(target.clone()), cx)
                            });
                        })
                        .into_any_element(),
                    _ => IconButton::editor(&theme, format!("remove-{id}"), Icon::X.path())
                        .editor_size(IconButtonSize::Small)
                        .label("Remove recent")
                        .tooltip(&theme, "Remove recent")
                        .disabled(!enabled)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.dispatch(Command::RemoveRecent(target.clone()), cx)
                            });
                        })
                        .into_any_element(),
                });
            }
            recents = recents.child(div().relative().group(group).child(main).child(actions));
        }
        recents
    }

    fn render_empty_state(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let enabled = !self.busy;
        let actions = div()
            .flex()
            .flex_wrap()
            .gap(px(METRICS.space_2))
            .child(self.action_button(
                "create",
                "Create…",
                enabled,
                true,
                Some(Icon::Plus),
                cx,
                |s, cx| s.pick(Picker::Create, cx),
            ))
            .child(self.action_button(
                "open",
                "Open…",
                enabled,
                false,
                Some(Icon::FolderOpen),
                cx,
                |s, cx| s.pick(Picker::Open, cx),
            ))
            .child(self.action_button(
                "import",
                "Import Rust…",
                enabled,
                false,
                Some(Icon::Upload),
                cx,
                |s, cx| s.pick(Picker::Import, cx),
            ));
        div()
            .id("empty-state")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Card::editor(&self.theme)
                    .w_full()
                    .max_w(px(520.))
                    .max_h_full()
                    .flex()
                    .flex_col()
                    .gap(px(METRICS.space_4))
                    .p(px(METRICS.space_6))
                    .child(heading(&self.theme, "fframes Studio"))
                    .child(muted(
                        &self.theme,
                        "A native workspace for your video projects",
                    ))
                    .child(actions)
                    .child(section_title(&self.theme, "Recent projects"))
                    .child(self.render_recents(cx).max_h(px(260.)).overflow_y_scroll()),
            )
            .into_any_element()
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

    /// The preview, transport and history controls: toolbar, stage and transport
    /// inside one card. The timeline card is placed by `render`.
    fn render_player_area(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let entity = cx.entity();
        let mut player = Card::editor(&self.theme)
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.render_player_toolbar(cx))
            .child(self.render_stage(cx))
            .child(self.render_transport(cx))
            .child(
                // Measures the card so the transport can pick its compact layout.
                canvas(
                    move |bounds, _, cx| {
                        let width = f32::from(bounds.size.width);
                        entity.update(cx, |shell, cx| {
                            if transport_mode(shell.player_width) != transport_mode(width)
                                || show_total_time(shell.player_width) != show_total_time(width)
                            {
                                cx.notify();
                            }
                            shell.player_width = width;
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        if let Some(menu) = self.render_overflow_menu(cx) {
            player = player.child(menu);
        }
        player.into_any_element()
    }

    fn render_player_toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let (label, tone) = match &self.preview_state.status {
            studio_engine::PreviewStatus::Building => {
                ("Building/preparing · prior preview retained", Tone::Info)
            }
            studio_engine::PreviewStatus::Preparing => {
                ("Preparing matching frame and audio", Tone::Info)
            }
            studio_engine::PreviewStatus::Error(_) => {
                ("Preview failed · prior preview retained", Tone::Warning)
            }
            _ => ("CPU preview · shaders unsupported", Tone::Neutral),
        };
        let audio_label = self.output.as_ref().map_or_else(
            || self.audio_note.clone(),
            |output| {
                let metrics = output.metrics();
                format!(
                    "{} · {} Hz · predicted output clock{}{} · latency {:.1}ms · underruns {}",
                    output.device,
                    output.sample_rate,
                    if self.muted { " · muted" } else { "" },
                    if metrics.invalid_timestamps > 0 {
                        " (latency estimate)"
                    } else {
                        ""
                    },
                    metrics.predicted_latency_ms,
                    metrics.underrun_frames
                )
            },
        );
        let audio_ready = self.displayed.is_some();
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            .h(px(METRICS.toolbar_h))
            .px(px(METRICS.space_3))
            .border_b_1()
            .border_color(palette.line)
            .overflow_hidden()
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .child(Badge::new(label).tone(tone)),
            )
            .child(with_tooltip(
                div().id("canvas-hint").flex_none().child(icon(
                    Icon::Info,
                    METRICS.icon_md,
                    palette.text_3,
                )),
                &theme,
                Some(CANVAS_HINT.into()),
            ))
            .child(
                text_style(div(), TextStyle::Small)
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_right()
                    .text_color(palette.text_2)
                    .child(audio_label),
            )
            .child(self.action_button(
                "next-audio-output",
                "Next output",
                audio_ready,
                false,
                None,
                cx,
                |s, cx| s.cycle_output(cx),
            ))
            .child(self.action_button(
                "retry-audio-output",
                "Retry audio",
                audio_ready,
                false,
                None,
                cx,
                |s, cx| s.change_output(OutputDevice::Default, cx),
            ))
            .child(self.action_button(
                "no-audio-output",
                "No audio",
                audio_ready,
                false,
                None,
                cx,
                |s, cx| s.change_output(OutputDevice::Unavailable, cx),
            ))
            .into_any_element()
    }

    /// The stage: the letterboxed frame with its ring, shadow and overlays inside
    /// the `preview-surface` that owns pointer, keyboard and telemetry.
    fn render_stage(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let radius = px(METRICS.radius_frame);
        let mut preview_surface = div()
            .id("preview-surface")
            .relative()
            .tab_index(0)
            .role(gpui::Role::Group)
            .aria_label("Compiled preview")
            .focus_visible(|s| s.shadow(focus_ring(&theme)))
            .on_key_down(cx.listener(|shell, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.alt
                    || event.keystroke.modifiers.control
                    || event.keystroke.modifiers.platform
                    || event.keystroke.modifiers.shift
                {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "space" => shell.toggle_playback(cx),
                    "left" => shell.step_preview(-1, cx),
                    "right" => shell.step_preview(1, cx),
                    "home" => shell.seek_preview(0, cx),
                    "c" => shell.cycle_canvas_selection(cx),
                    "0" => shell.reset_canvas_view(cx),
                    "escape" => {
                        shell.canvas.clear_selection();
                        shell.install_task_scope(shell.timeline.read(cx).selected_scope(), cx);
                        cx.notify();
                    }
                    "end" => {
                        if let Some(d) = &shell.displayed {
                            shell.seek_preview(d.timeline.total_frames, cx);
                        }
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, event: &gpui::MouseDownEvent, _, cx| {
                    shell.canvas_pointer_down(event, cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|shell, event: &gpui::MouseDownEvent, _, cx| {
                    shell.canvas_pointer_down(event, cx)
                }),
            )
            .on_mouse_move(cx.listener(|shell, event: &gpui::MouseMoveEvent, _, cx| {
                shell.canvas_pointer_move(event, cx)
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Left, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Left, cx)),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Middle, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Middle,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Middle, cx)),
            )
            .on_scroll_wheel(cx.listener(|shell, event: &gpui::ScrollWheelEvent, _, cx| {
                shell.canvas_scroll(event, cx)
            }))
            .flex_1()
            .min_h(px(120.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2();
        let entity = cx.entity();
        // The surface is the padded stage's content box; its bounds are what the
        // preview telemetry and click mapping report.
        preview_surface = preview_surface.child(
            canvas(
                move |bounds, window, cx| {
                    let dpi = window.scale_factor();
                    let extent = (
                        (f32::from(bounds.size.width) * dpi).round().max(1.) as u32,
                        (f32::from(bounds.size.height) * dpi).round().max(1.) as u32,
                    );
                    entity.update(cx, |shell, cx| {
                        shell.canvas_origin =
                            (f64::from(bounds.origin.x), f64::from(bounds.origin.y));
                        shell.canvas_extent =
                            (f64::from(bounds.size.width), f64::from(bounds.size.height));
                        shell.preview_extent = extent;
                        if let Some(displayed) = &shell.displayed {
                            let before = shell.canvas.viewport.clone();
                            let _ = shell.canvas.resize(
                                f64::from(bounds.size.width),
                                f64::from(bounds.size.height),
                                displayed.timeline.width as u32,
                                displayed.timeline.height as u32,
                            );
                            if before != shell.canvas.viewport {
                                cx.notify();
                            }
                        }
                        if shell.displayed.as_ref().is_some_and(|d| {
                            crate::frame_image::preview_scale(
                                d.timeline.width,
                                d.timeline.height,
                                extent,
                            ) != shell.preview_state.scale()
                        }) {
                            shell.request_frame(shell.transport.position(), cx);
                        }
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
        if let Some(image) = self.images.current_image() {
            let image = gpui::img(image).rounded(radius);
            preview_surface = if let Some(layout) = self
                .canvas
                .viewport
                .as_ref()
                .map(StageLayout::from_viewport)
            {
                let frame = layout.frame_rect;
                let placed = |element: gpui::Div, rect: studio_engine::CanvasRect| {
                    element
                        .absolute()
                        .left(px(rect.x as f32))
                        .top(px(rect.y as f32))
                        .w(px(rect.width as f32))
                        .h(px(rect.height as f32))
                };
                // Overlays live inside the frame, so they clip to it and use frame
                // coordinates.
                let mut content = placed(div(), frame)
                    .overflow_hidden()
                    .child(image.size_full());
                if let Some(bounds) = self.canvas.selected_bounds() {
                    content = content.child(
                        placed(div(), layout.to_frame_space(bounds))
                            .border_2()
                            .border_color(palette.accent)
                            .bg(theme.selection()),
                    );
                }
                if let (Some(viewport), Some(rect)) =
                    (&self.canvas.viewport, &self.canvas.rectangle_scope)
                {
                    content = content.child(
                        placed(
                            div(),
                            layout.to_frame_space(viewport.video_scope_bounds(rect)),
                        )
                        .border_2()
                        .border_color(theme.amber_11),
                    );
                }
                preview_surface
                    .child(
                        placed(div(), frame)
                            .rounded(radius)
                            .shadow(shadow_for(theme.appearance, Shadow::Preview).into_vec()),
                    )
                    .child(content)
                    .child(
                        placed(div(), frame)
                            .rounded(radius)
                            .border_1()
                            .border_color(palette.line),
                    )
            } else {
                preview_surface.child(image.size_full())
            };
        } else {
            preview_surface = preview_surface
                .child(
                    text_style(div(), TextStyle::Title)
                        .text_color(palette.text_1)
                        .child(if self.displayed.is_some() {
                            "Empty video"
                        } else {
                            "No preview available"
                        }),
                )
                .child(
                    text_style(div(), TextStyle::Body)
                        .text_color(palette.text_2)
                        .child("Build preview to compile the project's current source."),
                );
        }
        let mut stage = div()
            .id("stage")
            .relative()
            .flex_1()
            .min_h(px(120.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .p(px(METRICS.space_4))
            .bg(palette.stage)
            .child(preview_surface);
        if let Some(message) = &self.canvas.message {
            stage = stage.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(px(METRICS.space_4))
                    .flex()
                    .justify_center()
                    .child(
                        text_style(div(), TextStyle::Small)
                            .px(px(METRICS.space_3))
                            .py(px(METRICS.space_1))
                            .rounded(px(METRICS.radius_control))
                            .border_1()
                            .border_color(palette.line)
                            .bg(palette.card)
                            .text_color(palette.text_2)
                            .child(message.clone()),
                    ),
            );
        }
        stage.into_any_element()
    }

    fn render_transport(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let enabled = !self.busy;
        let project = self.presentation.project.clone();
        let mode = transport_mode(self.player_width);
        let has_frames = self
            .displayed
            .as_ref()
            .is_some_and(|d| d.timeline.total_frames > 0);
        let playing = self.transport.playing() || self.resume_install;
        let readout = match &self.displayed {
            Some(d) => {
                let fps = d.timeline.fps as f64;
                let current = format_clock(self.preview_state.position() as f64 / fps);
                let text = if show_total_time(self.player_width) {
                    format!(
                        "{current} / {}",
                        format_clock(d.timeline.total_frames as f64 / fps)
                    )
                } else {
                    current
                };
                let tip = format!(
                    "Displayed {} · frame {} / {} · {:.3}s{}",
                    &d.identity().source_revision[..12.min(d.identity().source_revision.len())],
                    self.preview_state.position(),
                    d.timeline.total_frames,
                    self.preview_state.position() as f64 / fps,
                    if project
                        .as_ref()
                        .is_some_and(|p| p.source.as_str() != d.identity().source_revision)
                    {
                        " · prior revision"
                    } else {
                        ""
                    }
                );
                (text, Some(tip))
            }
            None => ("0:00.00 / 0:00.00".to_owned(), None),
        };
        let readout_style = if mode == TransportMode::Compact {
            TextStyle::Label
        } else {
            TextStyle::Body
        };
        let can_build = enabled && project.as_ref().is_some_and(|p| p.worker_available);
        let can_cancel = self.preview.is_some();
        let (build, cancel) = match mode {
            TransportMode::Full => (
                self.action_button(
                    "build-preview",
                    "Build preview",
                    can_build,
                    true,
                    Some(Icon::Hammer),
                    cx,
                    |s, cx| s.dispatch(Command::Build, cx),
                )
                .into_any_element(),
                self.action_button(
                    "cancel-preview",
                    "Cancel build",
                    can_cancel,
                    false,
                    Some(Icon::Square),
                    cx,
                    |s, cx| s.dispatch(Command::CancelBuild, cx),
                )
                .into_any_element(),
            ),
            TransportMode::Compact => (
                self.action_icon_button(
                    "build-preview",
                    "Build preview",
                    Icon::Hammer,
                    IconButtonSize::Medium,
                    can_build,
                    cx,
                    |s, cx| s.dispatch(Command::Build, cx),
                )
                .accent(&self.theme)
                .into_any_element(),
                self.action_icon_button(
                    "cancel-preview",
                    "Cancel build",
                    Icon::Square,
                    IconButtonSize::Medium,
                    can_cancel,
                    cx,
                    |s, cx| s.dispatch(Command::CancelBuild, cx),
                )
                .ctl_fill(&self.theme)
                .into_any_element(),
            ),
        };
        let overflow = self.overflow_trigger(cx);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            .h(px(METRICS.transport_h))
            .px(px(METRICS.space_3))
            .border_t_1()
            .border_color(palette.line)
            .overflow_hidden()
            .child(with_tooltip(
                text_style(div().id("time-readout"), readout_style)
                    .flex_none()
                    .min_w(px(if mode == TransportMode::Compact {
                        60.
                    } else {
                        120.
                    }))
                    .text_color(palette.text_1)
                    .child(readout.0),
                &theme,
                readout.1.map(Into::into),
            ))
            .child(div().flex_1())
            .child(self.action_icon_button(
                "previous-frame",
                "Previous frame",
                Icon::StepBack,
                IconButtonSize::Small,
                has_frames,
                cx,
                |s, cx| s.step_preview(-1, cx),
            ))
            .child(self.action_icon_button(
                "play-pause",
                if playing { "Pause" } else { "Play" },
                if playing { Icon::Pause } else { Icon::Play },
                IconButtonSize::Large,
                has_frames,
                cx,
                |s, cx| s.toggle_playback(cx),
            ))
            .child(self.action_icon_button(
                "next-frame",
                "Next frame",
                Icon::StepForward,
                IconButtonSize::Small,
                has_frames,
                cx,
                |s, cx| s.step_preview(1, cx),
            ))
            .child(self.action_icon_button(
                "mute-audio",
                if self.muted { "Unmute (M)" } else { "Mute (M)" },
                if self.muted {
                    Icon::VolumeX
                } else {
                    Icon::Volume2
                },
                IconButtonSize::Medium,
                self.displayed.is_some(),
                cx,
                |s, cx| s.toggle_mute(cx),
            ))
            .child(div().flex_1())
            .child(build)
            .child(cancel)
            .child(overflow)
            .into_any_element()
    }

    /// The overflow menu's rows: the action ids identify them, unavailable ones are
    /// disabled.
    fn overflow_items(&self) -> Vec<MenuItem> {
        let actions = self.current_overflow_actions();
        actions
            .iter()
            .zip(disabled_rows(&actions))
            .map(|((action, _), disabled)| {
                MenuItem::new(action.label(), false)
                    .id(action.id())
                    .disabled(disabled)
            })
            .collect()
    }

    /// Actions behind the overflow button, in menu order.
    fn current_overflow_actions(&self) -> Vec<(OverflowAction, bool)> {
        let project = self.presentation.project.as_ref();
        overflow_actions(
            !self.busy,
            project.is_some(),
            project.is_some_and(|p| p.draft.is_some()),
        )
    }

    fn overflow_trigger(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let entity = cx.entity();
        let anchor = self.overflow_anchor.clone();
        IconButton::editor(&theme, "overflow-actions", Icon::MoreHorizontal.path())
            .editor_size(IconButtonSize::Medium)
            .label("More actions")
            .tooltip(&theme, "More actions")
            .on_bounds(move |bounds, _| anchor.set(Some(bounds)))
            .on_click(move |_, window, cx| {
                entity.update(cx, |shell, cx| {
                    if shell.overflow_open {
                        shell.close_overflow(window, cx);
                    } else {
                        shell.open_overflow(window, cx);
                    }
                });
            })
            .into_any_element()
    }

    fn open_overflow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.overflow_items();
        self.overflow_menu = match self.overflow_anchor.get() {
            Some(trigger) => MenuState::anchored(trigger, &items),
            None => MenuState::new(gpui::Point::default(), &items),
        };
        self.overflow_open = true;
        window.focus(&self.overflow_focus, cx);
        cx.notify();
    }

    fn close_overflow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overflow_open {
            self.overflow_open = false;
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// Marks the open menu's rows from current availability (nothing is cached), so it
    /// never shows or keyboard-navigates a stale enable flag.
    fn sync_overflow(&mut self) {
        let disabled = disabled_rows(&self.current_overflow_actions());
        self.overflow_menu.set_disabled(disabled);
    }

    fn run_overflow(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(action) = enabled_overflow_action(&self.current_overflow_actions(), index) else {
            return;
        };
        self.close_overflow(window, cx);
        let project = self.presentation.project.clone();
        match action {
            OverflowAction::Refresh => self.dispatch(Command::Refresh, cx),
            OverflowAction::Checkpoint => self.dispatch(Command::Checkpoint, cx),
            OverflowAction::Restore => self.pick(Picker::Restore, cx),
            OverflowAction::RevealDraft => {
                if let Some(draft) = project.and_then(|p| p.draft) {
                    cx.reveal_path(&draft);
                }
            }
            OverflowAction::RevealCheckpoint => {
                if let Some(project) = project {
                    cx.reveal_path(&project.checkpoint);
                }
            }
        }
    }

    /// Every studio action, handled at the root so it reaches the shell from anywhere in
    /// the window.
    fn on_actions(root: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        root.on_action(cx.listener(|s, _: &actions::OpenProject, _, cx| s.pick(Picker::Open, cx)))
            .on_action(
                cx.listener(|s, _: &actions::BuildPreview, _, cx| s.dispatch(Command::Build, cx)),
            )
            .on_action(cx.listener(|s, _: &actions::CancelPreview, _, cx| {
                s.dispatch(Command::CancelBuild, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::ExportVideo, _, cx| s.pick(Picker::Export, cx)))
            .on_action(cx.listener(|s, _: &actions::CancelExport, _, cx| s.cancel_export(cx)))
            .on_action(cx.listener(|s, action: &actions::ApplyPreset, _, cx| {
                let key = action.key.clone();
                s.run_preset(PresetCommand::Apply { key }, cx)
            }))
            .on_action(cx.listener(|s, action: &actions::ResetPreset, _, cx| {
                let key = action.key.clone();
                s.run_preset(PresetCommand::Reset { key }, cx)
            }))
            .on_action(cx.listener(|s, action: &actions::SetStyleOverride, _, cx| {
                let (token, value) = (action.token.clone(), action.value.clone());
                s.run_preset(PresetCommand::SetOverride { token, value }, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::SaveCheckpoint, _, cx| {
                s.dispatch(Command::Checkpoint, cx)
            }))
            .on_action(
                cx.listener(|s, _: &actions::RestoreCheckpoint, _, cx| s.pick(Picker::Restore, cx)),
            )
            .on_action(cx.listener(|s, _: &actions::SelectSdk, _, cx| s.pick(Picker::Sdk, cx)))
            .on_action(
                cx.listener(|s, _: &actions::ImportSdkBundle, _, cx| s.pick(Picker::SdkBundle, cx)),
            )
            .on_action(cx.listener(|s, _: &actions::DetectAgents, _, cx| {
                s.panel.update(cx, |panel, cx| panel.detect_agents(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::FocusPrompt, window, cx| {
                s.panel
                    .update(cx, |panel, cx| panel.focus_prompt(window, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::SubmitPrompt, _, cx| {
                s.panel.update(cx, |panel, cx| panel.submit(cx))
            }))
            .on_action(cx.listener(|s, action: &actions::AnswerPermission, _, cx| {
                let option = action.option.clone();
                s.panel
                    .update(cx, |panel, cx| panel.answer_oldest_permission(option, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ApplyCandidate, _, cx| {
                s.panel.update(cx, |panel, cx| panel.apply(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ToggleReviewPolicy, _, cx| {
                s.panel
                    .update(cx, |panel, cx| panel.toggle_review_policy(cx))
            }))
            .on_action(cx.listener(|s, action: &actions::UseDetectedAgent, _, cx| {
                s.panel
                    .update(cx, |panel, cx| panel.use_detected(&action.profile, cx))
            }))
            .on_action(cx.listener(|s, action: &actions::SelectCanvasAt, _, cx| {
                s.pause_at_painted_frame(cx);
                match s.video_to_canvas(action.x, action.y) {
                    Some(point) => s.select_canvas(point, action.cycle, cx),
                    None => {
                        s.canvas.message = Some("The video is not on the canvas yet.".into());
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|s, _: &actions::TogglePlayback, _, cx| s.toggle_playback(cx)))
            .on_action(cx.listener(|s, _: &actions::StepForward, _, cx| s.step_preview(1, cx)))
            .on_action(cx.listener(|s, _: &actions::StepBackward, _, cx| s.step_preview(-1, cx)))
            .on_action(cx.listener(|s, _: &actions::ToggleMute, _, cx| s.toggle_mute(cx)))
            .on_action(cx.listener(|s, _: &actions::NextAudioOutput, _, cx| s.cycle_output(cx)))
            .on_action(cx.listener(|s, _: &actions::RetryAudioOutput, _, cx| {
                s.change_output(OutputDevice::Default, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::DisableAudioOutput, _, cx| {
                s.change_output(OutputDevice::Unavailable, cx)
            }))
            .on_action(cx.listener(|s, action: &actions::Seek, _, cx| {
                s.timeline.update(cx, |t, cx| t.seek(action.frame, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::SeekToStart, _, cx| {
                s.timeline.update(cx, |t, cx| t.seek(0, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::SeekToEnd, _, cx| {
                s.timeline.update(cx, |t, cx| t.seek_to_end(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::CycleScene, _, cx| {
                s.timeline.update(cx, |t, cx| t.cycle_scene(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::MarkRangeStart, _, cx| {
                s.timeline.update(cx, |t, cx| t.mark_range_start(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::MarkRangeEnd, _, cx| {
                s.timeline.update(cx, |t, cx| t.mark_range_end(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ZoomIn, _, cx| {
                s.timeline.update(cx, |t, cx| t.zoom(2., cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ZoomOut, _, cx| {
                s.timeline.update(cx, |t, cx| t.zoom(0.5, cx))
            }))
    }

    fn render_overflow_menu(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if !self.overflow_open {
            return None;
        }
        let select = cx.entity();
        let highlight = cx.entity();
        let dismiss = cx.entity();
        Some(
            Menu::editor(
                &self.theme,
                "overflow-menu",
                self.overflow_items(),
                &self.overflow_menu,
            )
            .focus(&self.overflow_focus)
            .on_highlight(move |index, _, cx| {
                highlight.update(cx, |shell, cx| {
                    shell.overflow_menu.set_highlighted(*index);
                    cx.notify();
                });
            })
            .on_select(move |index, window, cx| {
                select.update(cx, |shell, cx| shell.run_overflow(*index, window, cx));
            })
            .on_dismiss(move |window, cx| {
                dismiss.update(cx, |shell, cx| shell.close_overflow(window, cx));
            })
            .into_any_element(),
        )
    }

    fn render_agent_column(&self) -> gpui::AnyElement {
        let theme = self.theme;
        div()
            .w(px(METRICS.agent_panel_w))
            .flex_none()
            .min_h_0()
            .overflow_hidden()
            .rounded(px(METRICS.radius_card))
            .border_1()
            .border_color(theme.editor.line)
            .bg(theme.editor.card)
            .child(self.panel.clone())
            .into_any_element()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Project,
    Assets,
    Style,
}

/// What a left-column tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftContent {
    Files,
    Assets,
    Presets,
}

/// Height of a file or asset row.
const LIST_ROW_H: f32 = 28.;

fn left_tab_content(tab: LeftTab) -> LeftContent {
    match tab {
        LeftTab::Project => LeftContent::Files,
        LeftTab::Assets => LeftContent::Assets,
        LeftTab::Style => LeftContent::Presets,
    }
}

/// The actions of a recent-project row, as the id prefix of each button: a missing
/// project can be located, every project can be removed.
fn recent_row_actions(missing: bool) -> Vec<&'static str> {
    if missing {
        vec!["locate", "remove"]
    } else {
        vec!["remove"]
    }
}

fn is_terminal(event: &ExportProgress) -> bool {
    matches!(
        event,
        ExportProgress::Complete { .. } | ExportProgress::Failed(_) | ExportProgress::Cancelled
    )
}

/// Dot tone of the managed SDK line: busy, ready or missing.
fn sdk_status_tone(busy: bool, ready: bool) -> Tone {
    if busy {
        Tone::Info
    } else if ready {
        Tone::Success
    } else {
        Tone::Warning
    }
}

impl LeftTab {
    const ALL: [LeftTab; 3] = [LeftTab::Project, LeftTab::Assets, LeftTab::Style];

    /// Also the tab's element id.
    fn name(self) -> &'static str {
        match self {
            LeftTab::Project => "Project",
            LeftTab::Assets => "Assets",
            LeftTab::Style => "Style",
        }
    }

    fn icon(self) -> Icon {
        match self {
            LeftTab::Project => Icon::Folder,
            LeftTab::Assets => Icon::Image,
            LeftTab::Style => Icon::Palette,
        }
    }

    fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(LeftTab::Project)
    }
}

/// Ids of the header actions. Create, open and import live in the empty state while
/// no project is open, so each id is rendered once.
fn header_actions(project_open: bool) -> Vec<&'static str> {
    if project_open {
        vec!["create", "open", "import", "export-mp4", "close"]
    } else {
        vec!["export-mp4", "close"]
    }
}

/// The status strip opens when asked to, while a job is active and on any error.
fn status_strip_expanded(job: &str, has_error: bool, user_open: bool) -> bool {
    user_open || has_error || matches!(job, "Queued" | "Running" | "Cancelling")
}

/// First line as the notice title; the full text as details when there is more.
fn error_title_details(text: &str) -> (String, Option<String>) {
    let title = text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text);
    let details = (text.trim() != title.trim()).then(|| text.to_owned());
    (title.trim().to_owned(), details)
}

const CANVAS_HINT: &str = "Select: click · Rectangle: Shift-drag · Cycle: Alt-click/C · Zoom: Ctrl-wheel · Pan: middle-drag · Fit: 0";

/// Player width under which the transport uses icon-only Build and Cancel.
const TRANSPORT_COMPACT_BELOW: f32 = 760.;
/// Player width under which the time readout drops the total.
const TRANSPORT_TOTAL_HIDDEN_BELOW: f32 = 620.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportMode {
    Full,
    Compact,
}

fn transport_mode(player_width: f32) -> TransportMode {
    if player_width < TRANSPORT_COMPACT_BELOW {
        TransportMode::Compact
    } else {
        TransportMode::Full
    }
}

fn show_total_time(player_width: f32) -> bool {
    player_width >= TRANSPORT_TOTAL_HIDDEN_BELOW
}

/// `m:ss.cc`, truncated to centiseconds.
fn format_clock(seconds: f64) -> String {
    let centis = (seconds.max(0.) * 100.).floor() as u64;
    format!(
        "{}:{:02}.{:02}",
        centis / 6000,
        centis / 100 % 60,
        centis % 100
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverflowAction {
    Refresh,
    Checkpoint,
    Restore,
    RevealDraft,
    RevealCheckpoint,
}

impl OverflowAction {
    /// The id the action's button had before it moved into the menu.
    fn id(self) -> &'static str {
        match self {
            Self::Refresh => "refresh",
            Self::Checkpoint => "checkpoint",
            Self::Restore => "restore",
            Self::RevealDraft => "draft",
            Self::RevealCheckpoint => "reveal-checkpoint",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Refresh => "Refresh",
            Self::Checkpoint => "Save checkpoint",
            Self::Restore => "Restore as copy…",
            Self::RevealDraft => "Reveal retained draft",
            Self::RevealCheckpoint => "Reveal checkpoint",
        }
    }
}

/// Every overflow entry with whether it is available now, under the enable rules the
/// buttons had: refresh needs an idle shell; checkpoint and restore need an idle shell and
/// a project; the draft needs a retained draft; the checkpoint reveal needs a project.
/// Unavailable entries stay in the menu as disabled rows.
fn overflow_actions(idle: bool, has_project: bool, has_draft: bool) -> Vec<(OverflowAction, bool)> {
    vec![
        (OverflowAction::Refresh, idle),
        (OverflowAction::Checkpoint, idle && has_project),
        (OverflowAction::Restore, idle && has_project),
        (OverflowAction::RevealDraft, has_project && has_draft),
        (OverflowAction::RevealCheckpoint, has_project),
    ]
}

/// The menu's disabled flags for `actions`, in menu order.
fn disabled_rows(actions: &[(OverflowAction, bool)]) -> Vec<bool> {
    actions.iter().map(|(_, enabled)| !enabled).collect()
}

/// The action at `index` when it is currently available.
fn enabled_overflow_action(
    actions: &[(OverflowAction, bool)],
    index: usize,
) -> Option<OverflowAction> {
    match actions.get(index) {
        Some((action, true)) => Some(*action),
        _ => None,
    }
}

impl Render for StudioShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme.refresh(window, cx);
        if self.overflow_open {
            self.sync_overflow();
        }
        if self.clear_images {
            self.images.clear(window);
            self.clear_images = false;
        }
        if let Some(install) = self.pending_ready.take() {
            let ready = &install.ready.clone();
            let audio_ready = install.audio_ready;
            let mut slot = Some(install);
            // No filesystem work here. The controller's nonblocking guard is the source
            // fence and it is held from the final `can_install` check through the
            // coordinator commit and the preview-state install: a workflow publication
            // or a source reconcile cannot interleave, and every check reads the guard's
            // current state, never a copy taken when preparation finished. Unavailable
            // (a job holds the controller, or a source scan is pending) means deferred.
            let engine = self.engine.clone();
            let outcome = audio_ready.then(|| {
                engine.with_install_fence(|source| {
                    let mut committed = false;
                    if self.presentation.state.as_ref().is_some_and(|s| {
                        s.session() == source.session()
                            && s.source() == source.source()
                            && s.generation() == source.generation()
                    }) && self.preview_state.can_install(ready, source).is_ok()
                    {
                        let image = ready
                            .frame
                            .as_ref()
                            .map(|f| create_render_image(&f.response.header, &f.pixels))
                            .transpose();
                        match image {
                            Ok(image) => {
                                let model =
                                    studio_engine::TimelineModel::new(ready.timeline.clone());
                                if let Err(e) = &model {
                                    self.error = Some(e.clone());
                                }
                                if self.preview.as_ref().is_some_and(|p| {
                                    model.is_ok()
                                        && p.commit(ready.identity().clone(), ready.seek_serial)
                                }) && self.preview_state.install(ready, source).is_ok()
                                {
                                    if let Some(image) = image {
                                        self.images.replace_image(image, window);
                                    } else {
                                        self.images.clear(window);
                                    }
                                    let install = slot.take().expect("taken once");
                                    self.transport = install.clock;
                                    if self.resume_install
                                        && ready.position < ready.timeline.total_frames
                                    {
                                        let _ = self
                                            .transport
                                            .toggle(self.clock_start.elapsed().as_secs_f64());
                                    }
                                    self.resume_install = false;
                                    self.output = install.audio;
                                    self.audio_ready_epoch = Some(self.transport.epoch());
                                    self.requested_clock_epoch = self.transport.epoch();
                                    self.presented_serial = ready.seek_serial;
                                    self.painted_frame =
                                        ready.frame.as_ref().map(|f| f.response.frame_index);
                                    if let Some(frame) = ready.frame.as_ref() {
                                        if let Err(error) = self
                                            .canvas
                                            .install_frame(ready.identity().clone(), frame)
                                        {
                                            self.canvas.clear();
                                            self.canvas.message = Some(format!(
                                                "Canvas metadata unavailable: {error}"
                                            ));
                                        }
                                    } else {
                                        self.canvas.clear();
                                    }
                                    self.timeline.update(cx, |v, cx| {
                                        v.install(model.unwrap(), ready.position, cx)
                                    });
                                    self.displayed = Some(ready.clone());
                                    if let Some(workflow) = &self.agent.workflow {
                                        let _ = workflow.set_displayed_preview_identity(Some(
                                            ready.identity().clone(),
                                        ));
                                    }
                                    if ready.tag().base_source == *source.source() {
                                        self.awaiting_preview = None;
                                    }
                                    // Tell the workflow what is on screen so its accepted-awaiting
                                    // label clears exactly when the matching preview shows.
                                    self.agent.displayed =
                                        Some(ready.identity().source_revision.clone());
                                    self.error = None;
                                    // The commit dropped the queued thumbnails of the old preview.
                                    self.thumbnails_sent = None;
                                    committed = true;
                                }
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        }
                    }
                    committed
                })
            });
            match outcome.flatten() {
                // Deferred, not rejected.
                None => self.pending_ready = slot,
                Some(true) => self.report_displayed(),
                Some(false) => {
                    self.audio.stop(self.transport.epoch());
                    self.discard_install(cx);
                }
            }
        }
        if let Some(f) = self.pending_frame.take()
            && self.preview_state.accepts_frame(&f)
            && self.requested_clock_epoch == self.transport.epoch()
        {
            match create_render_image(&f.response.header, &f.pixels) {
                Ok(image) => {
                    self.images.replace_image(image, window);
                    if let Some(preview) = self
                        .displayed
                        .as_ref()
                        .map(|ready| ready.identity().clone())
                    {
                        if let Err(error) = self.canvas.install_frame(preview, &f) {
                            self.canvas.clear();
                            self.canvas.message =
                                Some(format!("Canvas metadata unavailable: {error}"));
                        }
                    } else {
                        self.canvas.clear();
                    }
                    self.presented_serial = f.response.seek_serial;
                    self.painted_frame = Some(f.response.frame_index);
                    self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        self.activate_audio(cx);
        let theme = self.theme;
        let gutter = px(METRICS.gutter);
        let project_open = self.presentation.project.is_some();
        let header = self.render_header(cx);
        let notices = self.render_notices(cx);
        let strip = self.render_status_strip(cx);
        let left = self.render_left_column(cx);
        let main = if project_open {
            let player = self.render_player_area(cx);
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .flex()
                .flex_col()
                .gap(gutter)
                .child(player)
                .child(
                    Card::editor(&self.theme)
                        .flex_none()
                        .overflow_hidden()
                        .child(self.timeline.clone()),
                )
                .into_any_element()
        } else {
            self.render_empty_state(cx)
        };
        let agent = self.render_agent_column();
        Self::on_actions(div(), cx)
            .size_full()
            .track_focus(&self.focus)
            .tab_group()
            .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .bg(theme.editor.window)
            .text_color(theme.editor.text_1)
            .flex()
            .flex_col()
            .p(gutter)
            .gap(gutter)
            .child(header)
            .children(notices)
            .children(strip)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(gutter)
                    .child(left)
                    .child(main)
                    .child(agent),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_mode_switches_below_760() {
        assert_eq!(transport_mode(1200.), TransportMode::Full);
        assert_eq!(transport_mode(760.), TransportMode::Full);
        assert_eq!(transport_mode(759.9), TransportMode::Compact);
        assert!(show_total_time(620.));
        assert!(!show_total_time(619.));
    }

    #[test]
    fn clock_formats_minutes_seconds_centiseconds() {
        assert_eq!(format_clock(0.), "0:00.00");
        assert_eq!(format_clock(1.2), "0:01.20");
        assert_eq!(format_clock(5.5), "0:05.50");
        assert_eq!(format_clock(125.999), "2:05.99");
        assert_eq!(format_clock(-3.), "0:00.00");
    }

    #[test]
    fn a_click_on_an_action_that_became_unavailable_is_inert() {
        use OverflowAction::*;
        let idle = overflow_actions(true, true, false);
        assert_eq!(enabled_overflow_action(&idle, 1), Some(Checkpoint));
        // The shell went busy after the menu opened.
        let busy = overflow_actions(false, true, false);
        assert_eq!(enabled_overflow_action(&busy, 1), None);
        assert_eq!(enabled_overflow_action(&busy, 4), Some(RevealCheckpoint));
        assert_eq!(enabled_overflow_action(&busy, 9), None);
    }

    #[test]
    fn overflow_actions_follow_the_old_enable_rules() {
        use OverflowAction::*;
        assert_eq!(
            overflow_actions(true, true, true),
            vec![
                (Refresh, true),
                (Checkpoint, true),
                (Restore, true),
                (RevealDraft, true),
                (RevealCheckpoint, true)
            ]
        );
        assert_eq!(
            overflow_actions(true, true, false),
            vec![
                (Refresh, true),
                (Checkpoint, true),
                (Restore, true),
                (RevealDraft, false),
                (RevealCheckpoint, true)
            ]
        );
        assert_eq!(
            overflow_actions(true, false, false),
            vec![
                (Refresh, true),
                (Checkpoint, false),
                (Restore, false),
                (RevealDraft, false),
                (RevealCheckpoint, false)
            ]
        );
        assert_eq!(
            overflow_actions(false, true, false),
            vec![
                (Refresh, false),
                (Checkpoint, false),
                (Restore, false),
                (RevealDraft, false),
                (RevealCheckpoint, true)
            ]
        );
        assert!(
            overflow_actions(false, false, false)
                .iter()
                .all(|(_, on)| !on)
        );
        let ids: Vec<_> = overflow_actions(true, true, true)
            .iter()
            .map(|(a, _)| a.id())
            .collect();
        assert_eq!(
            ids,
            [
                "refresh",
                "checkpoint",
                "restore",
                "draft",
                "reveal-checkpoint"
            ]
        );
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn header_hides_project_pickers_while_the_empty_state_shows() {
        assert_eq!(header_actions(false), ["export-mp4", "close"]);
        assert_eq!(
            header_actions(true),
            ["create", "open", "import", "export-mp4", "close"]
        );
    }

    #[test]
    fn status_strip_expands_for_errors_and_active_jobs() {
        assert!(!status_strip_expanded("Idle", false, false));
        assert!(!status_strip_expanded("Succeeded", false, false));
        assert!(status_strip_expanded("Idle", true, false));
        assert!(status_strip_expanded("Running", false, false));
        assert!(status_strip_expanded("Cancelling", false, false));
        assert!(status_strip_expanded("Idle", false, true));
    }

    #[test]
    fn error_notice_splits_title_and_details() {
        assert_eq!(error_title_details("boom"), ("boom".into(), None));
        let (title, details) = error_title_details("boom\nmore context");
        assert_eq!(title, "boom");
        assert_eq!(details.as_deref(), Some("boom\nmore context"));
    }

    #[test]
    fn left_tab_content_maps_each_tab() {
        assert_eq!(left_tab_content(LeftTab::Project), LeftContent::Files);
        assert_eq!(left_tab_content(LeftTab::Assets), LeftContent::Assets);
        assert_eq!(left_tab_content(LeftTab::Style), LeftContent::Presets);
    }

    #[test]
    fn recent_rows_offer_locate_only_when_missing() {
        assert_eq!(recent_row_actions(true), ["locate", "remove"]);
        assert_eq!(recent_row_actions(false), ["remove"]);
    }

    #[test]
    fn sdk_status_tone_prefers_busy_then_ready() {
        assert_eq!(sdk_status_tone(true, true), Tone::Info);
        assert_eq!(sdk_status_tone(false, true), Tone::Success);
        assert_eq!(sdk_status_tone(false, false), Tone::Warning);
    }

    #[test]
    fn left_tabs_keep_their_element_ids() {
        let names: Vec<_> = LeftTab::ALL.iter().map(|t| t.name()).collect();
        assert_eq!(names, ["Project", "Assets", "Style"]);
        for (index, tab) in LeftTab::ALL.into_iter().enumerate() {
            assert_eq!(LeftTab::from_index(index), tab);
        }
        assert_eq!(LeftTab::from_index(9), LeftTab::Project);
    }
}
