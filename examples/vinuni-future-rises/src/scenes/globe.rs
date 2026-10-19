//! Scene 12, 92–102 s, chapter 04: the strategic partners. Over the exchange photo, which
//! dims as the scene starts, a turning globe of land dots (no borders) draws great-circle arcs from Hanoi to
//! Singapore, Ithaca and Philadelphia, and each partner's logo lands as its arc arrives.

use std::fmt::Write;

use fframes::{Duration, FFramesContext, Frame, Scene, ShaderUniforms, Svgr};

use super::Sfx;
use super::art::{Align, CHIP, kicker, logo_chip, wash_bottom};
use crate::geo_dots::LAND;
use crate::three_d::{Camera, V3, polyline};
use crate::{
    AMBER, BLUE, DISPLAY, LABEL, NAVY, PARTICLES, ease_in_out, ease_out, photo, progress,
    scene_fade,
};

super::scene_length!(GlobeScene, 11);

const HANOI: (f32, f32) = (21.03, 105.85);

struct Partner {
    name: &'static str,
    logo: &'static str,
    city: &'static str,
    at: (f32, f32),
    start: f32,
}

const PARTNERS: [Partner; 3] = [
    Partner {
        name: "NTU",
        logo: "logo-ntu.png",
        city: "SINGAPORE",
        at: (1.35, 103.68),
        start: 1.6,
    },
    Partner {
        name: "CORNELL",
        logo: "logo-cornell.png",
        city: "ITHACA",
        at: (42.44, -76.50),
        start: 3.6,
    },
    Partner {
        name: "PENN",
        logo: "logo-penn.png",
        city: "PHILADELPHIA",
        at: (39.95, -75.17),
        start: 5.0,
    },
];
const ARC_FOR: f32 = 1.6;

pub(crate) fn sfx() -> Vec<Sfx> {
    let mut cues: Vec<Sfx> = vec![("riser.wav", 0.0, -12.0, 0.0)];
    cues.extend(
        PARTNERS
            .iter()
            .map(|p| ("chime.wav", p.start + ARC_FOR, -12.0, 0.2)),
    );
    cues
}

/// The camera at scene second `t`: it turns east from Hanoi toward the Americas.
fn camera(t: f32) -> Camera {
    let turn = ease_in_out(progress(t, 0.0, GlobeScene::LENGTH));
    Camera {
        yaw: (100.0 + 80.0 * turn).to_radians(),
        pitch: 0.45,
        distance: 6.0,
        focal: 2280.0 * (0.9 + 0.1 * ease_out(progress(t, 0.0, 1.2))),
        center: (1260.0, 560.0),
    }
}

/// A great-circle arc from `a` to `b` lifted off the surface, up to `p`, as points.
fn great_circle(a: V3, b: V3, p: f32) -> Vec<V3> {
    let omega = a.dot(b).clamp(-1.0, 1.0).acos();
    let steps = (64.0 * p).ceil() as usize;
    (0..=steps)
        .map(|i| {
            let s = (i as f32 / 64.0).min(p);
            let (wa, wb) = (
                ((1.0 - s) * omega).sin() / omega.sin(),
                (s * omega).sin() / omega.sin(),
            );
            let lift =
                1.0 + 0.18 * omega / std::f32::consts::PI * 2.0 * (s * std::f32::consts::PI).sin();
            (a * wa + b * wb) * lift
        })
        .collect()
}

impl Scene for GlobeScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(Self::LENGTH)
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some("globe")
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds();
        let camera = camera(t);
        let dust = PARTICLES.draw(
            &frame,
            ShaderUniforms::new()
                .float("uAlpha", 0.5)
                .float("uPull", 0.3),
        );
        let reveal = progress(t, 0.0, 1.4);
        let radius = camera.focal / camera.distance;
        let mut land = String::new();
        for &(lat, lon) in &LAND {
            let v = camera.view(V3::from_lat_lon(lat, lon));
            // hidden behind the globe, or not yet revealed (the land fills in from the limb)
            if v.z < 0.05 || v.z > reveal * 1.2 {
                continue;
            }
            if let Some((x, y, s)) = camera.project_view(v) {
                let r = 0.003 * s;
                let _ = write!(land, "M{:.1} {y:.1}h{:.1}", x - r, 2.0 * r);
            }
        }
        let home = V3::from_lat_lon(HANOI.0, HANOI.1);
        let front = |v: V3| v.z > 0.0;
        let mut arcs = String::new();
        let mut labels = Vec::new();
        for (i, partner) in PARTNERS.iter().enumerate() {
            let p = ease_in_out(progress(t, partner.start, ARC_FOR));
            if p <= 0.0 {
                continue;
            }
            let to = V3::from_lat_lon(partner.at.0, partner.at.1);
            arcs.push_str(&polyline(&great_circle(home, to, p), &camera, front));
            let k = ease_out(progress(t, partner.start + ARC_FOR, 0.5));
            let v = camera.view(to);
            if let (true, Some((x, y, _))) = (v.z > 0.0, camera.project_view(v)) {
                let dy = if i == 2 { 34.0 } else { -18.0 };
                labels.push(fframes::svgr!(<g opacity={k}>
                    <circle cx={x} cy={y} r={6.0 + 10.0 * (1.0 - k)} fill={AMBER} />
                    <text x={x + 16.0} y={y + dy} font-family={DISPLAY} font-weight={LABEL} font-size="18" letter-spacing="3" fill="#ffffff">{format!("{} · {}", partner.name, partner.city)}</text>
                </g>));
            }
        }
        let hanoi = camera.project(home).filter(|_| camera.view(home).z > 0.0);
        let pulse = (t * 1.6).fract();
        let hanoi = match hanoi {
            Some((x, y, _)) => fframes::svgr!(<g>
                <circle cx={x} cy={y} r={8.0 + 30.0 * pulse} fill="none" stroke={AMBER} stroke-width="2" opacity={1.0 - pulse} />
                <circle cx={x} cy={y} r="7" fill={AMBER} />
                <text x={x - 16.0} y={y + 6.0} text-anchor="end" font-family={DISPLAY} font-weight={LABEL} font-size="18" letter-spacing="3" fill="#ffffff">"HANOI"</text>
            </g>),
            None => Svgr::empty(),
        };
        // the partners' logos stack up on the left in the order their arcs arrive
        let logos: Vec<Svgr<'a>> = PARTNERS
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let at = (110.0 + CHIP.0 / 2.0, 450.0 + i as f32 * (CHIP.1 + 26.0));
                logo_chip(ctx, p.logo, at, t, p.start + ARC_FOR - 0.1, 1.0)
            })
            .collect();
        let (cx, cy) = camera.center;
        // the exchange photo that closes the previous scene carries on at the same push-in
        let backdrop = photo(
            ctx,
            "exchange.jpg",
            "globe-photo",
            t,
            -3.45,
            30.0,
            (0.0, 0.0, 1920.0, 1080.0),
        );
        fframes::svgr!(<g>
            {backdrop}
            // the previous scene's wash, handing over to an even dim
            {wash_bottom(1.0 - progress(t, 0.0, 0.6))}
            <rect width="1920" height="1080" fill={NAVY} opacity={0.8 * ease_out(progress(t, 0.0, 0.6))} />
            <g opacity={scene_fade(t, Self::LENGTH)}>
            <image href={dust.href()} x="0" y="0" width="1920" height="1080" />
            <defs>
                <radialGradient id="globe-glow">
                    <stop offset="0.82" stop-color={BLUE} stop-opacity="0.9" />
                    <stop offset="1" stop-color={BLUE} stop-opacity="0" />
                </radialGradient>
            </defs>
            <circle cx={cx} cy={cy} r={radius * 1.08} fill="url(#globe-glow)" opacity={reveal} />
            <circle cx={cx} cy={cy} r={radius} fill={NAVY} stroke="#5f7fc4" stroke-opacity="0.4" stroke-width="1.5" opacity={reveal} />
            <path d={land} stroke="#8fb0ff" stroke-width={0.012 * radius} stroke-linecap="round" stroke-opacity="0.75" fill="none" />
            <path d={arcs} stroke={AMBER} stroke-width="3" fill="none" stroke-linecap="round" />
            {hanoi}
            {labels}
            {kicker("STRATEGIC PARTNERS", (110.0, 330.0), Align::Start, t, 0.4)}
            {logos}
            </g>
        </g>)
    }
}
