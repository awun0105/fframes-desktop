//! Opens the studio window on a fixture project: the real engine, the deterministic Python
//! preview worker compiled by a fake compiler, and app data, SDK home and writer ownership
//! of the test's own. Tests drive it with actions and read its telemetry; this file is the
//! only one that knows the window's root view.
use gpui::{Action, AnyWindowHandle, AppContext, TestAppContext, WindowHandle};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use studio_bootstrap::{ProcessTreeManager, WriterOwnership};
use studio_engine::containment::OwnershipPolicy;
use studio_engine::engine::backend::StudioEnvironment;
use studio_engine::providers::host::{self, AdapterFile, McpChoice};
use studio_engine::services::build_service::{BuildLimits, BuildService};
use studio_ui::shell::StudioShell;

use crate::build_fixture::{FAKE_CONFIG, FakeCompiler, fake_sdk, manifest};
use studio_engine::{app_paths::AppPaths, build_materialization::sdk_pin};
use studio_sdk::CompatibilityManifest;

/// Ninety frames at 30 fps in two scenes, with a one-second tone from 0 s. An export
/// renders its first frame and holds until it is cancelled.
pub const FIXTURE_CONFIG: &str = r#"{"frames":90,"fps":30,"scenes":[[0,45],[45,90]],"tracks":[[0.0,1.0]],"audio":"tone","export_hang":true}"#;

const WAIT: Duration = Duration::from_secs(60);
/// How far the fake test clock moves per step; unrelated to any production timer.
const CLOCK_STEP: Duration = Duration::from_millis(16);

pub struct Studio {
    pub cx: TestAppContext,
    window: WindowHandle<StudioShell>,
    /// The window that clicks, keys and text go to: the main window or the settings window.
    target: AnyWindowHandle,
    pub compiler: Arc<FakeCompiler>,
    pub project: PathBuf,
    pub app_data: PathBuf,
    pub sdk: PathBuf,
    pub temp: tempfile::TempDir,
}

impl Studio {
    /// A window with nothing open yet.
    pub fn new(cx: &mut TestAppContext) -> Self {
        Self::with_config(cx, FIXTURE_CONFIG)
    }

    /// [`Self::new`] with the fake worker configured by `config` (see
    /// `fake-preview-worker.py`).
    pub fn with_config(cx: &mut TestAppContext, config: &str) -> Self {
        Self::build(cx, config, "active", true)
    }

    /// A window whose managed SDK home is empty: the fixture's SDK is installed in
    /// `<sdk_home>/elsewhere` ([`Self::sdk`]), for the user to select.
    pub fn without_managed_sdk(cx: &mut TestAppContext) -> Self {
        Self::build(cx, FIXTURE_CONFIG, "elsewhere", true)
    }

    /// A first launch: nothing recorded in the app settings and no agent selected, so the
    /// first-run setup opens (the fixture's SDK is installed).
    pub fn first_run(cx: &mut TestAppContext) -> Self {
        Self::build(cx, FIXTURE_CONFIG, "active", false)
    }

    /// `onboarded`: the app settings record the first-run setup as done, so the window
    /// opens on the workspace like every test before onboarding existed.
    fn build(cx: &mut TestAppContext, config: &str, sdk_folder: &str, onboarded: bool) -> Self {
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().unwrap();
        let sdk_home = temp.path().join("sdk-home");
        let (sdk, compatibility) = install_sdk(&sdk_home, sdk_folder);
        let project = temp.path().join("video");
        studio_project::create(
            &project,
            "Video",
            sdk_pin(&compatibility),
            &compatibility.fframes_version,
            &compatibility.runtime_version,
        )
        .unwrap();
        fs::write(project.join(FAKE_CONFIG), config).unwrap();
        let app_data = temp.path().join("data");
        if onboarded {
            fs::create_dir_all(&app_data).unwrap();
            fs::write(
                app_data.join("app-settings.json"),
                r#"{"onboarding_completed":true}"#,
            )
            .unwrap();
        }
        let compiler = FakeCompiler::new(true);
        let environment = StudioEnvironment {
            app_data: Some(app_data.clone()),
            sdk_home: Some(sdk_home),
            builds: BuildService::new(
                ProcessTreeManager::new(),
                compiler.clone(),
                BuildLimits::default(),
            ),
            // The scripted adapter's writers count as contained, as in the workflow
            // fixtures; production takes this only from validated qualification evidence.
            ownership: OwnershipPolicy::TestInjected(WriterOwnership::ProcessGroupContained {
                qualification: "scripted-fixture".into(),
            }),
        };
        let window = cx.update(|cx| {
            studio_ui::shell::bind_keys(cx);
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    StudioShell::with_environment(
                        environment,
                        env!("CARGO_PKG_VERSION"),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        let mut studio = Self {
            cx: cx.clone(),
            window,
            target: window.into(),
            compiler,
            project,
            app_data,
            sdk,
            temp,
        };
        studio.wait("app storage", |t| t["busy"] == false);
        studio
    }

    /// A window with the fixture project open through the Open picker.
    pub fn open(cx: &mut TestAppContext) -> Self {
        Self::new(cx).open_project()
    }

    /// The fixture open with the scripted ACP agent (`tests/support/acp-agent.py`) as its
    /// saved adapter. The agent records what it receives in [`Self::agent_evidence`] and
    /// plays the turns written to `plan.json` there.
    pub fn with_agent(cx: &mut TestAppContext) -> Self {
        let studio = Self::new(cx);
        let evidence = studio.agent_evidence();
        fs::create_dir_all(&evidence).unwrap();
        let adapter = AdapterFile {
            provider: "scripted".into(),
            executable: python(),
            args: vec![
                format!(
                    "{}/support/acp-agent.py",
                    studio_test_support::ENGINE_TESTS_DIR
                ),
                evidence.to_string_lossy().into_owned(),
            ],
            auth_env_names: vec![],
            auth_method: None,
            mcp: McpChoice::Unsupported,
            wire: studio_harness::AgentWire::Acp,
        };
        let paths = AppPaths::new(studio.app_data.clone()).unwrap();
        host::save_settings(&paths, &adapter).unwrap();
        studio.open_project()
    }

    /// The fixture open with the fake native CLI of `wire` (`fake-claude.py` or
    /// `fake-codex.py` of the harness tests) as its saved adapter. The fake reads its
    /// writes from `writes.json` in [`Self::agent_evidence`] and picks each turn's
    /// behaviour from uppercase keywords in the prompt.
    pub fn with_native_agent(cx: &mut TestAppContext, wire: studio_harness::AgentWire) -> Self {
        let studio = Self::new(cx);
        let evidence = studio.agent_evidence();
        fs::create_dir_all(&evidence).unwrap();
        let script = match wire {
            studio_harness::AgentWire::ClaudeStreamJson => "fake-claude.py",
            studio_harness::AgentWire::CodexAppServer => "fake-codex.py",
            studio_harness::AgentWire::Acp => unreachable!("use `with_agent`"),
        };
        let adapter = AdapterFile {
            provider: "scripted".into(),
            executable: python(),
            args: vec![
                format!(
                    "{}/../../studio-harness/tests/{script}",
                    studio_test_support::ENGINE_TESTS_DIR
                ),
                "turns".into(),
                evidence.to_string_lossy().into_owned(),
            ],
            auth_env_names: vec![],
            auth_method: None,
            mcp: McpChoice::Unsupported,
            wire,
        };
        let paths = AppPaths::new(studio.app_data.clone()).unwrap();
        host::save_settings(&paths, &adapter).unwrap();
        studio.open_project()
    }

    pub fn agent_evidence(&self) -> PathBuf {
        self.path("agent-evidence")
    }

    /// Another project next to the fixture, pinned to the same SDK and worker config.
    pub fn another_project(&self, name: &str) -> PathBuf {
        let compatibility = CompatibilityManifest::from_json_str(
            &fs::read_to_string(self.sdk.join("compatibility.json")).unwrap(),
        )
        .unwrap();
        let project = self.path(name);
        studio_project::create(
            &project,
            name,
            sdk_pin(&compatibility),
            &compatibility.fframes_version,
            &compatibility.runtime_version,
        )
        .unwrap();
        fs::copy(self.project.join(FAKE_CONFIG), project.join(FAKE_CONFIG)).unwrap();
        project
    }

    /// Chooses `path` in the Open dialog. The shell ignores Open while a command or another
    /// dialog is in flight, so this waits until the dialog is actually up.
    pub fn open_path(&mut self, path: &Path) {
        let deadline = Instant::now() + WAIT;
        loop {
            self.wait("the shell idle", |t| t["busy"] == false);
            self.dispatch(studio_ui::actions::OpenProject);
            if self.cx.did_prompt_for_paths() {
                break;
            }
            assert!(Instant::now() < deadline, "the Open dialog never opened");
        }
        self.choose_path(path);
    }

    pub fn open_project(self) -> Self {
        let mut studio = self;
        let project = studio.project.clone();
        studio.open_path(&project);
        studio.wait("project open", |t| {
            t["project_open"] == true && t["busy"] == false
        });
        studio
    }

    /// The fixture open with its preview built and displayed.
    pub fn displayed(cx: &mut TestAppContext) -> Self {
        Self::new(cx).display()
    }

    /// [`Self::displayed`] with the fake worker configured by `config`.
    pub fn displayed_with(cx: &mut TestAppContext, config: &str) -> Self {
        Self::with_config(cx, config).display()
    }

    /// Starts a preview build. The shell ignores a command while another one is in flight,
    /// so this dispatches only when the shell is idle. The command can finish before
    /// telemetry is read: its build/preparation state, compiler start, or installed
    /// transport epoch also acknowledge it. A second dispatch would cancel that build.
    pub fn build_preview(&mut self) {
        let _watchdog = Watchdog::arm("a preview build to start");
        let deadline = Instant::now() + WAIT;
        loop {
            self.cx.executor().advance_clock(CLOCK_STEP);
            self.cx.run_until_parked();
            let before = self.telemetry();
            if before["busy"] == false {
                let started = self
                    .compiler
                    .started
                    .load(std::sync::atomic::Ordering::SeqCst);
                self.dispatch(studio_ui::actions::BuildPreview);
                let after = self.telemetry();
                assert!(
                    after["busy"] == true
                        || matches!(
                            after["preview"]["status"].as_str(),
                            Some("building" | "preparing" | "ready")
                        )
                        || self
                            .compiler
                            .started
                            .load(std::sync::atomic::Ordering::SeqCst)
                            > started
                        || after["preview"]["transport_epoch"]
                            != before["preview"]["transport_epoch"],
                    "the build was not taken: {after:#}"
                );
                return;
            }
            assert!(Instant::now() < deadline, "the shell never became idle");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn display(self) -> Self {
        let mut studio = self.open_project();
        studio.build_preview();
        studio.wait("preview displayed", |t| {
            t["preview"]["status"] == "displayed" && t["total_frames"] == 90
        });
        studio
    }

    pub fn dispatch(&mut self, action: impl Action) {
        let _watchdog = Watchdog::arm(action.name());
        self.cx.dispatch_action(self.window.into(), action);
    }

    pub fn telemetry(&mut self) -> Value {
        self.window
            .read_with(&self.cx, |shell, cx| shell.telemetry(cx))
            .unwrap()
    }

    /// Lets the shell's frame loop and the real worker threads run until `done` holds,
    /// and returns the telemetry that satisfied it.
    pub fn wait(&mut self, what: &str, done: impl Fn(&Value) -> bool) -> Value {
        let _watchdog = Watchdog::arm(what);
        let deadline = Instant::now() + WAIT;
        loop {
            self.cx.executor().advance_clock(CLOCK_STEP);
            self.cx.run_until_parked();
            let telemetry = self.telemetry();
            if done(&telemetry) {
                return telemetry;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {telemetry:#}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Runs the frame loop for `duration` of app time without a condition.
    pub fn settle(&mut self, duration: Duration) -> Value {
        let _watchdog = Watchdog::arm("the frame loop to settle");
        let steps = duration.as_millis() / CLOCK_STEP.as_millis();
        for _ in 0..steps {
            self.cx.executor().advance_clock(CLOCK_STEP);
            self.cx.run_until_parked();
            std::thread::sleep(Duration::from_millis(2));
        }
        self.telemetry()
    }

    /// Moves the keyboard focus to the agent prompt and waits until it holds it.
    pub fn focus_prompt(&mut self) {
        self.dispatch(studio_ui::actions::FocusPrompt);
        let _watchdog = Watchdog::arm("the prompt focused");
        let deadline = Instant::now() + WAIT;
        loop {
            self.cx.executor().advance_clock(CLOCK_STEP);
            self.cx.run_until_parked();
            let focused = self
                .window
                .update(&mut self.cx, |shell, window, cx| {
                    shell.prompt_focused(window, cx)
                })
                .expect("the window is open");
            if focused {
                return;
            }
            assert!(Instant::now() < deadline, "the prompt never took the focus");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Types `text` into the focused field.
    pub fn type_text(&mut self, text: &str) {
        self.cx.simulate_input(self.target, text);
    }

    /// The names of the actions `keystroke` is bound to for the focused element, from the
    /// keymap and the key contexts of the focus path.
    pub fn actions_bound_to(&mut self, keystroke: &str) -> Vec<String> {
        let keystroke = gpui::Keystroke::parse(keystroke).expect("a keystroke");
        self.window
            .update(&mut self.cx, |_, window, cx| {
                let stack = window.context_stack();
                let keymap = cx.key_bindings();
                let (bindings, _) = keymap.borrow().bindings_for_input(&[keystroke], &stack);
                bindings
                    .iter()
                    .map(|binding| binding.action().name().to_string())
                    .collect()
            })
            .expect("the window is open")
    }

    /// Presses real keys (`"enter"`, `"space"`, `"shift-tab"`) on the focused element,
    /// through the keymap like a keyboard does.
    pub fn press(&mut self, keystrokes: &str) {
        self.cx.simulate_keystrokes(self.target, keystrokes);
    }

    /// Turns the mouse wheel by `delta_y` pixels at window point (`x`, `y`), with Ctrl (Cmd
    /// on macOS) held when `zoom`.
    pub fn scroll(&mut self, x: f32, y: f32, delta_y: f32, zoom: bool) {
        let modifiers = gpui::Modifiers {
            control: zoom && !cfg!(target_os = "macos"),
            platform: zoom && cfg!(target_os = "macos"),
            ..Default::default()
        };
        gpui::VisualTestContext::from_window(self.target, &self.cx).simulate_event(
            gpui::ScrollWheelEvent {
                position: gpui::point(gpui::px(x), gpui::px(y)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(delta_y))),
                modifiers,
                touch_phase: gpui::TouchPhase::Moved,
            },
        );
    }

    /// The settings window, while it is open.
    pub fn settings_window(&mut self) -> Option<AnyWindowHandle> {
        self.window
            .read_with(&self.cx, |shell, _| shell.settings_window())
            .unwrap()
    }

    /// Sends the next clicks, keys and text to the settings window, which must be open.
    pub fn in_settings_window(&mut self) {
        self.target = self.settings_window().expect("the settings window is open");
    }

    /// The height of the window that clicks, keys and text go to.
    pub fn target_height(&mut self) -> f32 {
        self.target
            .update(&mut self.cx, |_, window, _| {
                f32::from(window.viewport_size().height)
            })
            .expect("the window is open")
    }

    /// Sends the next clicks, keys and text to the main window again.
    pub fn in_main_window(&mut self) {
        self.target = self.window.into();
    }

    /// Makes the shell report the measured buttons' bounds under `buttons` in the telemetry.
    pub fn report_button_bounds(&mut self) {
        self.window
            .update(&mut self.cx, |shell, _, cx| shell.report_button_bounds(cx))
            .unwrap();
    }

    /// Resizes the window's content area, as the user dragging its edge would.
    pub fn resize(&mut self, width: f32, height: f32) {
        gpui::VisualTestContext::from_window(self.window.into(), &self.cx)
            .simulate_resize(gpui::size(gpui::px(width), gpui::px(height)));
    }

    /// Moves the pointer to window point `at` with no button held.
    pub fn hover(&mut self, (x, y): (f32, f32)) {
        let mut window = gpui::VisualTestContext::from_window(self.target, &self.cx);
        window.simulate_mouse_move(
            gpui::point(gpui::px(x), gpui::px(y)),
            None,
            gpui::Modifiers::default(),
        );
    }

    /// Presses the left button at window point `at` and holds it, then lets a frame paint,
    /// as one always does between real input events.
    pub fn mouse_down(&mut self, (x, y): (f32, f32)) {
        let mut window = gpui::VisualTestContext::from_window(self.target, &self.cx);
        let at = gpui::point(gpui::px(x), gpui::px(y));
        window.simulate_mouse_down(at, gpui::MouseButton::Left, gpui::Modifiers::default());
        self.settle(Duration::from_millis(32));
    }

    /// Presses and releases the left button at window point `at` without moving.
    pub fn click(&mut self, (x, y): (f32, f32)) {
        self.mouse_down((x, y));
        let mut window = gpui::VisualTestContext::from_window(self.target, &self.cx);
        let at = gpui::point(gpui::px(x), gpui::px(y));
        window.simulate_mouse_up(at, gpui::MouseButton::Left, gpui::Modifiers::default());
    }

    /// Hovers window point `from`, presses the left button there, moves to `to` and releases.
    pub fn drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.hover(from);
        self.mouse_down(from);
        let mut window = gpui::VisualTestContext::from_window(self.target, &self.cx);
        let point = |(x, y): (f32, f32)| gpui::point(gpui::px(x), gpui::px(y));
        let left = gpui::MouseButton::Left;
        window.simulate_mouse_move(point(to), left, gpui::Modifiers::default());
        window.simulate_mouse_up(point(to), left, gpui::Modifiers::default());
    }

    /// Answers the open dialog the last action opened.
    pub fn choose_path(&mut self, path: &Path) {
        let path = path.to_path_buf();
        self.cx
            .simulate_path_prompt_response(move |_| Some(vec![path]));
    }

    /// Cancels the open dialog the last action opened.
    pub fn cancel_path(&mut self) {
        self.cx.simulate_path_prompt_response(|_| None);
    }

    /// Answers the save dialog the last action opened.
    pub fn choose_new_path(&mut self, path: &Path) {
        let path = path.to_path_buf();
        self.cx.simulate_new_path_selection(move |_| Some(path));
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.temp.path().join(relative)
    }

    pub fn project_file(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.project.join(relative)
    }
}

impl Drop for Studio {
    /// Quitting runs the shell's quit hook, which reaps workers and closes the project.
    fn drop(&mut self) {
        let _watchdog = Watchdog::arm("the app to quit");
        self.cx.update(|cx| cx.quit());
        self.cx.run_until_parked();
    }
}

/// Turns a hang into a failure. Work the test thread runs inline (a background task that
/// blocks on a lock) cannot be interrupted by a deadline check, so a watchdog thread ends
/// the test process, naming what never finished, if the guard is still armed long after
/// the deadline.
struct Watchdog(
    #[expect(dead_code, reason = "dropping the sender disarms the watchdog")]
    std::sync::mpsc::Sender<()>,
);

impl Watchdog {
    fn arm(what: &str) -> Self {
        let (disarm, armed) = std::sync::mpsc::channel::<()>();
        let what = what.to_owned();
        let test = std::thread::current().name().unwrap_or("parity").to_owned();
        std::thread::spawn(move || {
            if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) =
                armed.recv_timeout(WAIT + Duration::from_secs(30))
            {
                eprintln!("{test}: hung waiting for {what}; ending the test process");
                std::process::exit(101);
            }
        });
        Self(disarm)
    }
}

/// Installs a fake SDK as `<sdk_home>/<folder>` that passes the app's SDK verification: it
/// serves the preview contract, asks nothing of the host, and carries a stand-in compiler
/// that reports the manifest's channel and host, and empty FFmpeg folders. Nothing is ever
/// compiled with it; the fake compiler builds every worker.
fn install_sdk(sdk_home: &Path, folder: &str) -> (PathBuf, CompatibilityManifest) {
    let mut compatibility = manifest();
    compatibility.preview_contract_versions =
        vec![fframes_studio_protocol::PREVIEW_CONTRACT_VERSION];
    compatibility.host_prerequisites.clear();
    let sdk = sdk_home.join(folder);
    fs::rename(fake_sdk(sdk_home), &sdk).unwrap();
    fs::write(
        sdk.join("compatibility.json"),
        serde_json::to_vec(&compatibility).unwrap(),
    )
    .unwrap();
    for folder in [
        &compatibility.ffmpeg.headers_rel_path,
        &compatibility.ffmpeg.libs_rel_path,
    ] {
        fs::create_dir_all(sdk.join(folder)).unwrap();
    }
    let bin = sdk.join("toolchain/bin");
    fs::create_dir_all(&bin).unwrap();
    fs::copy(
        fake_rustc(),
        bin.join(format!("rustc{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    fs::write(
        bin.join("rustc-version.txt"),
        format!(
            "rustc {} (fake)\nhost: {}\n",
            compatibility.rust_toolchain.channel, compatibility.target_triple
        ),
    )
    .unwrap();
    (sdk.canonicalize().unwrap(), compatibility)
}

/// The test interpreter by its absolute path: the host never consults the global `PATH`,
/// so the adapter names it as a user's Setup would.
fn python() -> String {
    let python = studio_test_support::python();
    let absolute = if python.is_absolute() {
        python.to_path_buf()
    } else {
        std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
            .map(|dir| dir.join(python))
            .find(|candidate| candidate.is_file())
            .expect("the test interpreter on PATH")
    };
    absolute.to_string_lossy().into_owned()
}

/// `fake-rustc.rs` compiled once per test process. It also serves as any native executable a
/// test needs on disk, such as an adapter for Detect agents to find.
pub fn fake_rustc() -> &'static Path {
    static BINARY: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &BINARY
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let binary = directory
                .path()
                .join(format!("fake-rustc{}", std::env::consts::EXE_SUFFIX));
            let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
            let output = std::process::Command::new(rustc)
                .args(["--edition=2024", "-o"])
                .arg(&binary)
                .arg(Path::new(studio_test_support::ENGINE_TESTS_DIR).join("support/fake-rustc.rs"))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "cannot compile the fake rustc: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            (directory, binary)
        })
        .1
}
