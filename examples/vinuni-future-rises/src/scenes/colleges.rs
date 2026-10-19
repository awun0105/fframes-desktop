//! Scene 5, 34–42 s, chapter 02: a lecture hall and a lecturer at the board; the four
//! colleges land one per beat as cards with their photos from the academics page, then
//! English as the language of every program.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, Card, headline, kicker_in, stat, wash_left};
use super::college::{CAS, CBM, CECS, CHS};
use crate::{Grade, NAVY, SKY, Shot, reel, scene_fade};

super::scene_length!(CollegesScene, 4);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("lecture.mp4", 0.0),
    Shot::Clip("lecturewide.mp4", 2.28),
    Shot::Clip("chalk.mp4", 4.45),
];
/// Photo, card caption, name lines, colour and mark, the beat it lands on.
type Entry = (
    &'static str,
    &'static str,
    [&'static str; 2],
    (&'static str, &'static str, &'static str),
    f32,
);
const COLLEGES: [Entry; 4] = [
    (
        "card-engineering.jpg",
        "COLLEGE 01",
        ["ENGINEERING &", "COMPUTER SCIENCE"],
        CECS,
        1.0,
    ),
    (
        "card-health.jpg",
        "COLLEGE 02",
        ["HEALTH", "SCIENCES"],
        CHS,
        1.5,
    ),
    (
        "card-business.jpg",
        "COLLEGE 03",
        ["BUSINESS &", "MANAGEMENT"],
        CBM,
        2.0,
    ),
    (
        "card-arts.jpg",
        "COLLEGE 04",
        ["ARTS &", "SCIENCES"],
        CAS,
        2.5,
    ),
];
const IDS: [&str; 4] = ["colleges-a", "colleges-b", "colleges-c", "colleges-d"];
const ENGLISH_AT: f32 = 4.45;

pub(crate) fn sfx() -> Vec<Sfx> {
    let mut cues: Vec<Sfx> = COLLEGES
        .iter()
        .enumerate()
        .map(|(i, &(.., at))| ("blip.wav", at, -14.0, (i as f32 - 1.5) * 0.25))
        .collect();
    cues.push(("ticks.wav", ENGLISH_AT, -16.0, 0.3));
    cues
}

impl Scene for CollegesScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("colleges")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        let colleges: Vec<Svgr<'a>> = COLLEGES
            .iter()
            .enumerate()
            .map(|(i, &(file, caption, ref name, (color, mark, _), at))| {
                let x = 330.0 + i as f32 * 420.0;
                let card = Card {
                    file,
                    id: IDS[i],
                    caption,
                    center: (x, 490.0),
                    size: (360.0, 230.0),
                    tilt: if i % 2 == 0 { -2.0 } else { 2.0 },
                    at,
                    until: Self::LENGTH + 1.0,
                    accent: color,
                    logo: true,
                    mark: Some(mark),
                };
                fframes::svgr!(<g>
                    {card.draw(ctx, t)}
                    {headline(name, (x, 712.0), 30.0, Align::Middle, t, at + 0.15)}
                </g>)
            })
            .collect();
        fframes::svgr!(<g>
            {shots}
            {wash_left(0.95)}
            <rect width="1920" height="1080" fill={NAVY} opacity="0.35" />
            <g opacity={scene_fade(t, Self::LENGTH)}>
                // a neutral rule: the four colleges' colours are the accents here
                {kicker_in("FOUR COLLEGES, ONE CAMPUS", (960.0, 250.0), Align::Middle, t, 0.4, SKY)}
                {colleges}
                {stat(("100%", "OF PROGRAMS TAUGHT IN ENGLISH"), "colleges-english", (960.0, 920.0), 96.0, "#ffffff", t, ENGLISH_AT)}
            </g>
        </g>)
    }
}
