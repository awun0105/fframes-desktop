//! Scene 11, 84–92 s, chapter 04: international students together, then exchange
//! students in front of the main building; where the community comes from.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, kicker, stat, wash_bottom};
use crate::{Grade, Shot, reel, scene_fade};

super::scene_length!(CommunityScene, 10);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("friends.mp4", 0.0),
    Shot::Clip("chat.mp4", 3.15),
    Shot::Photo("exchange.jpg", 4.85),
];
const COUNTRIES_AT: f32 = 0.8;
const EXCHANGE_AT: f32 = 2.8;

pub(crate) fn sfx() -> Vec<Sfx> {
    vec![
        ("ticks.wav", COUNTRIES_AT, -15.0, -0.3),
        ("ticks.wav", EXCHANGE_AT, -15.0, 0.3),
    ]
}

impl Scene for CommunityScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("community")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            {wash_bottom(1.0)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("STUDENTS AND FACULTY FROM AROUND THE WORLD", (960.0, 690.0), Align::Middle, t, 0.3)}
                {stat(("40", "COUNTRIES AND TERRITORIES"), "community-countries", (620.0, 860.0), 120.0, "#ffffff", t, COUNTRIES_AT)}
                {stat(("1,000+", "EXCHANGE STUDENTS A YEAR"), "community-exchange", (1300.0, 860.0), 120.0, "#ffffff", t, EXCHANGE_AT)}
            </g>
        </g>)
    }
}
