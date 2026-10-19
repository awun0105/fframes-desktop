//! Scene 15, 116–124 s, chapter 05: a high five, a game of basketball and the pool, while the
//! five core values land one letter per two beats: E·X·C·E·L.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, kicker};
use crate::{
    AMBER, DISPLAY, Grade, LABEL, NAVY, Shot, ease_out, progress, punch, reel, scene_fade,
};

super::scene_length!(ValuesScene, 14);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("highfive.mp4", 0.0),
    Shot::Clip("hoops.mp4", 4.5),
    Shot::Clip("pool.mp4", 6.5),
];
/// Letter, the value it starts (two lines), the beat it lands on.
const VALUES: [(&str, [&str; 2], f32); 5] = [
    ("E", ["EMPATHY", ""], 0.5),
    ("X", ["EXCEPTIONAL", "CAPABILITY"], 1.5),
    ("C", ["CREATIVITY", ""], 2.5),
    ("E", ["ENTREPRENEURIAL", "SPIRIT"], 3.5),
    ("L", ["LEADERSHIP", "MINDSET"], 4.5),
];

pub(crate) fn sfx() -> Vec<Sfx> {
    VALUES
        .iter()
        .enumerate()
        .map(|(i, &(_, _, at))| ("blip.wav", at, -12.0, (i as f32 - 2.0) * 0.3))
        .collect()
}

impl Scene for ValuesScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("values")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        let columns: Vec<Svgr<'a>> = VALUES
            .iter()
            .enumerate()
            .map(|(i, &(letter, [first, second], at))| {
                let x = 960.0 + (i as f32 - 2.0) * 340.0;
                let scale = punch(t, at).max(0.0);
                let word = ease_out(progress(t, at + 0.15, 0.4));
                fframes::svgr!(<g>
                    <g transform={format!("translate({x} 600) scale({scale})")}>
                        <text x="0" y="0" text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size="170" fill={AMBER}>{letter}</text>
                    </g>
                    <g opacity={word}>
                        <text x={x} y="680" text-anchor="middle" font-family={DISPLAY} font-weight={LABEL} font-size="20" letter-spacing="4" fill="#ffffff">{first}</text>
                        <text x={x} y="710" text-anchor="middle" font-family={DISPLAY} font-weight={LABEL} font-size="20" letter-spacing="4" fill="#ffffff">{second}</text>
                    </g>
                </g>)
            })
            .collect();
        fframes::svgr!(<g>
            {shots}
            <rect width="1920" height="1080" fill={NAVY} opacity={0.25 + 0.3 * progress(t, 0.2, 0.4)} />
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("OUR CORE VALUES", (960.0, 360.0), Align::Middle, t, 0.2)}
                {columns}
            </g>
        </g>)
    }
}
