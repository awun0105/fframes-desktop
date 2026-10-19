//! Scene 4, 26–34 s, chapter 01: a full hall, graduation caps and graduation day, while
//! five extruded amber stars spin in for the QS 5-Star rating and the FIBAA accreditation
//! follows.

use std::f32::consts::{PI, TAU};
use std::sync::LazyLock;

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, body, headline, kicker, wash_bottom};
use crate::three_d::{Camera, Face, Rgb, Shading, extrude, render_faces, v3};
use crate::{Grade, NAVY, Shot, ease_out, progress, punch, reel, scene_fade};

super::scene_length!(StarsScene, 3);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("hall.mp4", 0.0),
    Shot::Clip("cap.mp4", 2.8),
    Shot::Photo("graduation.jpg", 5.6),
];
const HEADLINE_AT: f32 = 1.9;
const FIBAA_AT: f32 = 4.6;

/// When star `star` lands, with a chime: a beat's quarter apart.
fn star_at(star: usize) -> f32 {
    1.0 + star as f32 * 0.125
}

pub(crate) fn sfx() -> Vec<Sfx> {
    let mut cues: Vec<Sfx> = (0..5)
        .map(|i| ("chime.wav", star_at(i), -12.0, (i as f32 - 2.0) * 0.25))
        .collect();
    cues.push(("pop.wav", FIBAA_AT, -13.0, 0.0));
    cues
}

static STAR: LazyLock<Vec<Face>> = LazyLock::new(|| {
    let outline: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 1.0 } else { 0.42 };
            let a = PI / 2.0 + i as f32 * PI / 5.0;
            (a.cos() * r, a.sin() * r)
        })
        .collect();
    // the site's amber, darker on the sides
    extrude(&outline, 0.3, Rgb(0.98, 0.65, 0.19), Rgb(0.68, 0.40, 0.07))
});

impl Scene for StarsScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("stars")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(
            &frame,
            ctx,
            Self::INDEX,
            &SHOTS,
            Grade {
                exposure: 0.85,
                ..Default::default()
            },
        );
        let mut world: Vec<Face> = Vec::new();
        for i in 0..5 {
            let land = star_at(i);
            let p = progress(t, land - 0.45, 0.45);
            if p <= 0.0 {
                continue;
            }
            let spin = (1.0 - ease_out(p)) * TAU + (t * 1.2 + i as f32).sin() * 0.15;
            let scale = punch(t, land - 0.45).max(0.0);
            let x = (i as f32 - 2.0) * 2.6;
            world.extend(STAR.iter().map(|f| {
                Face {
                    points: f
                        .points
                        .iter()
                        .map(|&q| (q * scale).rot_y(spin) + v3(x, 0.0, 0.0))
                        .collect(),
                    color: f.color,
                }
            }));
        }
        let camera = Camera {
            yaw: 0.0,
            pitch: 0.05,
            distance: 18.0,
            focal: 1250.0,
            center: (960.0, 330.0),
        };
        let shading = Shading {
            light: v3(-0.3, 0.6, 0.75).normalize(),
            ambient: 0.45,
            ..Default::default()
        };
        let stars = render_faces(&world, |p| p, &camera, &shading);
        fframes::svgr!(<g>
            {shots}
            <rect width="1920" height="1080" fill={NAVY} opacity={0.15 + 0.2 * progress(t, 0.6, 0.6)} />
            {wash_bottom(1.0)}
            <g opacity={scene_fade(t, Self::LENGTH)}>
                <g>{stars}</g>
                {kicker("QS STARS RATING", (960.0, 590.0), Align::Middle, t, HEADLINE_AT - 0.2)}
                {headline(&["YOUNGEST AND FASTEST UNIVERSITY", "IN THE WORLD TO ACHIEVE QS 5 STARS"], (960.0, 680.0), 60.0, Align::Middle, t, HEADLINE_AT)}
                {body("FIBAA accreditation · 100% of criteria met", (960.0, 880.0), 34.0, Align::Middle, t, FIBAA_AT)}
            </g>
        </g>)
    }
}
