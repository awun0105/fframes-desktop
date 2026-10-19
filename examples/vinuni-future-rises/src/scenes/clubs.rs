//! Scene 14, 110–116 s, chapter 05: tennis, a team with its trophy, a night on stage; for
//! each shot one of the events on the student activities page, named under the logo with
//! its photo from that page on a card.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, Card, headline, kicker, logo_plate, wash_bottom};
use crate::{AMBER, Grade, Shot, progress, reel, scene_fade};

super::scene_length!(ClubsScene, 13);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("tennis.mp4", 0.0),
    Shot::Clip("champions.mp4", 2.5),
    Shot::Clip("stage.mp4", 3.95),
];
/// Event, its photo and caption, in from its cut and out at the next one.
const EVENTS: [(&str, &str, &str, f32, f32); 3] = [
    (
        "GREEN SPORTS DAY",
        "life-sports-day.jpg",
        "TUG OF WAR",
        0.2,
        2.5,
    ),
    (
        "CLUB FAIR",
        "life-club-fair.jpg",
        "30+ STUDENT CLUBS",
        2.6,
        3.95,
    ),
    (
        "SPRING FESTIVAL",
        "life-spring-festival.jpg",
        "MUSICAL SHOW",
        4.05,
        6.0,
    ),
];
const IDS: [&str; 3] = ["clubs-card-a", "clubs-card-b", "clubs-card-c"];

pub(crate) fn sfx() -> Vec<Sfx> {
    EVENTS
        .iter()
        .enumerate()
        .flat_map(|(i, &(.., at, _))| {
            [
                ("blip.wav", at, -13.0, (i as f32 - 1.0) * 0.35),
                ("swoosh.wav", at + 0.15, -18.0, 0.45),
            ]
        })
        .collect()
}

impl Scene for ClubsScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("clubs")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        let events: Vec<Svgr<'a>> = EVENTS
            .iter()
            .enumerate()
            .map(|(i, &(name, file, caption, at, until))| {
                let out = 1.0 - progress(t, until - 0.15, 0.15);
                let card = Card {
                    file,
                    id: IDS[i],
                    caption,
                    center: (1460.0, 360.0),
                    size: (500.0, 320.0),
                    tilt: if i % 2 == 0 { 3.0 } else { -3.0 },
                    at: at + 0.15,
                    until: until + 0.1,
                    accent: AMBER,
                    logo: true,
                    mark: None,
                };
                fframes::svgr!(<g>
                    <g opacity={out}>{headline(&[name], (960.0, 920.0), 96.0, Align::Middle, t, at)}</g>
                    {card.draw(ctx, t)}
                </g>)
            })
            .collect();
        fframes::svgr!(<g>
            {shots}
            {wash_bottom(1.0)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                <g opacity={progress(t, 0.1, 0.4)}>{logo_plate(ctx, (960.0, 700.0), 36.0)}</g>
                {kicker("STUDENT ACTIVITIES AND EVENTS", (960.0, 800.0), Align::Middle, t, 0.2)}
                {events}
            </g>
        </g>)
    }
}
