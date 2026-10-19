//! Scene 10, 74–84 s, chapter 03: a team around a laptop, a class around its tables and
//! a poster showcase, students pitching to their lecturer; how you learn, one word per shot.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, body, kicker, lines_at, wash_left};
use crate::{Grade, Shot, reel, scene_fade};

super::scene_length!(LearnScene, 9);

pub(crate) const SHOTS: [Shot; 4] = [
    Shot::Clip("laptops.mp4", 0.0),
    Shot::Clip("tables.mp4", 2.8),
    Shot::Clip("posters.mp4", 5.4),
    Shot::Clip("pitch.mp4", 7.7),
];
const WORDS: [(&str, f32); 3] = [
    ("ACTIVE.", 0.6),
    ("EXPERIENTIAL.", 2.8),
    ("TEAM-BASED.", 5.4),
];

pub(crate) fn sfx() -> Vec<Sfx> {
    WORDS
        .iter()
        .enumerate()
        .map(|(i, &(_, at))| ("pop.wav", at, -12.0, (i as f32 - 1.0) * 0.3))
        .collect()
}

impl Scene for LearnScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("learn")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            {wash_left(0.9)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("YOU LEARN BY DOING", (110.0, 470.0), Align::Start, t, 0.3)}
                {lines_at(&WORDS, (104.0, 570.0), 84.0, Align::Start, t)}
                {body("with interdisciplinary research across the colleges", (110.0, 880.0), 32.0, Align::Start, t, 7.7)}
            </g>
        </g>)
    }
}
