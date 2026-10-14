//! Opens the studio window on a fixture project: the real engine, the deterministic Python
//! preview worker compiled by a fake compiler, and app data, SDK home and writer ownership
//! of the test's own. Tests drive it with actions and read its telemetry; this file is the
//! only one that knows the window's root view.
use fframes_studio::{
    build_service::{BuildLimits, BuildService},
    conversation_panel::qualification::OwnershipPolicy,
    studio_shell::{StudioEnvironment, StudioShell},
};
use gpui::{Action, AppContext, TestAppContext, WindowHandle};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use studio_bootstrap::{ProcessTreeManager, WriterOwnership};

use crate::build_fixture::{FAKE_CONFIG, FakeCompiler, fake_sdk, manifest};
use studio_engine::build_materialization::sdk_pin;
use studio_sdk::CompatibilityManifest;

/// Ninety frames at 30 fps in two scenes, with a one-second tone from 0 s. An export
/// renders its first frame and holds until it is cancelled.
pub const FIXTURE_CONFIG: &str = r#"{"frames":90,"fps":30,"scenes":[[0,45],[45,90]],"tracks":[[0.0,1.0]],"audio":"tone","export_hang":true}"#;

const WAIT: Duration = Duration::from_secs(60);

/// Whether this platform may change project source (presets, agent Apply). Only Linux
/// x86-64 is qualified until M3 opens the gates; the gated tests assert the real outcome
/// where this holds and "blocked, with the reason shown" elsewhere, so M3 flips them here.
pub fn mutation_open() -> bool {
    studio_engine::preset_state::MUTATION_QUALIFIED
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
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().unwrap();
        let sdk_home = temp.path().join("sdk-home");
        let (sdk, compatibility) = install_sdk(&sdk_home);
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
            fframes_studio::text_input::bind_keys(cx);
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| StudioShell::with_environment(environment, window, cx))
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

    fn open_project(self) -> Self {
        let mut studio = self;
        let project = studio.project.clone();
        studio.dispatch(fframes_studio::actions::OpenProject);
        studio
            .cx
            .simulate_path_prompt_response(move |_| Some(vec![project]));
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
        studio.dispatch(fframes_studio::actions::BuildPreview);
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
struct Watchdog(std::sync::mpsc::Sender<()>);

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

/// Installs a fake SDK as `<sdk_home>/active` that passes the app's SDK verification: it
/// serves the preview contract, asks nothing of the host, and carries a stand-in compiler
/// that reports the manifest's channel and host, and empty FFmpeg folders. Nothing is ever
/// compiled with it; the fake compiler builds every worker.
fn install_sdk(sdk_home: &Path) -> (PathBuf, CompatibilityManifest) {
    let mut compatibility = manifest();
    compatibility.preview_contract_versions =
        vec![fframes_studio_protocol::PREVIEW_CONTRACT_VERSION];
    compatibility.host_prerequisites.clear();
    let sdk = sdk_home.join("active");
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

/// `fake-rustc.rs` compiled once per test process.
fn fake_rustc() -> &'static Path {
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
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/support/fake-rustc.rs"
                ))
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
