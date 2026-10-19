//! Scene 1, 0–10 s: the intro. A hit in the dark, then a montage cut on every beat (each
//! shot punching in under a light flash, one word every two shots), a push up the main
//! building's spire into a white hit; the white collapses into a plate, the logo's mark
//! assembles from three pieces, the wordmark slides out of the rule, and "HANOI, VIETNAM"
//! tracks in large under it, over the campus from the air.

use fframes::{Duration, FFramesContext, Frame, Scene, ShaderUniforms, Svgr};

use super::Sfx;
use crate::{
    AMBER, BEAT, DISPLAY, Grade, NAVY, PARTICLES, SANS, SKY, ease_in_out, ease_out, footage,
    progress, punch,
};

super::scene_length!(DawnScene, 0);

/// One clip per beat from [`MONTAGE_AT`].
const MONTAGE: [&str; 8] = [
    "sunrise.mp4",
    "machineshop.mp4",
    "hackathon.mp4",
    "arrivals.mp4",
    "labteach.mp4",
    "studypair.mp4",
    "aerialgold.mp4",
    "horizon.mp4",
];
const MONTAGE_AT: f32 = 0.5;
/// A word every two shots.
const WORDS: [&str; 4] = ["THINK", "BUILD", "CONNECT", "LEAD"];
/// The push up the spire, into the white hit.
const SPIRE_AT: f32 = MONTAGE_AT + MONTAGE.len() as f32 * BEAT;
/// The white hit: the riser's boom and the chime; the mark's pieces land on the next
/// three sixteenths, the wordmark and the place follow.
const HIT_AT: f32 = 6.0;
const WORDMARK_AT: f32 = 7.0;
const PLACE_AT: f32 = 7.75;
const MOTTO_AT: f32 = 8.5;

pub(crate) fn sfx() -> Vec<Sfx> {
    let mut cues = vec![
        ("sub.wav", 0.0, -4.0, 0.0),
        ("swoosh.wav", SPIRE_AT - 0.3, -10.0, 0.0),
        ("riser.wav", HIT_AT - 1.5, -6.0, 0.0),
        ("chime.wav", HIT_AT, -9.0, 0.0),
        ("blip.wav", WORDMARK_AT, -15.0, 0.2),
        ("pop.wav", PLACE_AT, -12.0, 0.0),
    ];
    cues.extend((0..3).map(|i| ("pop.wav", piece_at(i), -13.0, i as f32 * 0.3 - 0.3)));
    cues
}

fn piece_at(i: usize) -> f32 {
    HIT_AT + 0.15 + i as f32 * BEAT / 2.0
}

/// `logo.png` (711x153): the mark spans columns 10–142, the rule 155–160 and the wordmark
/// 174–699.
const LOGO: (f32, f32) = (711.0, 153.0);
/// The logo's height and centre on the plate.
const LOGO_H: f32 = 170.0;
const CENTER: (f32, f32) = (960.0, 470.0);
/// A piece of the mark: its box in logo pixels, and where it flies in from (px, degrees).
type Piece = ((f32, f32, f32, f32), (f32, f32, f32));
/// The mark's pieces: disjoint boxes, top left with the red tip, bottom left, right.
const PIECES: [Piece; 3] = [
    ((0.0, 0.0, 76.0, 70.0), (-260.0, -220.0, -50.0)),
    ((0.0, 70.0, 76.0, 83.0), (-220.0, 260.0, 40.0)),
    ((76.0, 0.0, 74.0, 153.0), (300.0, -120.0, 70.0)),
];

/// The montage: the shot of the current beat, punching in from 18% larger, under a light
/// flash, and the word of the current two beats.
fn montage<'a>(frame: &Frame, ctx: &FFramesContext<'a, '_>, t: f32) -> Svgr<'a> {
    let beat = ((t - MONTAGE_AT) / BEAT).floor();
    if beat < 0.0 || beat as usize >= MONTAGE.len() {
        return Svgr::empty();
    }
    let i = beat as usize;
    let at = MONTAGE_AT + i as f32 * BEAT;
    let shot = footage(
        frame,
        ctx,
        MONTAGE[i],
        at,
        Grade {
            amount: 0.2,
            zoom: 1.0 + 0.18 * (1.0 - punch(t, at)),
            ..Default::default()
        },
        (0.0, 0.0, 1920.0, 1080.0),
    );
    let flash = 0.35 * (1.0 - progress(t, at, 0.18));
    let word_at = MONTAGE_AT + (i / 2) as f32 * 2.0 * BEAT;
    let scale = 1.3 - 0.3 * punch(t, word_at);
    let spacing = 40.0 + 30.0 * progress(t, word_at, 2.0 * BEAT);
    fframes::svgr!(<g>
        {shot}
        <rect width="1920" height="1080" fill={NAVY} opacity="0.3" />
        <rect width="1920" height="1080" fill="#ffffff" opacity={flash} />
        <g transform={format!("translate(960 600) scale({scale})")} opacity={progress(t, word_at, 0.08)}>
            <text x="0" y="0" text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size="170" letter-spacing={spacing} fill="#ffffff">{WORDS[i / 2]}</text>
        </g>
    </g>)
}

/// The logo assembling on its plate: the mark's pieces fly in, then the whole logo
/// replaces them (no seams at rest) while the wordmark slides out from the rule.
fn logo<'a>(ctx: &FFramesContext<'a, '_>, t: f32) -> Svgr<'a> {
    let Some(image) = ctx.get_image("logo.png") else {
        return Svgr::empty();
    };
    let scale = LOGO_H / LOGO.1;
    let (x, y) = (CENTER.0 - LOGO.0 * scale / 2.0, CENTER.1 - LOGO_H / 2.0);
    let href = image.href();
    let settled = t >= piece_at(2) + 0.45;
    let mark = if settled {
        // the mark and the rule, whole
        fframes::svgr!(<g>
            <defs><clipPath id="dawn-mark"><rect x="0" y="0" width="166" height="153" /></clipPath></defs>
            <g transform={format!("translate({x} {y}) scale({scale})")} clip-path="url(#dawn-mark)">
                <image href={href.clone()} x="0" y="0" width={LOGO.0} height={LOGO.1} />
            </g>
        </g>)
    } else {
        let pieces: Vec<Svgr<'a>> = PIECES
            .iter()
            .enumerate()
            .map(|(i, &((px, py, pw, ph), (dx, dy, spin)))| {
                let k = ease_out(progress(t, piece_at(i), 0.45));
                let (cx, cy) = (px + pw / 2.0, py + ph / 2.0);
                let id = ["dawn-piece-a", "dawn-piece-b", "dawn-piece-c"][i];
                fframes::svgr!(<g opacity={progress(t, piece_at(i), 0.1)} transform={format!(
                    "translate({x} {y}) scale({scale}) translate({} {}) rotate({} {cx} {cy})",
                    dx * (1.0 - k),
                    dy * (1.0 - k),
                    spin * (1.0 - k)
                )}>
                    <defs><clipPath id={id}><rect x={px} y={py} width={pw} height={ph} /></clipPath></defs>
                    <g clip-path={format!("url(#{id})")}>
                        <image href={href.clone()} x="0" y="0" width={LOGO.0} height={LOGO.1} />
                    </g>
                </g>)
            })
            .collect();
        fframes::svgr!(<g>{pieces}</g>)
    };
    // the rule grows, then the wordmark slides out from behind it
    let rule = ease_out(progress(t, WORDMARK_AT - 0.25, 0.3));
    let open = ease_in_out(progress(t, WORDMARK_AT, 0.6));
    let rule_y = y + LOGO_H / 2.0;
    fframes::svgr!(<g>
        {mark}
        <rect x={x + 155.0 * scale} y={rule_y - 66.0 * scale * rule} width={5.0 * scale} height={(132.0 * scale * rule).max(0.01)} fill="#231f20" opacity={if settled { 0.0 } else { rule }} />
        <defs><clipPath id="dawn-wordmark"><rect x={x + 166.0 * scale} y={y} width={(540.0 * scale * open).max(0.01)} height={LOGO_H} /></clipPath></defs>
        <g clip-path="url(#dawn-wordmark)">
            <g transform={format!("translate({} 0)", -120.0 * (1.0 - open))}>
                <image href={href} x={x} y={y} width={LOGO.0 * scale} height={LOGO_H} />
            </g>
        </g>
    </g>)
}

impl Scene for DawnScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("dawn")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        // the spire, pushed in harder and harder and burnt out into the hit
        let push = ease_in_out(progress(t, SPIRE_AT, HIT_AT - SPIRE_AT));
        let spire = if (SPIRE_AT..HIT_AT).contains(&t) {
            footage(
                &frame,
                ctx,
                "spire.mp4",
                SPIRE_AT,
                Grade {
                    amount: 0.2,
                    zoom: 1.0 + 0.6 * push * push,
                    exposure: 1.0 + 1.2 * push * push,
                    ..Default::default()
                },
                (0.0, 0.0, 1920.0, 1080.0),
            )
        } else {
            Svgr::empty()
        };
        // after the hit: the campus from the air, dark, drifting out, with the dust
        let after = progress(t, HIT_AT, 0.4);
        let aerial = if t >= HIT_AT {
            footage(
                &frame,
                ctx,
                "aerial.mp4",
                HIT_AT,
                Grade {
                    amount: 0.4,
                    zoom: 1.12 - 0.08 * ease_out(progress(t, HIT_AT, 4.0)),
                    exposure: 0.5,
                    ..Default::default()
                },
                (0.0, 0.0, 1920.0, 1080.0),
            )
        } else {
            Svgr::empty()
        };
        let dust = PARTICLES.draw(
            &frame,
            ShaderUniforms::new()
                .float("uAlpha", 0.3 * after)
                .float("uPull", 0.6),
        );
        // letterbox bars through the montage, opening for the spire
        let bars =
            (120.0 * progress(t, 0.2, 0.3) * (1.0 - ease_in_out(progress(t, SPIRE_AT, 0.6))))
                .max(0.01);
        // the white builds into the hit, then collapses into the plate
        let white_in = progress(t, HIT_AT - 0.3, 0.3);
        let collapse = ease_in_out(progress(t, HIT_AT, 0.5));
        let (plate_w, plate_h) = (LOGO.0 * LOGO_H / LOGO.1 + 2.0 * 64.0, LOGO_H + 2.0 * 50.0);
        let white_w = 1920.0 + (plate_w - 1920.0) * collapse;
        let white_h = 1080.0 + (plate_h - 1080.0) * collapse;
        let white_y = 540.0 + (CENTER.1 - 540.0) * collapse - white_h / 2.0;
        let white = if t < HIT_AT { white_in } else { 1.0 };
        // the place: large, tracking in, between two amber rules
        let place = ease_out(progress(t, PLACE_AT, 0.7));
        let place_spacing = 26.0 + 40.0 * (1.0 - place);
        // the rules grow once the tracking has settled, so they never reach into the letters
        let rules = 180.0 * ease_out(progress(t, PLACE_AT + 0.5, 0.5));
        let motto = ease_out(progress(t, MOTTO_AT, 0.6));
        let out = 1.0 - progress(t, Self::LENGTH - 0.35, 0.3);
        fframes::svgr!(<g>
            <rect width="1920" height="1080" fill="#000000" />
            {montage(&frame, ctx, t)}
            {spire}
            <g opacity={after}>
                {aerial}
                <rect width="1920" height="1080" fill={NAVY} opacity="0.45" />
                <image href={dust.href()} x="0" y="0" width="1920" height="1080" />
            </g>
            <rect x="0" y="0" width="1920" height={bars} fill="#000000" />
            <rect x="0" y={1080.0 - bars} width="1920" height={bars} fill="#000000" />
            <g opacity={out}>
                <rect x={960.0 - white_w / 2.0} y={white_y} width={white_w} height={white_h} fill="#ffffff" opacity={white} />
                <rect x={960.0 - white_w / 2.0} y={white_y + white_h} width={white_w} height="4" fill={AMBER} opacity={collapse} />
                {logo(ctx, t)}
                <g opacity={place} transform={format!("translate(0 {})", (1.0 - place) * 24.0)}>
                    <text x="960" y="742" text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size="64" letter-spacing={place_spacing} fill="#ffffff">"HANOI, VIETNAM"</text>
                </g>
                <rect x={470.0 - rules} y="718" width={rules.max(0.01)} height="3" fill={AMBER} opacity={place} />
                <rect x="1450" y="718" width={rules.max(0.01)} height="3" fill={AMBER} opacity={place} />
                <text x="960" y="830" text-anchor="middle" font-family={SANS} font-size="36" letter-spacing="2" fill={SKY} opacity={motto} transform={format!("translate(0 {})", (1.0 - motto) * 16.0)}>"A University of Excellence"</text>
            </g>
        </g>)
    }
}
