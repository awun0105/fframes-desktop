//! Act 3: from idea to finished video in four steps, along a path of nodes:
//! click New project in the studio, let the agent check the video through the CLI tools (their
//! real output on `examples/marketing`), scrub it in the native player, export it.

use std::sync::LazyLock;

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};
use fframes_components::components::typing_code_block::{TypingCodeBlock, TypingCodeBlockConfig};

use super::comp;
use super::look::*;
use crate::beat::*;
use crate::facts::BENCH_FFRAMES_S;

beat_scene!(WorkflowScene, Some(80.0), Some(128.0));

/// Beats (inside the act) each step starts on.
const STEPS: [f32; 4] = [0.0, 10.0, 30.0, 40.0];
const STEP_NAMES: [&str; 4] = ["START", "CHECK", "SCRUB", "EXPORT"];
/// Beat the tools show what they found: the drop.
const DROP: f32 = 16.0;
/// Beat the cut-off text is fixed and the check passes.
const FIXED: f32 = 23.0;

/// Video beats of the act's sound cues.
pub const CLICK_BEAT: f32 = 80.0 + CLICK;
pub const TYPE_BEAT: f32 = 84.5;
pub const BLIP_BEATS: [f32; 6] = [96.0, 96.5, 97.0, 97.5, 103.0, 121.4];

static CODE: LazyLock<TypingCodeBlock> = LazyLock::new(|| {
    TypingCodeBlock::new(TypingCodeBlockConfig {
        code: "impl Video for Launch {\n    const FPS: usize = 60;\n\n    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {\n        let y = frame.animate(&timeline!(\n            at 0.0, animate 80.0 => 0.0, Easing::Spring { .. },\n        ));\n        svgr!(<text y={540.0 + y}>\"Launch day\"</text>)\n    }\n}".to_owned(),
        language: "rust".to_owned(),
        typing_speed: 0.22,
        background: INVERSE.to_owned(),
        font_size: Some(17.0),
        ..Default::default()
    })
});

fn typed(text: &'static str, from: f32, lb: f32, per_beat: f32) -> &'static str {
    let n = ((lb - from) * per_beat).max(0.0) as usize;
    let end = text.char_indices().nth(n).map_or(text.len(), |(i, _)| i);
    &text[..end]
}

impl Scene for WorkflowScene {
    fn duration(&self) -> Duration<'_> {
        Self::frames()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let lb = Self::lb(&frame);
        let step = STEPS.iter().rposition(|s| lb >= *s).unwrap_or(0);
        let (eo, ed) = rise(lb, 0.0, 24.0);
        let titles = [
            "Start in one click.",
            "Give your agent eyes and ears.",
            "Scrub it on the desktop.",
            "Export the MP4.",
        ];
        let headlines: Vec<Svgr> = titles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let from = STEPS[i];
                let until = STEPS.get(i + 1).copied().unwrap_or(48.0) - 0.4;
                if lb < from || lb > until + 0.5 {
                    return Svgr::empty();
                }
                let (o, d) = rise(lb, from + 0.2, 40.0);
                fframes::svgr!(<g opacity={o * leave(lb, until)} transform={at(0.0, d)}>{headline(116.0, 252.0, *t, 72.0, TEXT)}</g>)
            })
            .collect();

        let body = match step {
            0 => start(&frame, lb),
            1 => check(ctx, lb),
            2 => scrub(&frame, ctx, lb),
            _ => export(lb),
        };

        fframes::svgr!(
            <g opacity={leave(lb, 47.4)}>
                <g opacity={eo} transform={at(0.0, ed)}>{act_eyebrow("03", "THE WORKFLOW")}</g>
                {headlines}
                {path(lb)}
                {body}
                {caption(lb, 1.0, 9.4, "Step 1:", "click New project, the studio sets up scenes and a timeline.", ACTION)}
                {caption(lb, 11.0, 29.4, "Step 2:", "agents can’t watch video, so fframes turns it into images and numbers.", ACTION)}
                {caption(lb, 31.0, 39.4, "Step 3:", "scrub every frame in the native player, in real time.", ACTION)}
                {caption(lb, 41.0, 47.0, "Step 4:", "one click on Export, frames go straight into the encoder.", ACTION)}
            </g>
        )
    }
}

/// The four steps as nodes on a line, top right; the line draws on as the act moves on.
fn path(lb: f32) -> Svgr<'static> {
    let x0 = 1180.0;
    let gap = 200.0;
    let y = 150.0;
    let reached = STEPS.iter().filter(|s| lb >= **s).count();
    let drawn: f32 = (1..4).map(|i| prog(lb, STEPS[i] - 0.6, STEPS[i])).sum();
    let nodes: Vec<Svgr> = (0..4)
        .map(|i| {
            let x = x0 + i as f32 * gap;
            let on = i < reached;
            let current = i + 1 == reached;
            let pop = if on { snap(lb - STEPS[i]) } else { 1.0 };
            let r = if current { 11.0 * pop } else { 8.0 };
            let fill = if on { ACTION } else { BG };
            let stroke = if on { ACTION } else { CONTROL };
            let label_fill = if current { TEXT } else { SECONDARY };
            let number = format!("{}", i + 1);
            fframes::svgr!(
                <g>
                    <circle cx={x} cy={y} r={r.max(8.0)} fill={fill} stroke={stroke} stroke-width="2" />
                    <text x={x} y={y + 5.5} text-anchor="middle" font-family={MONO} font-weight="500" font-size="14" fill={if on { ON_ACTION } else { SECONDARY }}>{number}</text>
                    <text x={x} y={y + 44.0} text-anchor="middle" font-family={MONO} font-weight="500" font-size="16" letter-spacing="3" fill={label_fill}>{STEP_NAMES[i]}</text>
                </g>
            )
        })
        .collect();
    let (o, _) = rise(lb, 0.5, 0.0);
    fframes::svgr!(
        <g opacity={o}>
            <rect x={x0} y={y - 1.0} width={gap * 3.0} height="2" fill={RULE} />
            <rect x={x0} y={y - 1.0} width={(gap * drawn).max(0.5)} height="2" fill={ACTION} />
            {nodes}
        </g>
    )
}

// ---------------------------------------------------------------------------
// step 1: one click on New project in the studio, and the scene the agent writes

/// Beat (inside the act) the New project button is clicked.
const CLICK: f32 = 2.2;

/// The new project's clips and their seconds, as the studio creates them.
const CLIPS: [(&str, f32); 3] = [("Intro", 2.0), ("Story", 2.0), ("Outro", 1.0)];

fn start(frame: &Frame, lb: f32) -> Svgr<'static> {
    let (o, d) = rise(lb, 0.3, 60.0);
    let (x, y, w, h) = (120.0, 320.0, 780.0, 520.0);
    let opened = prog(lb, CLICK + 0.2, CLICK + 0.6);
    let title = if lb < CLICK + 0.4 {
        "FFrames Studio"
    } else {
        "FFrames Studio — launch-video"
    };

    // the home screen, with the button the cursor goes for
    let press = 1.0 - 0.05 * (1.0 - snap(lb - CLICK)) * f32::from(lb >= CLICK);
    let (bx, by, bw, bh) = (x + 40.0, y + 190.0, 260.0, 64.0);
    let home = fframes::svgr!(
        <g opacity={1.0 - opened}>
            <text x={x + 40.0} y={y + 112.0} font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="38" letter-spacing="-1.5" fill={TEXT}>"Welcome to FFrames Studio"</text>
            <text x={x + 40.0} y={y + 150.0} font-family={SANS} font-weight="400" font-size="22" fill={SECONDARY}>"Start a video, or open one you made."</text>
            <g transform={zoom(bx + bw / 2.0, by + bh / 2.0, press)}>
                <rect x={bx} y={by} width={bw} height={bh} rx={RADIUS} fill={ACTION} />
                <text x={bx + bw / 2.0} y={by + 40.0} text-anchor="middle" font-family={SANS} font-weight="500" font-size="23" fill={ON_ACTION}>"+  New project"</text>
            </g>
            <rect x={bx + bw + 20.0} y={by} width="220" height={bh} rx={RADIUS} fill="none" stroke={CONTROL} stroke-width="1.5" />
            <text x={bx + bw + 130.0} y={by + 40.0} text-anchor="middle" font-family={SANS} font-weight="500" font-size="23" fill={TEXT}>"Open folder"</text>
            <text x={x + 40.0} y={y + 330.0} font-family={MONO} font-weight="500" font-size="15" letter-spacing="3" fill={SECONDARY}>"RECENT"</text>
            <rect x={x + 40.0} y={y + 350.0} width={w - 80.0} height="1.5" fill={RULE} />
            <text x={x + 40.0} y={y + 392.0} font-family={SANS} font-weight="400" font-size="20" fill={SECONDARY}>"No projects yet."</text>
        </g>
    );

    // the project it opens: a preview and the three clips on the timeline
    let total: f32 = CLIPS.iter().map(|(_, s)| s).sum();
    let (tx, tw) = (x + 30.0, w - 60.0);
    let mut cx = tx;
    let clips: Vec<Svgr> = CLIPS
        .iter()
        .enumerate()
        .map(|(i, (name, seconds))| {
            let cw = tw * seconds / total - 6.0;
            let land = CLICK + 0.5 + i as f32 * 0.25;
            let co = prog(lb, land, land + 0.3);
            let clip = fframes::svgr!(
                <g opacity={co}>
                    <rect x={cx} y={y + 400.0} width={cw} height="64" rx={RADIUS} fill={if i == 0 { SELECTION_SURFACE } else { PANEL }} stroke={if i == 0 { SELECTION } else { RULE }} stroke-width="1.5" />
                    <text x={cx + 16.0} y={y + 439.0} font-family={SANS} font-weight="500" font-size="20" fill={TEXT}>{*name}</text>
                    <text x={cx + cw - 14.0} y={y + 439.0} text-anchor="end" font-family={MONO} font-weight="400" font-size="16" fill={SECONDARY}>{format!("{seconds:.0}s")}</text>
                </g>
            );
            cx += cw + 6.0;
            clip
        })
        .collect();
    let head = tx + tw * prog(lb, CLICK + 1.5, 9.5);
    let project = fframes::svgr!(
        <g opacity={opened}>
            <rect x={x + 30.0} y={y + 70.0} width={w - 60.0} height="300" rx={RADIUS} fill={EMPHASIS} />
            <text x={x + w / 2.0} y={y + 236.0} text-anchor="middle" font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="58" letter-spacing="-2.3" fill={TEXT}>"Launch day"</text>
            <text x={x + 30.0} y={y + 388.0} font-family={MONO} font-weight="500" font-size="14" letter-spacing="3" fill={SECONDARY}>"TIMELINE"</text>
            {clips}
            <rect x={head} y={y + 392.0} width="3" height="80" fill={TEXT} opacity={prog(lb, CLICK + 1.2, CLICK + 1.5)} />
        </g>
    );

    // the cursor glides to the button and clicks
    let glide = cubic_in_out(prog(lb, 0.6, CLICK - 0.2));
    let (px, py) = (
        lerp(x + w - 120.0, bx + bw * 0.6, glide),
        lerp(y + h - 60.0, by + bh * 0.62, glide),
    );
    let ring = prog(lb, CLICK, CLICK + 0.6);
    let cursor_o = 1.0 - prog(lb, CLICK + 0.6, CLICK + 0.9);
    let cursor = fframes::svgr!(
        <g opacity={cursor_o}>
            <circle cx={px} cy={py} r={8.0 + 30.0 * ring} fill="none" stroke={ACTION} stroke-width="2" opacity={if ring > 0.0 { 1.0 - ring } else { 0.0 }} />
            <path d={format!("M{px} {py} l0 26 l7 -6 l5 11 l5 -2 l-5 -11 l9 0 z")} fill={TEXT} stroke={BG} stroke-width="1.5" />
        </g>
    );

    let code = comp::show(
        &*CODE,
        frame,
        (lb - 4.5) * BEAT,
        (930.0, 300.0, 870.0, 560.0),
    );
    fframes::svgr!(
        <g opacity={o * leave(lb, 9.6)} transform={at(0.0, d)}>
            {window(x, y, w, h, title)}
            {home}
            {project}
            {cursor}
            <g opacity={prog(lb, 4.3, 4.6)}>{code}</g>
        </g>
    )
}

// ---------------------------------------------------------------------------
// step 2: the four tools an agent reads instead of watching

const TOOLS: [(&str, &str); 4] = [
    ("inspect --every 0.25s", "PROBLEMS IN EVERY FRAME"),
    ("strip -n 12", "THE WHOLE STORY, ONE IMAGE"),
    ("onion 5.2s..6.4s -n 6", "THE PATH OF A MOVEMENT"),
    ("audio analyze --waveform", "LEVELS, PEAKS, SILENCE"),
];

fn check(ctx: &FFramesContext<'_, '_>, lb: f32) -> Svgr<'static> {
    let (pw, ph) = (820.0, 300.0);
    let panels: Vec<Svgr> = TOOLS
        .iter()
        .enumerate()
        .map(|(i, (cmd, what))| {
            let x = 120.0 + (i % 2) as f32 * (pw + 40.0);
            let y = 300.0 + (i / 2) as f32 * (ph + 30.0);
            let at_beat = 11.0 + i as f32;
            let (o, d) = rise(lb, at_beat, 40.0);
            let shown = DROP + i as f32 * 0.5;
            let pop = prog(lb, shown, shown + 0.3);
            let s = 0.96 + 0.04 * snap(lb - shown);
            let cmd = typed(cmd, at_beat + 0.2, lb, 30.0);
            let content = match i {
                0 => inspect(ctx, lb, x, y),
                1 => picture(ctx, "strip.jpg", (x + 24.0, y + 64.0, 500.0, 214.0), "12 FRAMES OF", "examples/marketing", y),
                2 => picture(ctx, "onion.jpg", (x + 24.0, y + 64.0, 380.0, 214.0), "6 FRAMES BLENDED", "bars rising, 5.2s..6.4s", y),
                _ => audio(ctx, x, y),
            };
            fframes::svgr!(
                <g opacity={o} transform={at(0.0, d)}>
                    {panel(x, y, pw, ph)}
                    <text x={x + 24.0} y={y + 40.0} font-family={MONO} font-weight="500" font-size="19" fill={ACTION}>"$"<tspan fill={TEXT}>" "{cmd}</tspan></text>
                    <text x={x + pw - 24.0} y={y + 40.0} text-anchor="end" font-family={MONO} font-weight="500" font-size="14" letter-spacing="2" fill={SECONDARY}>{*what}</text>
                    <g opacity={pop} transform={zoom(x + pw / 2.0, y + ph / 2.0, s)}>{content}</g>
                </g>
            )
        })
        .collect();
    fframes::svgr!(<g opacity={leave(lb, 29.6)}>{panels}</g>)
}

/// A real output image of a tool with two lines of what it shows.
fn picture(
    ctx: &FFramesContext<'_, '_>,
    name: &str,
    rect: (f32, f32, f32, f32),
    line1: &'static str,
    line2: &'static str,
    y: f32,
) -> Svgr<'static> {
    let id = format!("tool-{}", name.replace('.', "-"));
    let tx = rect.0 + rect.2 + 28.0;
    fframes::svgr!(
        <g>
            {rounded(&id, rect, RADIUS, still(ctx, name, rect))}
            <text x={tx} y={y + 120.0} font-family={MONO} font-weight="500" font-size="15" letter-spacing="2" fill={SECONDARY}>{line1}</text>
            <text x={tx} y={y + 150.0} font-family={SANS} font-weight="400" font-size="20" fill={TEXT}>{line2}</text>
        </g>
    )
}

/// `inspect` finds text cut off by the canvas edge, the agent moves it, the check passes.
fn inspect(ctx: &FFramesContext<'_, '_>, lb: f32, x: f32, y: f32) -> Svgr<'static> {
    let fix = cubic_in_out(prog(lb, FIXED - 1.0, FIXED));
    let ok = lb >= FIXED;
    let (fx, fy, fw, fh) = (x + 24.0, y + 64.0, 380.0, 214.0);
    let text_x = lerp(fx + 150.0, fx + 40.0, fix);
    let color = if ok { ACCEPTED } else { FAILURE };
    let box_w = if ok { 270.0 } else { fx + fw - text_x + 10.0 };
    let line1 = if ok {
        "checked 121 frames: 0 findings"
    } else {
        "warning: text \"Launch day\""
    };
    let line2 = if ok {
        "exit code 0"
    } else {
        "is cut off by the canvas edge"
    };
    fframes::svgr!(
        <g>
            <clipPath id="inspect-frame"><rect x={fx} y={fy} width={fw} height={fh} rx={RADIUS} /></clipPath>
            <g clip-path="url(#inspect-frame)">
                {still(ctx, "marketing_poster.jpg", (fx, fy, fw, fh))}
                <rect x={fx} y={fy} width={fw} height={fh} fill={BG} opacity="0.55" />
                <text x={text_x} y={fy + 120.0} font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="56" letter-spacing="-1" fill={TEXT}>"Launch day"</text>
            </g>
            <rect x={text_x - 10.0} y={fy + 66.0} width={box_w} height="74" fill="none" stroke={color} stroke-width="2.5" stroke-dasharray="8 5" />
            <text x={fx + fw + 28.0} y={y + 120.0} font-family={MONO} font-weight="500" font-size="17" fill={color}>{line1}</text>
            <text x={fx + fw + 28.0} y={y + 148.0} font-family={MONO} font-weight="400" font-size="17" fill={color}>{line2}</text>
        </g>
    )
}

/// The waveform and loudness report of `audio analyze`, as it printed them.
fn audio(ctx: &FFramesContext<'_, '_>, x: f32, y: f32) -> Svgr<'static> {
    let rect = (x + 24.0, y + 64.0, 772.0, 170.0);
    fframes::svgr!(
        <g>
            {rounded("tool-waveform", rect, RADIUS, still(ctx, "waveform.jpg", rect))}
            <text x={x + 24.0} y={y + 272.0} font-family={MONO} font-weight="400" font-size="17" fill={SECONDARY}>"integrated -16.0 LUFS · true peak -0.9 dBTP · clipped samples 0"</text>
        </g>
    )
}

// ---------------------------------------------------------------------------
// step 3: the native player, scrubbed back and forth

/// Seconds into the capture the playhead shows at a beat of the act.
fn scrub_position(lb: f32) -> f32 {
    let t = lb - STEPS[2];
    if t < 2.5 {
        // it plays
        1.0 + t * BEAT
    } else {
        // then the playhead is dragged back and forth across the scenes
        let u = t - 2.5;
        let base = 1.0 + 2.5 * BEAT;
        (base + (u * 0.9).sin() * 2.6 + u * 0.35).max(0.0)
    }
}

fn scrub(frame: &Frame, ctx: &FFramesContext<'_, '_>, lb: f32) -> Svgr<'static> {
    let (o, d) = rise(lb, STEPS[2] + 0.2, 60.0);
    let pos = scrub_position(lb);
    let local = frame.seconds();
    let (x, y, w) = (470.0, 292.0, 980.0);
    let h = w * 896.0 / 1600.0;
    let player = clip(
        frame,
        ctx,
        "preview_capture.mp4",
        "preview_poster.jpg",
        local - pos,
        (x, y, w, h),
    );
    // our own scrub bar under the window, with the cursor holding the playhead
    let bar_y = y + h + 30.0;
    let length = 11.4;
    let px = x + (pos / length).clamp(0.0, 1.0) * w;
    let dragging = lb - STEPS[2] > 2.5;
    let cursor = if dragging {
        pointer(px - 4.0, bar_y + 4.0)
    } else {
        Svgr::empty()
    };
    let tc = crate::ui::timecode(pos);
    fframes::svgr!(
        <g opacity={o * leave(lb, 39.6)} transform={at(0.0, d)}>
            {rounded("scrub-player", (x, y, w, h), RADIUS, player)}
            <rect x={x} y={y} width={w} height={h} rx={RADIUS} fill="none" stroke={RULE} stroke-width={RULE_WIDTH} />
            <rect x={x} y={bar_y - 2.0} width={w} height="4" rx="2" fill={EMPHASIS} />
            <rect x={x} y={bar_y - 2.0} width={(px - x).max(0.5)} height="4" rx="2" fill={ACTION} />
            <circle cx={px} cy={bar_y} r="9" fill={ACTION} />
            <text x={x + w} y={bar_y - 16.0} text-anchor="end" font-family={MONO} font-weight="400" font-size="18" fill={SECONDARY}>{tc}</text>
            {cursor}
        </g>
    )
}

// ---------------------------------------------------------------------------
// step 4: render

fn export(lb: f32) -> Svgr<'static> {
    let (o, d) = rise(lb, STEPS[3] + 0.2, 60.0);
    let (x, y, w, h) = (320.0, 330.0, 1280.0, 400.0);
    // the cursor clicks Export MP4, then the frames are rendered and encoded
    let click = STEPS[3] + 1.4;
    let (bx, by, bw, bh) = (x + w - 280.0, y + 70.0, 240.0, 56.0);
    let press = 1.0 - 0.05 * (1.0 - snap(lb - click)) * f32::from(lb >= click);
    let glide = cubic_in_out(prog(lb, STEPS[3] + 0.4, click - 0.2));
    let (px, py) = (
        lerp(x + w / 2.0, bx + bw * 0.55, glide),
        lerp(y + h - 40.0, by + bh * 0.6, glide),
    );
    let ring = prog(lb, click, click + 0.6);
    let go = click + 0.3;
    let p = expo_out(prog(lb, go, go + 2.2));
    let frames = (p * 300.0).round() as u32;
    let elapsed = p * BENCH_FFRAMES_S;
    let done = p >= 0.999;
    let status = format!("{frames} / 300 frames   {elapsed:.2} s");
    let file = prog(lb, go + 2.4, go + 2.8);
    let fs = snap(lb - go - 2.4);
    fframes::svgr!(
        <g opacity={o} transform={at(0.0, d)}>
            {window(x, y, w, h, "FFrames Studio — launch-video")}
            <text x={x + 40.0} y={y + 108.0} font-family={MONO} font-weight="500" font-size="16" letter-spacing="3" fill={SECONDARY}>"EXPORT"</text>
            <text x={x + 40.0} y={y + 140.0} font-family={MONO} font-weight="400" font-size="20" fill={SECONDARY}>"launch.mp4 · 1920×1080 · 60 fps · H.264"</text>
            <g transform={zoom(bx + bw / 2.0, by + bh / 2.0, press)}>
                <rect x={bx} y={by} width={bw} height={bh} rx={RADIUS} fill={ACTION} />
                <text x={bx + bw / 2.0} y={by + 36.0} text-anchor="middle" font-family={SANS} font-weight="500" font-size="23" fill={ON_ACTION}>"Export MP4"</text>
            </g>
            // the render is a candidate until the file is written
            <g opacity={prog(lb, go - 0.2, go)}>
                <rect x={x + 40.0} y={y + 200.0} width={w - 80.0} height="12" rx="6" fill={EMPHASIS} />
                <rect x={x + 40.0} y={y + 200.0} width={((w - 80.0) * p).max(0.5)} height="12" rx="6" fill={if done { ACCEPTED } else { CANDIDATE }} />
                <text x={x + 40.0} y={y + 266.0} font-family={MONO} font-weight="500" font-size="22" fill={TEXT}>{status}</text>
            </g>
            <g opacity={1.0 - prog(lb, click + 0.6, click + 0.9)}>
                <circle cx={px} cy={py} r={8.0 + 30.0 * ring} fill="none" stroke={ACTION} stroke-width="2" opacity={if ring > 0.0 { 1.0 - ring } else { 0.0 }} />
                <path d={format!("M{px} {py} l0 26 l7 -6 l5 11 l5 -2 l-5 -11 l9 0 z")} fill={TEXT} stroke={BG} stroke-width="1.5" />
            </g>
            <g opacity={file} transform={zoom(960.0, 650.0, 0.9 + 0.1 * fs)}>
                <rect x="790" y="610" width="340" height="80" rx={RADIUS} fill={ACCEPTED_SURFACE} />
                {super::look::check(866.0, 650.0, 24.0, ACCEPTED)}
                <text x="906" y="660" font-family={MONO} font-weight="500" font-size="26" fill={ACCEPTED}>"launch.mp4"</text>

            </g>
        </g>
    )
}
