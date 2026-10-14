//! Opens the studio window on a fixture project: the real engine, the deterministic Python
//! preview worker compiled by a fake compiler, and app data, SDK home and writer ownership
//! of the test's own. Tests drive it with actions and read its telemetry; this file is the
//! only one that knows the window's root view.
use gpui::{Action, AppContext, TestAppContext, WindowHandle};
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

/// A change to project source that is behind a platform gate.
pub enum Mutation {
    /// Applying, resetting or overriding a style preset.
    Preset,
    /// Publishing an agent's candidate.
    AgentApply,
}

pub struct Studio {
    pub cx: TestAppContext,
    window: WindowHandle<StudioShell>,
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
        Self::build(cx, config, "active")
    }

    /// A window whose managed SDK home is empty: the fixture's SDK is installed in
    /// `<sdk_home>/elsewhere` ([`Self::sdk`]), for the user to select.
    pub fn without_managed_sdk(cx: &mut TestAppContext) -> Self {
        Self::build(cx, FIXTURE_CONFIG, "elsewhere")
    }

    fn build(cx: &mut TestAppContext, config: &str, sdk_folder: &str) -> Self {
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
            "0.1.0",
        )
        .unwrap();
        fs::write(project.join(FAKE_CONFIG), config).unwrap();
        let app_data = temp.path().join("data");
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
            studio_ui::kit::bind_text_input_keys(cx);
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
        };
        let paths = AppPaths::new(studio.app_data.clone()).unwrap();
        host::save_settings(&paths, &adapter).unwrap();
        studio.open_project()
    }

    pub fn agent_evidence(&self) -> PathBuf {
        self.path("agent-evidence")
    }

    /// Whether this platform may make `mutation`: presets are qualified only on Linux
    /// x86-64, agent Apply only where no-clobber publication is proven (Linux). The gated
    /// tests assert the real outcome where this holds and "blocked, with the reason shown"
    /// elsewhere, so qualifying another platform changes only this answer.
    pub fn mutation_open(&self, mutation: Mutation) -> bool {
        match mutation {
            Mutation::Preset => studio_engine::preset_state::MUTATION_QUALIFIED,
            Mutation::AgentApply => matches!(
                studio_engine::edit_transaction::probe_apply_gate(&self.project),
                studio_engine::ApplyGate::Ready(_)
            ),
        }
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
            "0.1.0",
        )
        .unwrap();
        fs::copy(self.project.join(FAKE_CONFIG), project.join(FAKE_CONFIG)).unwrap();
        project
    }

    pub fn open_project(self) -> Self {
        let mut studio = self;
        let project = studio.project.clone();
        studio.dispatch(studio_ui::actions::OpenProject);
        studio.choose_path(&project);
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

    fn display(self) -> Self {
        let mut studio = self.open_project();
        studio.dispatch(studio_ui::actions::BuildPreview);
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
            self.cx.executor().advance_clock(Duration::from_millis(16));
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
        let steps = duration.as_millis() / 16;
        for _ in 0..steps {
            self.cx.executor().advance_clock(Duration::from_millis(16));
            self.cx.run_until_parked();
            std::thread::sleep(Duration::from_millis(2));
        }
        self.telemetry()
    }

    /// Types `text` into the focused field.
    pub fn type_text(&mut self, text: &str) {
        self.cx.simulate_input(self.window.into(), text);
    }

    /// Presses real keys (`"enter"`, `"space"`, `"shift-tab"`) on the focused element,
    /// through the keymap like a keyboard does.
    pub fn press(&mut self, keystrokes: &str) {
        self.cx.simulate_keystrokes(self.window.into(), keystrokes);
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
