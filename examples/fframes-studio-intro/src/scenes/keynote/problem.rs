//! Act 1: why making video hurts today. Three screens side by side and the
//! camera whips from one to the next: an editor timeline full of keyframes, a
//! browser renderer screenshotting one frame at a time, and a coding agent that
//! cannot see what it rendered. The act ends by striking the old way out.

use std::sync::LazyLock;

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};
use fframes_components::components::showcase::strike_through::{
    StrikeThrough, StrikeThroughConfig,
};

use super::comp;
use super::look::*;
use crate::beat::*;

beat_scene!(ProblemScene, None, Some(48.0));

/// Beats the camera starts whipping to the next screen on.
const WHIPS: [f32; 2] = [15.5, 31.5];
/// Beat the three screens give way to the strike-through.
const RECAP: f32 = 44.0;
/// Frames of the video the browser renderer is working through.
const BROWSER_FRAMES: f32 = 300.0;
/// How many screenshots it takes a second, for the picture: a few.
const BROWSER_FPS: f32 = 2.5;

/// Seconds (video time) of the screenshots that get a shutter sound.
pub fn shutter_times() -> Vec<f32> {
    (0..3).map(|k| beat_time(16.0) + shot_time(k)).collect()
}

/// Seconds after the browser screen appears that its `k`th screenshot is taken.
fn shot_time(k: usize) -> f32 {
    0.6 + k as f32 / BROWSER_FPS
}

static STRIKE: LazyLock<StrikeThrough> = LazyLock::new(|| {
    StrikeThrough::new(StrikeThroughConfig {
        before: "Make video the".to_owned(),
        struck: "slow".to_owned(),
        after: "way.".to_owned(),
        text_color: TEXT.to_owned(),
        strike_color: FAILURE.to_owned(),
        stroke_width: 7.0,
        font_family: Some(SANS.to_owned()),
        ..Default::default()
    })
});

fn expo_in_out(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        expo_in(t * 2.0) / 2.0
    } else {
        0.5 + expo_out(t * 2.0 - 1.0) / 2.0
    }
}

impl Scene for ProblemScene {
    fn duration(&self) -> Duration<'_> {
        Self::frames()
    }

    fn render_frame<'a>(&'a self, frame: Frame, _ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let lb = Self::lb(&frame);
        let whip = |at: f32| expo_in_out(prog(lb, at, at + 1.0));
        let cam = WHIPS.iter().map(|w| whip(*w)).sum::<f32>();
        // how fast the camera moves, for the smear while it whips
        let speed = WHIPS
            .iter()
            .map(|w| (1.0 - ((lb - w - 0.5).abs() * 4.0).min(1.0)).max(0.0))
            .fold(0.0_f32, f32::max);

        let screens: Vec<Svgr> = (0..3)
            .filter(|i| (cam - *i as f32).abs() < 1.0)
            .map(|i| {
                let body = match i {
                    0 => editor(lb),
                    1 => browser(lb),
                    _ => agent(lb),
                };
                let x = (i as f32 - cam) * 1920.0;
                fframes::svgr!(<g transform={at(x, 0.0)}>{body}</g>)
            })
            .collect();
        let smear = if speed > 0.05 {
            let copies: Vec<Svgr> = [-1.0_f32, 1.0]
                .iter()
                .map(|dir| {
                    let shifted: Vec<Svgr> = (0..3)
                        .filter(|i| (cam - *i as f32).abs() < 1.0)
                        .map(|i| {
                            let body = match i {
                                0 => editor(lb),
                                1 => browser(lb),
                                _ => agent(lb),
                            };
                            let x = (i as f32 - cam) * 1920.0 + dir * 70.0 * speed;
                            fframes::svgr!(<g transform={at(x, 0.0)}>{body}</g>)
                        })
                        .collect();
                    fframes::svgr!(<g opacity={0.22 * speed}>{shifted}</g>)
                })
                .collect();
            fframes::svgr!(<g>{copies}</g>)
        } else {
            Svgr::empty()
        };

        let recap = prog(lb, RECAP - 0.5, RECAP);
        let (eo, ed) = rise(lb, -4.2, 24.0);
        let captions = fframes::svgr!(
            <g>
                {caption(lb, 1.0, 15.0, "Traditional editors:", "every change means dragging keyframes by hand.", FAILURE)}
                {caption(lb, 17.0, 31.0, "Browser frameworks:", "a web page is screenshotted for every frame.", FAILURE)}
                {caption(lb, 33.0, RECAP - 1.0, "AI coding agents:", "great at code, blind to what they render.", FAILURE)}
            </g>
        );
        let strike = comp::show(
            &*STRIKE,
            &frame,
            (lb - RECAP) * BEAT,
            (0.0, 0.0, 1920.0, 1080.0),
        );
        fframes::svgr!(
            <g>
                <g opacity={1.0 - recap} transform={zoom(960.0, 600.0, 1.0 - 0.06 * recap)}>
                    {smear}
                    {screens}
                    <g opacity={eo} transform={at(0.0, ed)}>{act_eyebrow("01", "THE PROBLEM")}</g>
                </g>
                {captions}
                <g opacity={leave(lb, 47.4)}>{strike}</g>
            </g>
        )
    }
}

/// A screen's headline under the act label.
fn screen_title(lb: f32, from: f32, text: &'static str) -> Svgr<'static> {
    let (o, d) = rise(lb, from, 40.0);
    fframes::svgr!(<g opacity={o} transform={at(0.0, d)}>{headline(116.0, 252.0, text, 72.0, TEXT)}</g>)
}

/// A keyframe diamond.
fn diamond(x: f32, y: f32, r: f32, fill: &'static str) -> Svgr<'static> {
    let d = format!(
        "M{x} {} L{} {y} L{x} {} L{} {y} Z",
        y - r,
        x + r,
        y + r,
        x - r
    );
    fframes::svgr!(<path d={d} fill={fill} />)
}

// ---------------------------------------------------------------------------
// the editor: a timeline that fills up with keyframes

const TRACKS: [&str; 7] = [
    "V4 TITLE",
    "V3 LOGO",
    "V2 SHAPES",
    "V1 CLIP",
    "FX GLOW",
    "A1 MUSIC",
    "A2 VOICE",
];

fn editor(lb: f32) -> Svgr<'static> {
    let (wo, wd) = rise(lb, -2.0, 60.0);
    let (x0, y0, w, h) = (120.0, 300.0, 1680.0, 610.0);
    let lane_x = x0 + 190.0;
    let lane_w = w - 220.0;
    let row_y = |i: usize| y0 + 300.0 + i as f32 * 40.0;

    // the cursor drags the title clip back and forth, faster and faster
    let drag = if lb > 4.0 {
        ((lb - 4.0) * (1.6 + (lb - 4.0) * 0.12)).sin() * 90.0
    } else {
        0.0
    };

    let mut keys = Vec::new();
    let mut shown = 0;
    for (i, _) in TRACKS.iter().enumerate() {
        for k in 0..44 {
            let n = (i * 44 + k) as f32;
            // keyframes land in waves from the start of the screen
            let appear = lb - (-1.0 + hash(n * 3.7) * 11.0);
            if appear < 0.0 {
                continue;
            }
            shown += 1;
            let dx = if i == 0 { drag } else { 0.0 };
            let x = lane_x + 20.0 + hash(n * 1.3 + i as f32) * (lane_w - 40.0) + dx;
            let s = snap(appear).min(1.2);
            // the title track is the client's target, every one of its keys moves by hand
            let fill = if i == 0 && lb > 4.0 {
                SELECTION
            } else {
                SECONDARY
            };
            keys.push(diamond(x, row_y(i) + 17.0, 6.0 * s, fill));
        }
    }

    let clips: Vec<Svgr> = TRACKS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let y = row_y(i);
            let cx = lane_x + 10.0 + hash(i as f32 * 5.1) * 300.0 + if i == 0 { drag } else { 0.0 };
            let cw = 500.0 + hash(i as f32 * 2.3) * 700.0;
            let fill = if i == 0 { SELECTION_SURFACE } else { PANEL };
            let stroke = if i == 0 { SELECTION } else { RULE };
            fframes::svgr!(
                <g>
                    <text x={x0 + 24.0} y={y + 23.0} font-family={MONO} font-weight="500" font-size="16" fill={SECONDARY}>{*name}</text>
                    <rect x={lane_x} y={y + 2.0} width={lane_w} height="30" fill={EMPHASIS} />
                    <rect x={cx} y={y + 4.0} width={cw} height="26" rx={RADIUS} fill={fill} stroke={stroke} stroke-width="1.5" />
                </g>
            )
        })
        .collect();

    let playhead = lane_x + ((lb + 4.0) * 0.045).fract() * lane_w;
    let ticks: Vec<Svgr> = (0..=24)
        .map(|k| {
            let x = lane_x + k as f32 * lane_w / 24.0;
            fframes::svgr!(<rect x={x} y={y0 + 278.0} width="1" height={if k % 4 == 0 { 14.0 } else { 7.0 }} fill={SECONDARY} />)
        })
        .collect();

    // the client changed one word of the title
    let word = if lb < 6.0 {
        "Launch day"
    } else {
        "Launch week"
    };
    let note = prog(lb, 3.0, 3.5);
    let cursor_x = lane_x + 10.0 + hash(0.0) * 300.0 + drag + 160.0;
    let count = format!("{shown} KEYFRAMES");

    fframes::svgr!(
        <g opacity={wo} transform={at(0.0, wd)}>
            {screen_title(lb, -3.5, "Hours of keyframes.")}
            {window(x0, y0, w, h, "Untitled Project – Edit")}
            // the program monitor
            <rect x={x0 + 560.0} y={y0 + 66.0} width="560" height="200" fill={EMPHASIS} />
            <text x={x0 + 840.0} y={y0 + 182.0} text-anchor="middle" font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="46" fill={TEXT}>{word}</text>
            <text x={x0 + 24.0} y={y0 + 90.0} font-family={MONO} font-weight="500" font-size="16" fill={SECONDARY}>"00:00:12:24"</text>
            <text x={x0 + w - 24.0} y={y0 + 90.0} text-anchor="end" font-family={MONO} font-weight="500" font-size="16" letter-spacing="2" fill={FAILURE}>{count}</text>
            {ticks}
            {clips}
            {keys}
            <rect x={playhead} y={y0 + 276.0} width="2" height="300" fill={TEXT} />
            <g opacity={note} transform={at(0.0, (1.0 - snap(lb - 3.0)) * 20.0)}>
                <rect x={x0 + 1250.0} y={y0 + 120.0} width="340" height="96" rx={RADIUS} fill={SELECTION_SURFACE} stroke={SELECTION} stroke-width="1.5" />
                <text x={x0 + 1272.0} y={y0 + 156.0} font-family={MONO} font-weight="500" font-size="16" letter-spacing="2" fill={SELECTION}>"CLIENT NOTE"</text>
                <text x={x0 + 1272.0} y={y0 + 190.0} font-family={SANS} font-weight="400" font-size="22" fill={TEXT}>"Change one word, please."</text>
            </g>
            {if lb > 3.6 { pointer(cursor_x, row_y(0) + 14.0) } else { Svgr::empty() }}
        </g>
    )
}

// ---------------------------------------------------------------------------
// the browser renderer: a page, a screenshot, the next page

fn browser(lb: f32) -> Svgr<'static> {
    let local = (lb - 16.0) * BEAT;
    let shots = (0..64).take_while(|k| shot_time(*k) <= local).count();
    let since_shot = if shots > 0 {
        local - shot_time(shots - 1)
    } else {
        9.0
    };
    let flash = (-since_shot * 9.0).exp();
    let frame_no = 96 + shots;
    let total = BROWSER_FRAMES as usize;
    let status = format!("RENDERING FRAME {frame_no} / {total}  ·  ONE SCREENSHOT PER FRAME");
    let progress = frame_no as f32 / total as f32;
    // the page being photographed: a title card whose circle moves a little per frame
    let t = frame_no as f32 * 0.08;
    let (px, py, pw, ph) = (150.0, 396.0, 940.0, 446.0);
    let page = fframes::svgr!(
        <g>
            <rect x={px} y={py} width={pw} height={ph} fill={EMPHASIS} />
            <circle cx={px + 300.0 + t.sin() * 120.0} cy={py + 230.0} r="110" fill={CANDIDATE} />
            <text x={px + 480.0} y={py + 220.0} font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="58" fill={TEXT}>"Launch day"</text>
            <text x={px + 482.0} y={py + 268.0} font-family={MONO} font-weight="400" font-size="22" fill={SECONDARY}>"localhost:3000"</text>
        </g>
    );

    // the screenshots taken so far, filed in a grid
    let thumbs: Vec<Svgr> = (0..shots.min(12))
        .map(|k| {
            let col = k % 3;
            let row = k / 3;
            let x = 1190.0 + col as f32 * 200.0;
            let y = 380.0 + row as f32 * 120.0;
            let arrive = snap((local - shot_time(k)) / BEAT);
            let tt = (96 + k + 1) as f32 * 0.08;
            fframes::svgr!(
                <g opacity={arrive} transform={at((1.0 - arrive) * -200.0, 0.0)}>
                    <rect x={x} y={y} width="180" height="100" fill={EMPHASIS} stroke={RULE} stroke-width="1" />
                    <circle cx={x + 60.0 + tt.sin() * 24.0} cy={y + 50.0} r="22" fill={CANDIDATE} />
                    <rect x={x + 96.0} y={y + 40.0} width="64" height="10" fill={TEXT} />
                    <text x={x} y={y + 118.0} font-family={MONO} font-weight="500" font-size="13" fill={SECONDARY}>{format!("frame_{:04}.png", 96 + k + 1)}</text>
                </g>
            )
        })
        .collect();

    let url = format!("localhost:3000/render?frame={frame_no}");
    fframes::svgr!(
        <g>
            {screen_title(lb, 16.0, "One screenshot at a time.")}
            {window(120.0, 300.0, 1000.0, 570.0, "Chromium – headless")}
            <rect x="150" y="354" width="940" height="30" rx={RADIUS} fill={EMPHASIS} />
            <text x="620" y="375" text-anchor="middle" font-family={MONO} font-weight="400" font-size="15" fill={SECONDARY}>{url}</text>
            {page}
            // the camera flash of the screenshot
            <rect x={px} y={py} width={pw} height={ph} fill={BG} opacity={flash * 0.9} />
            <text x="1190" y="350" font-family={MONO} font-weight="500" font-size="16" letter-spacing="2" fill={SECONDARY}>"SCREENSHOTS"</text>
            {thumbs}
            <text x="120" y="904" font-family={MONO} font-weight="500" font-size="18" letter-spacing="2" fill={FAILURE}>{status}</text>
            <rect x="120" y="918" width="1680" height="6" rx="3" fill={EMPHASIS} />
            <rect x="120" y="918" width={1680.0 * progress} height="6" rx="3" fill={FAILURE} />
            {detour(1800.0, 904.0)}
        </g>
    )
}

/// `HEADLESS BROWSER · SCREENSHOTS → PNG FILES → ENCODER`, right-aligned at
/// `right`, with the arrows drawn (the Latin subset of Geist Mono has none).
fn detour(right: f32, y: f32) -> Svgr<'static> {
    // Geist Mono advances 0.6 em per character, plus the letter spacing
    let advance = 15.0 * 0.6 + 1.0;
    let gap = 4.0 * advance;
    let parts = ["HEADLESS BROWSER · SCREENSHOTS", "PNG FILES", "ENCODER"];
    let width = parts
        .iter()
        .map(|p| p.chars().count() as f32 * advance)
        .sum::<f32>()
        + gap * 2.0;
    let mut x = right - width;
    let mut out = Vec::new();
    for (k, part) in parts.iter().enumerate() {
        if k > 0 {
            out.push(arrow(x + advance, x + gap - advance, y - 5.0, SECONDARY));
            x += gap;
        }
        out.push(fframes::svgr!(
            <text x={x} y={y} font-family={MONO} font-weight="400" font-size="15" letter-spacing="1" fill={SECONDARY}>{*part}</text>
        ));
        x += part.chars().count() as f32 * advance;
    }
    fframes::svgr!(<g>{out}</g>)
}

// ---------------------------------------------------------------------------
// the blind agent: it reports success on a broken frame

fn typed(text: &'static str, from: f32, lb: f32, per_beat: f32) -> &'static str {
    let n = ((lb - from) * per_beat).max(0.0) as usize;
    let end = text.char_indices().nth(n).map_or(text.len(), |(i, _)| i);
    &text[..end]
}

fn agent(lb: f32) -> Svgr<'static> {
    let ask = typed("Make a launch video for my app.", 32.5, lb, 14.0);
    let reply_at = 35.0;
    let reply = typed(
        "Done! I wrote the scenes and rendered launch.mp4.",
        reply_at,
        lb,
        16.0,
    );
    let rendered = prog(lb, 36.5, 37.0);
    let shake = if lb > 36.5 && lb < 37.5 {
        (hash(lb * 50.0) - 0.5) * 10.0 * (37.5 - lb)
    } else {
        0.0
    };

    // the frame the agent made, with what it cannot see
    let (fx, fy, fw, fh) = (840.0, 320.0, 960.0, 540.0);
    let problems: [(f32, f32, f32, f32, &str, f32); 3] = [
        (fx + 420.0, fy + 120.0, 540.0, 120.0, "TEXT CUT OFF", 39.0),
        (fx + 60.0, fy + 300.0, 360.0, 70.0, "MISSING FONT", 40.0),
        (fx + 60.0, fy + 430.0, 840.0, 70.0, "AUDIO CLIPPING", 41.0),
    ];
    let marks: Vec<Svgr> = problems
        .iter()
        .map(|(x, y, w, h, text, at_beat)| {
            let o = prog(lb, *at_beat, at_beat + 0.3);
            let s = 1.0 + (1.0 - snap(lb - at_beat)) * 0.08;
            fframes::svgr!(
                <g opacity={o} transform={zoom(x + w / 2.0, y + h / 2.0, s)}>
                    <rect x={*x} y={*y} width={*w} height={*h} fill="none" stroke={FAILURE} stroke-width="3" stroke-dasharray="10 6" />
                    <rect x={*x} y={y - 30.0} width={text.len() as f32 * 12.0 + 20.0} height="28" fill={FAILURE} />
                    <text x={x + 10.0} y={y - 10.0} font-family={MONO} font-weight="500" font-size="16" letter-spacing="1.5" fill={ON_ACTION}>{*text}</text>
                </g>
            )
        })
        .collect();
    let meter: Vec<Svgr> = (0..48)
        .map(|k| {
            let v = (hash(k as f32 * 2.7) * 0.7 + 0.35).min(1.0);
            let hh = v * 54.0;
            let fill = if v > 0.92 { FAILURE } else { INVERSE_SECONDARY };
            fframes::svgr!(<rect x={fx + 70.0 + k as f32 * 17.0} y={fy + 492.0 - hh} width="10" height={hh} fill={fill} />)
        })
        .collect();

    fframes::svgr!(
        <g>
            {screen_title(lb, 32.0, "The agent can’t see the video.")}
            // the chat
            {panel(120.0, 320.0, 680.0, 540.0)}
            <text x="148" y="364" font-family={MONO} font-weight="500" font-size="16" letter-spacing="2" fill={SECONDARY}>"AGENT"</text>
            // the user's request is the selection pink of supplied context
            <rect x="260" y="396" width="510" height="84" rx={RADIUS} fill={SELECTION_SURFACE} />
            <text x="284" y="446" font-family={SANS} font-weight="400" font-size="24" fill={TEXT}>{ask}</text>
            <g opacity={prog(lb, reply_at - 0.3, reply_at)}>
                <rect x="150" y="510" width="560" height="120" rx={RADIUS} fill="none" stroke={RULE} stroke-width="1.5" />
                <text x="174" y="556" font-family={SANS} font-weight="400" font-size="24" fill={TEXT}>{typed("Done! I wrote the scenes", reply_at, lb, 16.0)}</text>
                <text x="174" y="592" font-family={SANS} font-weight="400" font-size="24" fill={TEXT}>{if reply.len() > 24 { reply[24..].trim_start() } else { "" }}</text>
            </g>
            // what the agent claims: a file, not a checked video
            <g opacity={rendered}>
                <rect x="150" y="660" width="300" height="56" rx={RADIUS} fill={ACCEPTED_SURFACE} />
                {check(172.0, 690.0, 18.0, ACCEPTED)}
                <text x="204" y="696" font-family={MONO} font-weight="500" font-size="19" fill={ACCEPTED}>"launch.mp4"</text>
            </g>
            // the frame it rendered
            <g opacity={rendered} transform={at(shake, 0.0)}>
                <clipPath id="agent-frame"><rect x={fx} y={fy} width={fw} height={fh} /></clipPath>
                <rect x={fx} y={fy} width={fw} height={fh} rx={RADIUS} fill={INVERSE} />
                <g clip-path="url(#agent-frame)">
                    <text x={fx + 60.0} y={fy + 210.0} font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="112" letter-spacing="-4" fill={INVERSE_TEXT}>"LAUNCH DAY IS HERE"</text>
                    <rect x={fx + 70.0} y={fy + 312.0} width="40" height="46" fill="none" stroke={INVERSE_SECONDARY} stroke-width="3" />
                    <rect x={fx + 122.0} y={fy + 312.0} width="40" height="46" fill="none" stroke={INVERSE_SECONDARY} stroke-width="3" />
                    <rect x={fx + 174.0} y={fy + 312.0} width="40" height="46" fill="none" stroke={INVERSE_SECONDARY} stroke-width="3" />
                    <rect x={fx + 250.0} y={fy + 312.0} width="40" height="46" fill="none" stroke={INVERSE_SECONDARY} stroke-width="3" />
                    <rect x={fx + 302.0} y={fy + 312.0} width="40" height="46" fill="none" stroke={INVERSE_SECONDARY} stroke-width="3" />
                    {meter}
                </g>
                {marks}
            </g>
        </g>
    )
}
