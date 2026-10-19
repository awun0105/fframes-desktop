//! Scene 17, 132–138 s, chapter 06: the hall at graduation, sashes and flags; the
//! universities that invited a quarter of the first cohort to master's and doctoral programs
//! ("`VinUniversity` graduates the first academic cohort" on vinuni.edu.vn), as their logos.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, headline, kicker, logo_wall};
use super::outcomes::{STEP, logo_blips};
use crate::{Grade, NAVY, Shot, reel, scene_fade};

super::scene_length!(StudyScene, 16);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("hallgrads.mp4", 0.0),
    Shot::Clip("sashes.mp4", 3.2),
    Shot::Clip("flags.mp4", 4.7),
];
const SCHOOLS: [&str; 7] = [
    "logo-harvard.png",
    "logo-cornell.png",
    "logo-penn.png",
    "logo-duke.png",
    "logo-nus.png",
    "logo-ntu.png",
    "logo-epfl.png",
];
const TITLE_AT: f32 = 0.3;
const LOGOS_AT: f32 = 0.8;

pub(crate) fn sfx() -> Vec<Sfx> {
    let mut cues = vec![("pop.wav", TITLE_AT, -13.0, 0.0)];
    cues.extend(logo_blips(LOGOS_AT, SCHOOLS.len()));
    cues
}

impl Scene for StudyScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("study")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            <rect width="1920" height="1080" fill={NAVY} opacity="0.55" />
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("25% OF THE FIRST COHORT", (960.0, 330.0), Align::Middle, t, 0.1)}
                {headline(&["INVITED TO GRADUATE STUDY AT"], (960.0, 470.0), 60.0, Align::Middle, t, TITLE_AT)}
                {logo_wall(ctx, &SCHOOLS, 4, 530.0, (t, LOGOS_AT, STEP, Self::LENGTH + 1.0))}
            </g>
        </g>)
    }
}
