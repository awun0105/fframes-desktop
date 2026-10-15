// A GUI-subsystem executable, so Start menu and Explorer launches open no console window.
#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::{Parser, Subcommand};
use fframes_studio::StudioSpikeApp;
use gpui::{App, AppContext, Bounds, WindowBounds, WindowOptions, point, px, size};
use std::path::PathBuf;

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
        #[command(flatten)]
        observation: Observation,
        /// The packaged launchers forward their own arguments after `studio`; the
        /// workspace takes none, so they are accepted and ignored rather than keeping the
        /// window from opening.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        _forwarded: Vec<std::ffi::OsString>,
    },
    /// Open the native GPUI spike window.
    #[command(long_flag = "spike-ui")]
    SpikeUi,
}

/// Hidden observation of the studio for test harnesses: opens a project and writes the
/// shell's redacted state to a telemetry file. It never changes what the studio does,
/// except that the test injection may stand in for writer containment evidence. The
/// names are long so that no argument a launcher forwards matches them.
#[derive(Debug, clap::Args)]
struct Observation {
    /// Project folder to open.
    #[arg(
        long = "observe-project",
        value_name = "DIR",
        value_parser = existing_dir,
        requires = "telemetry",
        hide = true
    )]
    project: Option<PathBuf>,
    /// Telemetry file to write; it must not exist yet.
    #[arg(
        long = "observe-telemetry",
        value_name = "FILE",
        value_parser = fresh_path,
        requires = "project",
        hide = true
    )]
    telemetry: Option<PathBuf>,
    /// Test-only injection for fixtures (never reachable from the product entry or the
    /// settings file): treats the adapter's writers as process-group contained WITHOUT
    /// the containment check of the launch, and the panel says so. Debug builds only: a
    /// release build refuses it and exits.
    #[arg(
        long = "observe-test-writer-containment",
        value_name = "LABEL",
        requires = "telemetry",
        hide = true
    )]
    test_writer_containment: Option<String>,
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
            // Help and version succeed; a usage error exits 1 as it always has.
            let code = if error.use_stderr() { 1 } else { 0 };
            let _ = error.print();
            std::process::exit(code);
        }
    };
    match cli.command {
        None => run_studio(None),
        Some(Command::Studio { observation, .. }) => run_studio(observed(observation)),
        Some(Command::SpikeUi) => run_spike_ui(),
    }
}

/// What the studio observes; clap makes the project and the telemetry file come together.
struct Observed {
    project: PathBuf,
    output: PathBuf,
    test_ownership: Option<studio_bootstrap::WriterOwnership>,
}

fn observed(observation: Observation) -> Option<Observed> {
    let Observation {
        project: Some(project),
        telemetry: Some(output),
        test_writer_containment,
    } = observation
    else {
        return None;
    };
    // A release build never injects ownership: the capture gate must rest on the
    // containment the launch verified.
    let test_ownership = match test_writer_containment {
        None => None,
        Some(label) if cfg!(debug_assertions) => {
            Some(studio_bootstrap::WriterOwnership::ProcessGroupContained {
                qualification: format!("test-injection:{label}"),
            })
        }
        Some(_) => {
            eprintln!(
                "--observe-test-writer-containment is a test injection and only exists in debug builds"
            );
            std::process::exit(2);
        }
    };
    Some(Observed {
        project,
        output,
        test_ownership,
    })
}

fn run_studio(observation: Option<Observed>) {
    gpui_platform::application()
        .with_assets(fframes_studio::assets::Assets)
        .run(move |cx: &mut App| {
            studio_ui::shell::bind_keys(cx);
            let settings = studio_engine::app_paths::AppPaths::system()
                .map(|paths| studio_engine::app_settings::AppSettings::load(&paths))
                .unwrap_or_default();
            studio_theme::init(cx, settings.theme);
            let bounds = Bounds::centered(None, size(px(1275.), px(800.)), cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        // The header draws the title bar, as in Cap's editor: the caption
                        // buttons on Windows, room for the inset traffic lights on macOS.
                        // Only the Windows and macOS backends read these; Linux keeps the
                        // system's decorations.
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some("fframes Studio".into()),
                            appears_transparent: true,
                            traffic_light_position: Some(point(px(20.), px(20.))),
                        }),
                        app_owns_titlebar_drag: true,
                        ..Default::default()
                    },
                    move |window, cx| {
                        cx.new(|cx| {
                            let mut shell = studio_ui::shell::StudioShell::new(
                                env!("CARGO_PKG_VERSION"),
                                window,
                                cx,
                            );
                            studio_theme::bind_window(window, cx);
                            if let Some(Observed {
                                project,
                                output,
                                test_ownership,
                            }) = observation
                            {
                                shell.start_agent_telemetry(project, output, test_ownership, cx)
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

fn run_spike_ui() {
    gpui_platform::application().run(move |cx: &mut App| {
        studio_ui::kit::bind_text_input_keys(cx);
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
                    let mut app = StudioSpikeApp::new(window, cx);
                    app.load_initial_frame(window, cx);
                    app
                })
            },
        )
        .expect("failed to open window");
        cx.activate(true);
    });
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

    fn observation(args: &[&str]) -> Result<Observation, clap::Error> {
        match parse(args)?.command {
            Some(Command::Studio { observation, .. }) => Ok(observation),
            other => panic!("expected studio, got {other:?}"),
        }
    }

    #[test]
    fn the_studio_observation_needs_an_existing_project_and_a_fresh_telemetry_file() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().to_str().unwrap();
        let fresh = dir.path().join("telemetry.json");
        let fresh = fresh.to_str().unwrap();
        let observed = observation(&[
            "studio",
            "--observe-project",
            project,
            "--observe-telemetry",
            fresh,
            "--observe-test-writer-containment",
            "fixture",
        ])
        .unwrap();
        assert_eq!(observed.project.as_deref(), Some(dir.path()));
        assert_eq!(observed.telemetry.as_deref(), Some(fresh.as_ref()));
        assert_eq!(observed.test_writer_containment.as_deref(), Some("fixture"));
        // The project folder itself exists, so it is not a fresh telemetry file.
        let both = |project: &str, telemetry: &str| {
            observation(&[
                "studio",
                "--observe-project",
                project,
                "--observe-telemetry",
                telemetry,
            ])
        };
        assert!(both(project, project).is_err());
        assert!(both(fresh, fresh).is_err());
        // Each needs the other; the test injection needs an observation.
        assert!(observation(&["studio", "--observe-project", project]).is_err());
        assert!(observation(&["studio", "--observe-telemetry", fresh]).is_err());
        assert!(observation(&["studio", "--observe-test-writer-containment", "x"]).is_err());
    }

    #[test]
    fn the_observation_is_hidden_and_launcher_arguments_still_open_the_studio() {
        let forwarded =
            observation(&["studio", "--from-launcher", "--project", "file.txt"]).unwrap();
        assert!(forwarded.project.is_none() && forwarded.telemetry.is_none());
        let help = Cli::command()
            .find_subcommand_mut("studio")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(!help.contains("observe"), "{help}");
        for removed in [
            "qualify-m2",
            "qualify-m3",
            "qualify-presentation",
            "qualify-writer",
        ] {
            assert!(parse(&[removed]).is_err(), "{removed}");
        }
    }
}
