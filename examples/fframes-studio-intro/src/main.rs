use fframes_studio_intro::{StudioIntroVideo, render_cli};
use std::process::ExitCode;

fn main() -> ExitCode {
    render_cli::run(&StudioIntroVideo, "fframes-studio-intro.mp4")
}
