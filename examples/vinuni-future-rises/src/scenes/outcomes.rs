//! Scene 16, 124–132 s, chapter 06: graduates in their gowns, students at the Career Day,
//! employers' booths; where `VinUni` graduates go: the share of Cohort 2 employed or in
//! further study (`VinUni` Annual Report 2025), then the logos of the companies that offered
//! graduates jobs ("`VinUniversity` graduates the first academic cohort" on vinuni.edu.vn and
//! the annual report). The universities follow in [`super::study`].

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, body, headline, kicker, logo_wall};
use crate::{DISPLAY, Grade, LABEL, NAVY, SKY, Shot, ease_out, progress, reel, scene_fade};

super::scene_length!(OutcomesScene, 15);

pub(crate) const SHOTS: [Shot; 3] = [
    Shot::Clip("gowns.mp4", 0.0),
    Shot::Clip("careerday.mp4", 2.2),
    Shot::Clip("booths.mp4", 5.0),
];
const SHARE_AT: f32 = 0.5;
/// Where graduates were employed (Annual Report 2025) or received job offers (the first
/// cohort's article); an employment is an offer, so the headline says offers.
const EMPLOYERS: [&str; 8] = [
    "logo-google.png",
    "logo-qualcomm.png",
    "logo-bcg.png",
    "logo-unilever.png",
    "logo-pg.png",
    "logo-mckinsey.png",
    "logo-ibm.png",
    "logo-bosch.png",
];
const WORK_AT: f32 = 3.0;
/// The logos land a quarter beat apart, four to a row.
pub(crate) const STEP: f32 = 0.25;

/// A blip on every other logo of `count` landing from `at`.
pub(crate) fn logo_blips(at: f32, count: usize) -> impl Iterator<Item = Sfx> {
    (0..count).step_by(2).map(move |i| {
        (
            "blip.wav",
            at + i as f32 * STEP,
            -14.0,
            (i as f32 / 3.0 - 1.0) * 0.4,
        )
    })
}

pub(crate) fn sfx() -> Vec<Sfx> {
    let mut cues = vec![
        ("pop.wav", SHARE_AT, -12.0, 0.0),
        ("chime.wav", SHARE_AT + 0.6, -12.0, 0.0),
    ];
    cues.extend(logo_blips(WORK_AT + 0.3, EMPLOYERS.len()));
    cues
}

/// The share counting up to 99% over a second from [`SHARE_AT`], growing as it counts and
/// pulsing once as it lands.
fn share(t: f32) -> Svgr<'static> {
    let count = ease_out(progress(t, SHARE_AT, 1.0));
    let pulse = (std::f32::consts::PI * progress(t, SHARE_AT + 0.95, 0.3)).sin();
    let scale = 0.8 + 0.2 * count + 0.1 * pulse;
    let value = format!("{}%", (99.0 * count).round());
    fframes::svgr!(<g opacity={progress(t, SHARE_AT, 0.15)} transform={format!("translate(960 620) scale({scale})")}>
        <text x="0" y="0" text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size="220" fill="#ffffff">{value}</text>
    </g>)
}

impl Scene for OutcomesScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("outcomes")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let shots = reel(&frame, ctx, Self::INDEX, &SHOTS, Grade::default());
        let share_out = 1.0 - progress(t, WORK_AT - 0.25, 0.25);
        let label = ease_out(progress(t, SHARE_AT + 0.5, 0.5));
        fframes::svgr!(<g>
            {shots}
            <rect width="1920" height="1080" fill={NAVY} opacity="0.55" />
            <g opacity={scene_fade(t, Self::LENGTH)}>
                {kicker("WHERE OUR GRADUATES GO", (960.0, 330.0), Align::Middle, t, 0.2)}
                <g opacity={share_out}>
                    {share(t)}
                    <g opacity={label} transform={format!("translate(0 {})", (1.0 - label) * 16.0)}>
                        <text x="960" y="700" text-anchor="middle" font-family={DISPLAY} font-weight={LABEL} font-size="24" letter-spacing="4" fill={SKY}>"OF COHORT 2 GRADUATES WORKING OR IN FURTHER STUDY"</text>
                        <text x="960" y="740" text-anchor="middle" font-family={DISPLAY} font-weight={LABEL} font-size="24" letter-spacing="4" fill={SKY}>"WITHIN SIX MONTHS OF GRADUATION"</text>
                    </g>
                </g>
                {headline(&["JOB OFFERS FROM"], (960.0, 470.0), 60.0, Align::Middle, t, WORK_AT)}
                {logo_wall(ctx, &EMPLOYERS, 4, 530.0, (t, WORK_AT + 0.3, STEP, Self::LENGTH + 1.0))}
                {body("and top Vietnamese technology companies and banks", (960.0, 880.0), 30.0, Align::Middle, t, WORK_AT + 2.4)}
            </g>
        </g>)
    }
}
