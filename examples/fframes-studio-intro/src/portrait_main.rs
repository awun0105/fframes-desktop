use fframes_studio_intro::{StudioIntroPortraitVideo, render_cli};
use std::process::ExitCode;

fn main() -> ExitCode {
    render_cli::run(&StudioIntroPortraitVideo, "fframes-studio-intro-9x16.mp4")
}
