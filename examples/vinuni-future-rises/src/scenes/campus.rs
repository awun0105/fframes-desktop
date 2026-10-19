//! Scene 13, 102–110 s, chapter 05: friends down a corridor, lunch together, a walk under
//! the trees; every first-year lives on campus, moving in for the Week of Welcome.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, Card, body, kicker, lines_at, wash_left};
use crate::{AMBER, Grade, Shot, reel, scene_fade};

super::scene_length!(CampusScene, 12);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("corridor.mp4", 0.0),
    Shot::Clip("canteen.mp4", 2.82),
    Shot::Clip("walkers.mp4", 5.46),
];
const LINES: [(&str, f32); 3] = [
    ("CO-LIVING.", 1.0),
    ("CO-WORKING.", 3.0),
    ("CO-LEARNING.", 5.0),
];
/// Move-in Day, the two days before the Week of Welcome, the first-years' orientation
/// week (photo from the student activities page).
const MOVE_IN: Card = Card {
    file: "life-move-in.jpg",
    id: "campus-move-in",
    caption: "WEEK OF WELCOME",
    center: (1440.0, 400.0),
    size: (520.0, 340.0),
    tilt: 3.0,
    at: 1.5,
    until: 5.3,
    accent: AMBER,
    logo: true,
    mark: None,
};

pub(crate) fn sfx() -> Vec<Sfx> {
    LINES
        .iter()
        .enumerate()
        .map(|(i, &(_, at))| ("pop.wav", at, -13.0, (i as f32 - 1.0) * 0.3))
        .chain([("swoosh.wav", MOVE_IN.at, -18.0, 0.45)])
        .collect()
}

impl Scene for CampusScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("campus")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        fframes::svgr!(<g>
            {shots}
            {wash_left(0.9)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("EVERY FIRST-YEAR LIVES ON CAMPUS", (110.0, 470.0), Align::Start, t, 0.3)}
                {lines_at(&LINES, (104.0, 570.0), 84.0, Align::Start, t)}
                {body("in the residences, by the Rose Garden and Central Square", (110.0, 880.0), 32.0, Align::Start, t, 6.0)}
                {MOVE_IN.draw(ctx, t)}
            </g>
        </g>)
    }
}
