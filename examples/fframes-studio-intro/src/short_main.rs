use fframes_studio_intro::{render_cli, short::StudioShortVideo};
use std::process::ExitCode;

fn main() -> ExitCode {
    render_cli::run(&StudioShortVideo, "fframes-studio-intro-short-9x16.mp4")
}
