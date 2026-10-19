//! Scene 18, 138–146 s, chapter 06: graduates celebrate, a hand reaches up, a graduate in
//! front of the main building, the main building's spire; the invitation, building into the
//! finale.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, body, headline, kicker, wash_bottom};
use crate::{Grade, Shot, reel, scene_fade};

super::scene_length!(JoinScene, 17);

pub(crate) const SHOTS: [Shot; 4] = [
    Shot::Clip("burst.mp4", 0.0),
    Shot::Clip("hand.mp4", 2.0),
    Shot::Clip("graduate.mp4", 4.0),
    Shot::Clip("spire.mp4", 7.0),
];
const TAGLINE_AT: f32 = 2.0;
const INVITE_AT: f32 = 4.0;

pub(crate) fn sfx() -> Vec<Sfx> {
    vec![
        ("pop.wav", 0.4, -13.0, 0.0),
        ("chime.wav", TAGLINE_AT, -11.0, 0.0),
        // builds into the finale's cut
        ("riser.wav", JoinScene::LENGTH - 1.5, -9.0, 0.0),
    ]
}

impl Scene for JoinScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("join")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            {wash_bottom(1.0)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("BIG THINKERS WELCOME", (960.0, 660.0), Align::Middle, t, 0.4)}
                {headline(&["FROM VIETNAM.", "FOR THE WORLD."], (960.0, 770.0), 96.0, Align::Middle, t, TAGLINE_AT)}
                {body("Your future starts at VinUniversity", (960.0, 1000.0), 36.0, Align::Middle, t, INVITE_AT)}
            </g>
        </g>)
    }
}
