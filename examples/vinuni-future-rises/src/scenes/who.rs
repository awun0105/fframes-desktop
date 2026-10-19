//! Scene 2, 10–18 s, chapter 01: a student walks up to the main building, then the campus
//! from straight above, under who `VinUniversity` is.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, body, headline, kicker, wash_left};
use crate::{Grade, Shot, reel, scene_fade};

super::scene_length!(WhoScene, 1);

pub(crate) const SHOTS: [Shot; 2] = [
    Shot::Clip("approach.mp4", 0.0),
    Shot::Clip("rooftops.mp4", 2.03),
];

pub(crate) fn sfx() -> Vec<Sfx> {
    vec![
        ("pop.wav", 0.6, -13.0, -0.2),
        ("chime.wav", 3.0, -14.0, 0.2),
    ]
}

impl Scene for WhoScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("who")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            {wash_left(0.9)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("FOUNDED BY VINGROUP", (110.0, 560.0), Align::Start, t, 0.6)}
                {headline(&["A PRIVATE,", "NOT-FOR-PROFIT", "UNIVERSITY"], (104.0, 650.0), 76.0, Align::Start, t, 0.9)}
                {body("in Gia Lam, Hanoi, Vietnam", (110.0, 900.0), 32.0, Align::Start, t, 3.0)}
            </g>
        </g>)
    }
}
