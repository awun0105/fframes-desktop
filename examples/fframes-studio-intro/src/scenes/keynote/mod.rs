//! The 16:9 cut: the seven acts of the storyboard in `plan.md` on the beat grid
//! of the soundtrack, from the problem to where to get it.

pub mod comp;
pub mod finale;
pub mod layers;
pub mod look;
pub mod problem;
pub mod solution;
pub mod speed;
pub mod use_cases;
pub mod workflow;

pub use finale::FinaleScene;
pub use layers::LayersScene;
pub use problem::ProblemScene;
pub use solution::SolutionScene;
pub use speed::SpeedScene;
pub use use_cases::UseCasesScene;
pub use workflow::WorkflowScene;

use crate::beat::beat_time;

/// Every sound effect of the 16:9 cut as (file, second, gain in dB), placed on
/// its beat minus the attack of the file.
pub fn sound_effects() -> Vec<(&'static str, f32, f32)> {
    let mut cues = vec![
        // whip pans between the screens of the problem, the glitch of the blind agent
        ("sfx_whoosh.mp3", beat_time(15.5), -14.0),
        ("sfx_whoosh.mp3", beat_time(31.5), -14.0),
        ("sfx_glitch.mp3", beat_time(36.5) - 0.05, -12.0),
        // the drops
        ("sfx_impact.mp3", beat_time(48.0), -10.0),
        ("sfx_impact.mp3", beat_time(96.0), -11.0),
        ("sfx_impact.mp3", beat_time(176.0), -11.0),
        ("sfx_impact.mp3", beat_time(240.0), -10.0),
        ("sfx_braam.mp3", beat_time(240.0), -10.0),
    ];
    for b in [48.0, 80.0, 128.0, 176.0, 208.0, 240.0] {
        cues.push(("sfx_whoosh.mp3", beat_time(b) - 0.18, -15.0));
    }
    // the prompt is typed and sent
    cues.push(("sfx_typing.mp3", beat_time(solution::TYPE_BEAT), -16.0));
    cues.push((
        "sfx_enter.mp3",
        beat_time(solution::SEND_BEAT) - 0.067,
        -9.0,
    ));
    for b in solution::CARD_BEATS {
        cues.push(("sfx_blip.mp3", beat_time(b), -10.0));
    }
    // the New project click, the scene typing itself, the tools landing on the drop
    cues.push(("sfx_typing.mp3", beat_time(workflow::TYPE_BEAT), -17.0));
    cues.push((
        "sfx_enter.mp3",
        beat_time(workflow::CLICK_BEAT) - 0.067,
        -10.0,
    ));
    for b in workflow::BLIP_BEATS {
        cues.push(("sfx_blip.mp3", beat_time(b), -10.0));
    }
    // the use cases land, then take the spotlight in turn
    for b in use_cases::BLIP_BEATS {
        cues.push(("sfx_blip.mp3", beat_time(b), -11.0));
    }
    for b in use_cases::spotlight_beats() {
        cues.push(("sfx_whoosh.mp3", beat_time(b) - 0.18, -19.0));
    }
    // the layers are named, the engines and the measurements land, the end card
    for b in layers::BLIP_BEATS {
        cues.push(("sfx_blip.mp3", beat_time(b), -12.0));
    }
    for b in speed::BLIP_BEATS {
        cues.push(("sfx_blip.mp3", beat_time(b), -12.0));
    }
    for b in finale::BLIP_BEATS {
        cues.push(("sfx_blip.mp3", beat_time(b), -12.0));
    }
    for t in problem::shutter_times() {
        cues.push(("sfx_shutter.mp3", t - 0.02, -12.0));
    }
    cues
}
