//! Act 5: how it works. A finished frame tilts into an isometric view and comes
//! apart into its three layers, a GPU shader, vector shapes and the timeline
//! that moves them. Each is named in turn, then they snap back into one frame.

use fframes::{Color, Duration, FFramesContext, Frame, Scene, ShaderUniforms, Svgr};

use super::look::*;
use crate::beat::*;
use crate::shaders::SHADERS;

beat_scene!(LayersScene, Some(176.0), Some(208.0));

const TILT: (f32, f32) = (3.0, 5.0);
const APART: f32 = 5.0;
/// Beats each layer is named on: shader, shapes, timeline.
const NAMED: [f32; 3] = [8.0, 13.0, 18.0];
const TOGETHER: f32 = 23.5;
const MERGE: (f32, f32) = (25.0, 27.5);

/// Video beats the layers are named on.
pub const BLIP_BEATS: [f32; 3] = [184.0, 189.0, 194.0];

const PW: f32 = 960.0;
const PH: f32 = 540.0;
/// Centre of the stack on screen.
const CX: f32 = 800.0;
const CY: f32 = 630.0;
const GAP: f32 = 190.0;

struct Layer {
    number: &'static str,
    name: &'static str,
    lines: [&'static str; 2],
}

const LAYERS: [Layer; 3] = [
    Layer {
        number: "01",
        name: "SHADERS & 3D",
        lines: ["Lit 3D scenes and SkSL", "or Shadertoy shaders."],
    },
    Layer {
        number: "02",
        name: "VECTOR SHAPES",
        lines: ["svgr! trees and Lottie files,", "sharp at 1080p or 4K."],
    },
    Layer {
        number: "03",
        name: "TIMELINE",
        lines: ["timeline! says when each", "thing moves, on the beat."],
    },
];

/// The plane transform: flat at `tilt` 0, at 1 the axonometric projection of
/// the system's stacks, `matrix(.866 .5 -.866 .5)`, raised by `lift` px.
fn plane(tilt: f32, lift: f32) -> (f32, f32, f32, f32, f32, f32) {
    let k = 0.5;
    let iso = (0.866 * k, 0.5 * k, -0.866 * k, 0.5 * k);

    let flat = (0.82, 0.0, 0.0, 0.82);
    let a = lerp(flat.0, iso.0, tilt);
    let b = lerp(flat.1, iso.1, tilt);
    let c = lerp(flat.2, iso.2, tilt);
    let d = lerp(flat.3, iso.3, tilt);
    let cy = CY - lift;
    let e = CX - (a * PW / 2.0 + c * PH / 2.0);
    let f = cy - (b * PW / 2.0 + d * PH / 2.0);
    (a, b, c, d, e, f)
}

fn apply(m: (f32, f32, f32, f32, f32, f32), x: f32, y: f32) -> (f32, f32) {
    (m.0 * x + m.2 * y + m.4, m.1 * x + m.3 * y + m.5)
}

fn matrix(m: (f32, f32, f32, f32, f32, f32)) -> String {
    format!("matrix({} {} {} {} {} {})", m.0, m.1, m.2, m.3, m.4, m.5)
}

impl Scene for LayersScene {
    fn duration(&self) -> Duration<'_> {
        Self::frames()
    }

    fn render_frame<'a>(&'a self, frame: Frame, _ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let lb = Self::lb(&frame);
        let tilt = cubic_in_out(prog(lb, TILT.0, TILT.1))
            * (1.0 - cubic_in_out(prog(lb, MERGE.0 + 1.0, MERGE.1)));
        let named = NAMED.iter().rposition(|n| lb >= *n && lb < TOGETHER);

        let mut planes = Vec::new();
        let mut labels = Vec::new();
        for (i, layer) in LAYERS.iter().enumerate() {
            let start = APART + i as f32 * 0.35;
            let apart =
                soft(lb - start).max(0.0) * (1.0 - cubic_in_out(prog(lb, MERGE.0, MERGE.0 + 1.0)));
            let lift = (i as f32 - 1.0) * GAP * apart;
            let m = plane(tilt, lift);
            let dim = match named {
                Some(n) if n != i => 0.3,
                _ => 1.0,
            };
            let body = match i {
                0 => shader_layer(&frame),
                1 => shapes_layer(lb),
                _ => timeline_layer(lb),
            };
            let edge = if named == Some(i) { ACTION } else { CONTROL };
            planes.push(fframes::svgr!(
                <g transform={matrix(m)} opacity={dim}>
                    {body}
                    <rect x="0" y="0" width={PW} height={PH} fill="none" stroke={edge} stroke-width="2.5" />
                </g>
            ));

            // the name, with a leader to the plane's right corner
            let shown = prog(lb, NAMED[i], NAMED[i] + 0.4) * leave(lb, MERGE.0 - 0.5);
            if shown > 0.0 {
                let (px, py) = apply(m, PW, PH / 2.0);
                let lx = 1380.0;
                let ly = CY - lift + 8.0;
                let o = if named.is_none_or(|n| n == i) {
                    1.0
                } else {
                    0.4
                };
                labels.push(fframes::svgr!(
                    <g opacity={shown * o}>
                        <path d={format!("M{px} {py} L{} {ly} L{lx} {ly}", lx - 40.0)} fill="none" stroke={ACTION} stroke-width="1.5" />
                        <circle cx={px} cy={py} r="5" fill={ACTION} />
                        <text x={lx + 16.0} y={ly - 22.0} font-family={MONO} font-weight="500" font-size="18" letter-spacing="3" fill={ACTION}>{layer.number}" "{layer.name}</text>
                        <text x={lx + 16.0} y={ly + 14.0} font-family={SANS} font-weight="400" font-size="24" fill={TEXT}>{layer.lines[0]}</text>
                        <text x={lx + 16.0} y={ly + 44.0} font-family={SANS} font-weight="400" font-size="24" fill={SECONDARY}>{layer.lines[1]}</text>
                    </g>
                ));
            }
        }

        let loupe = loupe(lb);
        let (eo, ed) = rise(lb, 0.0, 24.0);
        let (ho, hd) = rise(lb, 0.4, 40.0);
        fframes::svgr!(
            <g opacity={leave(lb, 31.4)}>
                <g opacity={eo} transform={at(0.0, ed)}>{act_eyebrow("05", "HOW IT WORKS")}</g>
                <g opacity={ho} transform={at(0.0, hd)}>{headline(116.0, 252.0, "Three layers, one frame.", 72.0, TEXT)}</g>
                {planes}
                {labels}
                {loupe}
                {caption(lb, MERGE.1, 31.0, "Every frame:", "shaders, vector shapes and a timeline, drawn together.", ACTION)}
            </g>
        )
    }
}

fn shader_layer(frame: &Frame) -> Svgr<'static> {
    let layer = SHADERS.contour.draw(
        frame,
        ShaderUniforms::new()
            .float2("uWell", 0.62, 0.48)
            .float("uDepth", 0.8)
            .float("uDensity", 16.0)
            .float("uBright", 0.9)
            .color("uInk", Color::hex(CANDIDATE_SURFACE))
            .color("uHot", Color::hex(COMMAND)),
    );
    // the shader draws on the inverse surface, like the code that makes it
    fframes::svgr!(
        <g>
            <rect x="0" y="0" width={PW} height={PH} fill={INVERSE} />
            <image href={layer.href()} x="0" y="0" width={PW} height={PH} />
        </g>
    )
}

/// The vector layer: the title card's text and shapes.
fn shapes_layer(lb: f32) -> Svgr<'static> {
    let spin = lb * 12.0;
    fframes::svgr!(
        <g>
            <rect x="0" y="0" width={PW} height={PH} fill={PANEL} opacity="0.88" />
            <text x="70" y="250" font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="104" letter-spacing="-4.2" fill={TEXT}>"Launch day"</text>
            <text x="74" y="310" font-family={SANS} font-weight="400" font-size="30" fill={SECONDARY}>"FFrames Studio"</text>
            <circle cx="770" cy="200" r="90" fill="none" stroke={CANDIDATE} stroke-width="10" />
            <g transform={format!("rotate({spin} 770 200)")}>
                <rect x="730" y="160" width="80" height="80" fill={CANDIDATE} />
            </g>
            <rect x="74" y="380" width="260" height="64" rx={RADIUS} fill={ACTION} />
            <text x="204" y="421" text-anchor="middle" font-family={SANS} font-weight="500" font-size="26" fill={ON_ACTION}>"Get it"</text>
        </g>
    )
}

/// The timeline layer: tracks, keyframes and the playhead.
fn timeline_layer(lb: f32) -> Svgr<'static> {
    let head = 80.0 + (lb * 0.06).fract() * 800.0;
    let rows: Vec<Svgr> = (0..3)
        .map(|r| {
            let y = 380.0 + r as f32 * 44.0;
            let keys: Vec<Svgr> = (0..5)
                .map(|k| {
                    let x = 120.0 + k as f32 * 170.0 + r as f32 * 40.0;
                    let d = format!(
                        "M{x} {} L{} {} L{x} {} L{} {} Z",
                        y - 9.0,
                        x + 9.0,
                        y,
                        y + 9.0,
                        x - 9.0,
                        y
                    );
                    fframes::svgr!(<path d={d} fill={TEXT} />)
                })
                .collect();
            fframes::svgr!(
                <g>
                    <rect x="80" y={y - 2.0} width="800" height="4" fill={EMPHASIS} />
                    {keys}
                </g>
            )
        })
        .collect();
    fframes::svgr!(
        <g>
            <rect x="0" y="0" width={PW} height={PH} fill={PANEL} opacity="0.88" />
            <text x="80" y="90" font-family={MONO} font-weight="400" font-size="24" fill={TEXT}>"at 0.0, animate 80 => 0, Easing::Spring"</text>
            <text x="80" y="130" font-family={MONO} font-weight="400" font-size="24" fill={SECONDARY}>"at 2.0 => 2.6, animate 1 => 0, Easing::EaseIn"</text>
            {rows}
            <rect x={head} y="340" width="3" height="150" fill={TEXT} />
        </g>
    )
}

/// A loupe on one letter of the vector layer, many times its size and still sharp.
fn loupe(lb: f32) -> Svgr<'static> {
    let from = NAMED[1] + 0.8;
    let o = prog(lb, from, from + 0.4) * leave(lb, NAMED[2] - 0.4);
    if o <= 0.0 {
        return Svgr::empty();
    }
    let s = 0.85 + 0.15 * snap(lb - from);
    let (cx, cy, r) = (1180.0, 400.0, 120.0);
    fframes::svgr!(
        <g opacity={o} transform={zoom(cx, cy, s)}>
            <clipPath id="loupe"><circle cx={cx} cy={cy} r={r} /></clipPath>
            <circle cx={cx} cy={cy} r={r} fill={PANEL} />
            <g clip-path="url(#loupe)">
                <text x={cx} y={cy + 72.0} text-anchor="middle" font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="280" fill={TEXT}>"a"</text>
            </g>
            <circle cx={cx} cy={cy} r={r} fill="none" stroke={CANDIDATE} stroke-width="2.5" />
            <text x={cx} y={cy + r + 36.0} text-anchor="middle" font-family={MONO} font-weight="500" font-size="16" letter-spacing="3" fill={CANDIDATE}>"STILL SHARP"</text>

        </g>
    )
}
