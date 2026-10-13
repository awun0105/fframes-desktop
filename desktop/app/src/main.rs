// A GUI-subsystem executable, so Start menu and Explorer launches open no console window.
#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::{Parser, Subcommand};
use fframes_studio::StudioSpikeApp;
use gpui::{App, AppContext, Bounds, WindowBounds, WindowOptions, px, size};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use studio_sdk::{CompatibilityManifest, SdkInstaller};

/// Longest `qualify-writer --timeout` accepted (one day).
const MAX_WRITER_TIMEOUT_SECS: u64 = 24 * 60 * 60;

/// fframes Studio, the native project workspace. Without a command it opens the studio.
#[derive(Debug, Parser)]
#[command(
    name = "fframes-studio",
    version,
    after_help = "Use studio_setup for GPUI-free doctor and project scaffolding."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Open the native project workspace (the default).
    Studio {
        /// The packaged launchers forward their own arguments after `studio`; the
        /// workspace takes none, so they are accepted and ignored rather than keeping the
        /// window from opening.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        _forwarded: Vec<std::ffi::OsString>,
    },
    /// Open the native GPUI spike window.
    #[command(long_flag = "spike-ui")]
    SpikeUi,
    /// Build a project's preview and record playback telemetry (M2 harness).
    QualifyM2 {
        #[command(flatten)]
        observation: Observation,
    },
    /// Open a project and record the agent panel's redacted state (M3 harness).
    QualifyM3 {
        #[command(flatten)]
        observation: Observation,
        /// Test-only injection for fixtures (never reachable from the product entry or the
        /// settings file): treats the adapter's writers as process-group contained WITHOUT
        /// qualification evidence, and the panel says so.
        #[arg(long, value_name = "LABEL", hide = true)]
        test_writer_containment: Option<String>,
    },
    /// Measure whether the configured adapter's writer processes stay inside their task
    /// scope, and record the result in the installed qualification ledger.
    QualifyWriter {
        /// Provider profile to measure; the selected profile when omitted.
        #[arg(long, value_name = "ID")]
        profile: Option<String>,
        /// Time limit for the whole run (all three scenarios), in seconds.
        #[arg(
            long,
            value_name = "SECS",
            default_value_t = 1800,
            value_parser = clap::value_parser!(u64).range(1..=MAX_WRITER_TIMEOUT_SECS)
        )]
        timeout: u64,
        /// Development run against a fixture: the evidence goes to --out, never the
        /// installed ledger.
        #[arg(long, requires = "out")]
        development: bool,
        /// Folder for the evidence of a --development run (not inside the installed
        /// qualification folder).
        #[arg(long, value_name = "DIR", requires = "development")]
        out: Option<PathBuf>,
    },
    /// Install an SDK bundle and run the native presentation qualification.
    QualifyPresentation {
        #[arg(long, value_name = "DIR")]
        bundle: PathBuf,
        #[arg(long, value_name = "DIR")]
        sdk_home: PathBuf,
        #[arg(long, value_name = "DIR")]
        project: PathBuf,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
}

/// The project and telemetry file of an observation entry.
#[derive(Debug, clap::Args)]
struct Observation {
    /// Project folder to open.
    #[arg(long, value_name = "DIR", value_parser = existing_dir)]
    project: PathBuf,
    /// Telemetry file to write; it must not exist yet.
    #[arg(long, value_name = "FILE", value_parser = fresh_path)]
    telemetry: PathBuf,
}

fn existing_dir(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.is_dir() {
        Ok(path)
    } else {
        Err("not an existing folder".into())
    }
}

fn fresh_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.exists() {
        Err("already exists; pass a fresh file".into())
    } else {
        Ok(path)
    }
}

struct PresentationQualification {
    sdk_home: PathBuf,
    project: PathBuf,
    output: PathBuf,
    manifest: CompatibilityManifest,
}
/// Reattaches the terminal a command was typed in, so subcommand output and errors stay
/// visible. Redirected handles (files, pipes) are already valid and are left alone.
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
    };
    // SAFETY: plain Win32 calls without pointers; failure only means no parent console.
    unsafe {
        if GetStdHandle(STD_ERROR_HANDLE).is_null() {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

fn main() {
    #[cfg(windows)]
    attach_parent_console();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            // Help and version succeed; a usage error exits 1 as it always has (2 is
            // reserved for a failed qualify-writer gate).
            let code = if error.use_stderr() { 1 } else { 0 };
            let _ = error.print();
            std::process::exit(code);
        }
    };
    match cli.command.unwrap_or(Command::Studio {
        _forwarded: Vec::new(),
    }) {
        Command::Studio { .. } => run_studio(None),
        Command::SpikeUi => run_spike_ui(None),
        Command::QualifyM2 {
            observation: Observation { project, telemetry },
        } => run_studio(Some(Qualification::Playback {
            project,
            output: telemetry,
        })),
        Command::QualifyM3 {
            observation: Observation { project, telemetry },
            test_writer_containment,
        } => {
            let test_ownership = test_writer_containment.map(|label| {
                studio_bootstrap::WriterOwnership::ProcessGroupContained {
                    qualification: format!("test-injection:{label}"),
                }
            });
            run_studio(Some(Qualification::AgentPanel {
                project,
                output: telemetry,
                test_ownership,
            }))
        }
        Command::QualifyWriter {
            profile,
            timeout,
            development: _,
            out,
        } => qualify_writer(fframes_studio::qualify_writer::Options {
            profile,
            timeout: Duration::from_secs(timeout),
            // clap ties --out and --development together.
            development_out: out,
        }),
        Command::QualifyPresentation {
            bundle,
            sdk_home,
            project,
            output,
        } => match prepare_presentation(bundle, sdk_home, project, output) {
            Ok(qualification) => run_spike_ui(Some(qualification)),
            Err(error) => {
                eprintln!("Qualification failed: {error}");
                std::process::exit(1);
            }
        },
    }
}

fn qualify_writer(options: fframes_studio::qualify_writer::Options) {
    let result = studio_engine::app_paths::AppPaths::system()
        .map_err(|e| e.to_string())
        .and_then(|paths| fframes_studio::qualify_writer::run(&paths, &options));
    match result {
        Ok((measured, path)) => {
            println!("evidence: {}", path.display());
            if measured.passed() {
                println!(
                    "PASS: {} ({}) is qualified as process-group contained on this computer.",
                    measured.profile, measured.agent_name
                );
            } else {
                println!(
                    "FAIL: {} is not qualified; capture stays blocked.",
                    measured.profile
                );
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("qualify-writer: {error}");
            std::process::exit(1);
        }
    }
}

/// Installs the bundle's SDK into `sdk_home`; relative paths are taken from the current
/// folder.
fn prepare_presentation(
    bundle: PathBuf,
    sdk_home: PathBuf,
    project: PathBuf,
    output: PathBuf,
) -> Result<PresentationQualification, Box<dyn std::error::Error>> {
    let bundle = bundle.canonicalize()?;
    let current = std::env::current_dir()?;
    let sdk_home = current.join(sdk_home);
    let project = current.join(project);
    let output = current.join(output);
    if output.exists() {
        return Err("Evidence output already exists".into());
    }
    let manifest = CompatibilityManifest::from_json_str(&std::fs::read_to_string(
        bundle.join("compatibility.json"),
    )?)?;
    let artifacts: Vec<_> = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.clone(),
                bundle.join(artifact.url.trim_start_matches("file://")),
            )
        })
        .collect();
    SdkInstaller::new(&sdk_home).install_from_local_artifacts_for_app(
        &manifest,
        &artifacts,
        env!("CARGO_PKG_VERSION"),
    )?;
    Ok(PresentationQualification {
        sdk_home,
        project,
        output,
        manifest,
    })
}

/// Observation entries of the production shell (telemetry files for the external
/// harnesses; they never change behavior).
enum Qualification {
    /// M2: build the preview and record playback telemetry.
    Playback { project: PathBuf, output: PathBuf },
    /// M3: open the project and record the agent panel's redacted state.
    AgentPanel {
        project: PathBuf,
        output: PathBuf,
        test_ownership: Option<studio_bootstrap::WriterOwnership>,
    },
}

fn run_studio(qualification: Option<Qualification>) {
    gpui_platform::application()
        .with_assets(fframes_studio::assets::Assets)
        .run(move |cx: &mut App| {
            fframes_studio::text_input::bind_keys(cx);
            fframes_studio::design_system::set_appearance(
                fframes_studio::design_system::appearance_from_window(cx.window_appearance()),
            );
            let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some("fframes Studio".into()),
                            appears_transparent: false,
                            traffic_light_position: None,
                        }),
                        ..Default::default()
                    },
                    move |window, cx| {
                        cx.new(|cx| {
                            let mut shell =
                                fframes_studio::studio_shell::StudioShell::new(window, cx);
                            cx.observe_window_appearance(window, |_shell, window, cx| {
                                fframes_studio::design_system::set_appearance(
                                    fframes_studio::design_system::appearance_from_window(
                                        window.appearance(),
                                    ),
                                );
                                cx.notify();
                            })
                            .detach();
                            match qualification {
                                Some(Qualification::Playback { project, output }) => {
                                    shell.start_qualification(project, output, cx)
                                }
                                Some(Qualification::AgentPanel {
                                    project,
                                    output,
                                    test_ownership,
                                }) => {
                                    shell.start_agent_telemetry(project, output, test_ownership, cx)
                                }
                                None => (),
                            }
                            shell
                        })
                    },
                )
                .expect("failed to open Studio");
            // Closing the last window must not release the shell before its awaited
            // quit hook can stop audio and drop the final materialization consumers.
            let shell = window.entity(cx).expect("Studio root entity");
            cx.on_window_closed(move |cx, _| {
                let _keep_alive = &shell;
                cx.quit();
            })
            .detach();
            cx.activate(true);
        });
}

fn run_spike_ui(qualification: Option<PresentationQualification>) {
    let evidence_output = qualification
        .as_ref()
        .map(|qualification| qualification.output.clone());
    gpui_platform::application().run(move |cx: &mut App| {
        fframes_studio::text_input::bind_keys(cx);
        let bounds = Bounds::centered(None, size(px(1000.), px(1000.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("fframes studio — Native GPUI Spike".into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            move |window, cx| {
                cx.new(|cx| {
                    let mut app = StudioSpikeApp::new(cx);
                    app.load_initial_frame(window, cx);
                    if let Some(qualification) = qualification {
                        app.setup_view.update(cx, |view, _| {
                            view.sdk_home = qualification.sdk_home;
                            view.manifest = qualification.manifest;
                        });
                        app.worker_project_root = Some(qualification.project);
                        app.qualification_output = Some(qualification.output);
                        app.launch_real_worker(cx);
                        cx.spawn(async move |this, cx| {
                            let deadline = Instant::now() + Duration::from_secs(120);
                            loop {
                                cx.background_executor()
                                    .timer(Duration::from_millis(100))
                                    .await;
                                let waiting = this
                                    .update(cx, |app, cx| {
                                        if !app.worker_busy {
                                            if app.worker_client.is_some() {
                                                app.start_stress_test(cx);
                                            } else {
                                                app.abort_stress(
                                                    format!(
                                                        "Worker startup failed: {}",
                                                        app.worker_status
                                                    ),
                                                    cx,
                                                );
                                            }
                                            return false;
                                        }
                                        if Instant::now() >= deadline {
                                            app.abort_stress(
                                                "Worker startup exceeded deadline".into(),
                                                cx,
                                            );
                                            return false;
                                        }
                                        true
                                    })
                                    .unwrap_or(false);
                                if !waiting {
                                    break;
                                }
                            }
                        })
                        .detach();
                    }
                    app
                })
            },
        )
        .expect("failed to open window");
        cx.activate(true);
    });
    if let Some(output) = evidence_output {
        let verified = std::fs::read(&output)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|record| {
                record["completed"] == true
                    && record["confirmed_presentations"] == 1000
                    && record["paint_submissions"] == 1000
                    && record["verified_render_requests"] == 2000
                    && record["failure"].is_null()
            });
        if !verified {
            let details = std::fs::read_to_string(&output).unwrap_or_else(|e| {
                format!(
                    "Could not read evidence output from {}: {e}",
                    output.display()
                )
            });
            eprintln!("Native presentation qualification did not complete: {details}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("fframes-studio").chain(args.iter().copied()))
    }

    #[test]
    fn the_command_line_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn no_command_opens_the_studio_and_the_legacy_spike_flag_still_works() {
        assert!(parse(&[]).unwrap().command.is_none());
        for args in [&["studio"][..], &["studio", "--from-launcher", "file.txt"]] {
            assert!(matches!(
                parse(args).unwrap().command,
                Some(Command::Studio { .. })
            ));
        }
        for args in [["spike-ui"], ["--spike-ui"]] {
            assert!(matches!(
                parse(&args).unwrap().command,
                Some(Command::SpikeUi)
            ));
        }
        assert!(parse(&["unknown"]).is_err());
    }

    #[test]
    fn qualify_writer_out_and_development_require_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_str().unwrap();
        assert!(parse(&["qualify-writer", "--out", out]).is_err());
        assert!(parse(&["qualify-writer", "--development"]).is_err());
        let Some(Command::QualifyWriter {
            development,
            out: parsed,
            timeout,
            ..
        }) = parse(&["qualify-writer", "--development", "--out", out])
            .unwrap()
            .command
        else {
            panic!("qualify-writer");
        };
        assert!(development);
        assert_eq!(parsed.as_deref(), Some(dir.path()));
        assert_eq!(timeout, 1800);
    }

    #[test]
    fn qualify_writer_timeout_is_bounded_and_flags_are_not_values() {
        let max = MAX_WRITER_TIMEOUT_SECS.to_string();
        let over = (MAX_WRITER_TIMEOUT_SECS + 1).to_string();
        assert!(parse(&["qualify-writer", "--timeout", &max]).is_ok());
        for bad in ["0", "-5", &over, "18446744073709551615", "soon"] {
            assert!(
                parse(&["qualify-writer", "--timeout", bad]).is_err(),
                "{bad}"
            );
        }
        assert!(parse(&["qualify-writer", "--profile", "--development"]).is_err());
        assert!(parse(&["qualify-writer", "--unknown"]).is_err());
    }

    #[test]
    fn observation_entries_need_an_existing_project_and_a_fresh_telemetry_file() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().to_str().unwrap();
        let fresh = dir.path().join("telemetry.json");
        let fresh = fresh.to_str().unwrap();
        for command in ["qualify-m2", "qualify-m3"] {
            assert!(parse(&[command, "--project", project, "--telemetry", fresh]).is_ok());
            // The project folder itself exists, so it is not a fresh telemetry file.
            assert!(parse(&[command, "--project", project, "--telemetry", project]).is_err());
            assert!(parse(&[command, "--project", fresh, "--telemetry", fresh]).is_err());
            assert!(parse(&[command, "--project", project]).is_err());
        }
        let Some(Command::QualifyM3 {
            test_writer_containment,
            ..
        }) = parse(&[
            "qualify-m3",
            "--project",
            project,
            "--telemetry",
            fresh,
            "--test-writer-containment",
            "fixture",
        ])
        .unwrap()
        .command
        else {
            panic!("qualify-m3");
        };
        assert_eq!(test_writer_containment.as_deref(), Some("fixture"));
        let help = Cli::command()
            .find_subcommand_mut("qualify-m3")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(!help.contains("test-writer-containment"));
    }
}
