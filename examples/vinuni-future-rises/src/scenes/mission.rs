//! Scene 3, 18–26 s, chapter 01: students walking across campus, then the main steps;
//! the mission lands one line per bar.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, body, kicker, lines_at, wash_left};
use crate::{Grade, Shot, reel, scene_fade};

super::scene_length!(MissionScene, 2);

pub(crate) const SHOTS: [Shot; 2] = [Shot::Clip("stroll.mp4", 0.0), Shot::Photo("steps.jpg", 4.0)];
/// Each line of the mission lands on a bar.
const LINES: [(&str, f32); 3] = [
    ("BIG THINKERS.", 1.0),
    ("PROBLEM SOLVERS.", 2.0),
    ("ADAPTIVE LEARNERS.", 3.0),
];

pub(crate) fn sfx() -> Vec<Sfx> {
    LINES
        .iter()
        .enumerate()
        .map(|(i, &(_, at))| ("pop.wav", at, -13.0, (i as f32 - 1.0) * 0.3))
        .collect()
}

impl Scene for MissionScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("mission")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            {wash_left(0.9)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("OUR MISSION: TO NURTURE", (110.0, 470.0), Align::Start, t, 0.4)}
                {lines_at(&LINES, (104.0, 560.0), 76.0, Align::Start, t)}
                {body("who strive to create a better future for themselves,", (110.0, 830.0), 32.0, Align::Start, t, 3.6)}
                {body("their communities and the world", (110.0, 876.0), 32.0, Align::Start, t, 3.8)}
            </g>
        </g>)
    }
}
