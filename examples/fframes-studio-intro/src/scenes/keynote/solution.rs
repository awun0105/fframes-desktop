//! Act 2: meet the studio. The name settles on the drop, a prompt types
//! itself and the view cuts into it, the desktop window opens on the real
//! native player, and three cards say what it does.

use std::sync::LazyLock;

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};
use fframes_components::components::showcase::prompt_zoom::{PromptZoom, PromptZoomConfig};
use fframes_components::components::tracking_in::{TrackingIn, TrackingInConfig};
use fframes_components::ui::{Mode, UiTheme};

use super::comp;
use super::look::*;
use crate::beat::*;

beat_scene!(SolutionScene, Some(48.0), Some(80.0));

/// Beat (inside the act) the prompt appears.
const PROMPT_AT: f32 = 4.0;
/// Beat the window opens on the native player.
const WINDOW_AT: f32 = 10.0;
/// Beat the window makes room for the cards.
const CARDS_AT: f32 = 16.0;

/// The prompt typed into the agent.
const PROMPT: &str = "A 30-second launch video on a 132 BPM beat";

/// Video beats of the act's sound cues: typing starts, the prompt is sent.
pub const TYPE_BEAT: f32 = 48.0 + PROMPT_AT + 1.2;
pub const SEND_BEAT: f32 = 48.0 + WINDOW_AT;
/// Video beats the cards land on.
pub const CARD_BEATS: [f32; 3] = [66.0, 68.0, 70.0];

static TITLE: LazyLock<TrackingIn> = LazyLock::new(|| {
    TrackingIn::new(TrackingInConfig {
        text: "FFrames Studio".to_owned(),
        font_size: 116.0,
        color: TEXT.to_owned(),
        font_weight: DISPLAY_WEIGHT,
        start_tracking: 0.16,
        ..Default::default()
    })
});

static PROMPT_ZOOM: LazyLock<PromptZoom> = LazyLock::new(|| {
    PromptZoom::new(PromptZoomConfig {
        greeting: "What are we making?".to_owned(),
        text: PROMPT.to_owned(),
        placeholder: "Describe your video".to_owned(),
        model: "Codex".to_owned(),
        effort: "High".to_owned(),
        type_start: 1.2 * BEAT,
        cut_at: 1.4 * BEAT,
        chars_per_second: 22.0,
        zoom: 1.55,
        focus_x: 0.5,
        focus_y: 0.47,
        accent_color: Some(ACTION.to_owned()),
        mode: Mode::Light,
        theme: Some(theme()),
        font_family: Some(SANS.to_owned()),
        duration: 300,
        ..Default::default()
    })
});

/// The components' light palette on the tokens of this cut.
fn theme() -> UiTheme {
    UiTheme {
        background: BG.to_owned(),
        foreground: TEXT.to_owned(),
        card: PANEL.to_owned(),
        card_foreground: TEXT.to_owned(),
        popover: PANEL.to_owned(),
        popover_foreground: TEXT.to_owned(),
        primary: ACTION.to_owned(),
        primary_foreground: ON_ACTION.to_owned(),
        secondary: EMPHASIS.to_owned(),
        secondary_foreground: TEXT.to_owned(),
        muted: EMPHASIS.to_owned(),
        muted_foreground: SECONDARY.to_owned(),
        accent: EMPHASIS.to_owned(),
        accent_foreground: TEXT.to_owned(),
        border: RULE.to_owned(),
        input: CONTROL.to_owned(),
        ring: ACTION.to_owned(),
        radius: RADIUS,
        font_family: Some(SANS.to_owned()),
        ..UiTheme::light()
    }
}

struct Card {
    title: &'static str,
    lines: [&'static str; 2],
}

const CARDS: [Card; 3] = [
    Card {
        title: "Write or prompt",
        lines: ["Scenes are plain Rust,", "or ask Codex or Claude Code."],
    },
    Card {
        title: "See it instantly",
        lines: ["A native preview at 60 fps", "with a timeline to scrub."],
    },
    Card {
        title: "Render in seconds",
        lines: ["Skia draws on the GPU and", "frames go straight to MP4."],
    },
];

impl Scene for SolutionScene {
    fn duration(&self) -> Duration<'_> {
        Self::frames()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let lb = Self::lb(&frame);

        // the name, on the drop
        let title = if lb < PROMPT_AT + 0.5 {
            let (so, sd) = rise(lb, 1.5, 20.0);
            fframes::svgr!(
                <g opacity={leave(lb, PROMPT_AT - 0.5)}>
                    <g transform={at(0.0, -40.0)}><text x="960" y="380" text-anchor="middle" font-family={MONO} font-weight="500" font-size="22" letter-spacing="3" fill={ACTION}>"02"<tspan fill={SECONDARY}>" / THE SOLUTION"</tspan></text></g>
                    {comp::show(&*TITLE, &frame, lb * BEAT, (0.0, -110.0, 1920.0, 1080.0))}
                    <g opacity={so} transform={at(0.0, sd)}>
                        <text x="960" y="680" text-anchor="middle" font-family={SANS} font-weight="400" font-size="34" letter-spacing="-0.5" fill={SECONDARY}>"Programmatic video, with a desktop studio."</text>
                    </g>
                </g>
            )
        } else {
            Svgr::empty()
        };

        // the prompt, typed and cut into
        let prompt = if (PROMPT_AT..WINDOW_AT + 0.5).contains(&lb) {
            let o = prog(lb, PROMPT_AT, PROMPT_AT + 0.4) * leave(lb, WINDOW_AT - 0.3);
            fframes::svgr!(
                <g opacity={o}>{comp::show(&*PROMPT_ZOOM, &frame, (lb - PROMPT_AT) * BEAT, (0.0, 0.0, 1920.0, 1080.0))}</g>
            )
        } else {
            Svgr::empty()
        };

        // the window springs open on the native player, then moves up for the cards
        let window = if lb >= WINDOW_AT - 0.3 {
            let open = soft(lb - WINDOW_AT + 0.3);
            let up = cubic_in_out(prog(lb, CARDS_AT, CARDS_AT + 1.2));
            let w = lerp(1280.0, 960.0, up);
            let h = w * 896.0 / 1600.0 + 44.0;
            let x = 960.0 - w / 2.0;
            let y = lerp(130.0, 110.0, up);
            let s = 0.6 + 0.4 * open;
            let start = (WINDOW_AT - 0.3) * BEAT;
            let player = clip(
                &frame,
                ctx,
                "preview_capture.mp4",
                "preview_poster.jpg",
                start,
                (x, y + 44.0, w, h - 44.0),
            );
            fframes::svgr!(
                <g opacity={prog(lb, WINDOW_AT - 0.3, WINDOW_AT)} transform={zoom(960.0, y + h / 2.0, s)}>
                    {window(x, y, w, h, "FFrames Studio — launch-video")}
                    {rounded("solution-player", (x + 2.0, y + 44.0, w - 4.0, h - 46.0), RADIUS, player)}
                </g>
            )
        } else {
            Svgr::empty()
        };

        let cards: Vec<Svgr> = CARDS
            .iter()
            .enumerate()
            .map(|(i, c)| card(lb, i, c))
            .collect();

        fframes::svgr!(
            <g>
                {title}
                {prompt}
                <g opacity={leave(lb, 31.4)}>
                    {window}
                    {cards}
                </g>
                {caption(lb, WINDOW_AT + 1.0, CARDS_AT - 0.5, "FFrames Studio:", "describe the video, watch it play, export it.", ACTION)}
            </g>
        )
    }
}

fn card(lb: f32, i: usize, c: &Card) -> Svgr<'static> {
    let land = CARD_BEATS[i] - 48.0;
    if lb < land - 0.2 {
        return Svgr::empty();
    }
    // the storyboard's spring: stiffness 220, damping 18, from 0.85 to 1
    let s = 0.85 + 0.15 * spring((lb - land + 0.2) * BEAT, 220.0, 18.0);
    let o = prog(lb, land - 0.2, land + 0.2);
    let (w, h) = (520.0, 220.0);
    let x = 140.0 + i as f32 * (w + 40.0);
    let y = 748.0;
    let trace = prog(lb, land, land + 1.5);
    let number = format!("0{}", i + 1);
    fframes::svgr!(
        <g opacity={o} transform={zoom(x + w / 2.0, y + h / 2.0, s)}>
            {panel(x, y, w, h)}
            <rect x={x} y={y} width={(w * trace).max(0.5)} height="2" fill={ACTION} opacity={if trace > 0.0 { 1.0 } else { 0.0 }} />
            <text x={x + 32.0} y={y + 52.0} font-family={MONO} font-weight="500" font-size="20" letter-spacing="3" fill={ACTION}>{number}</text>
            <text x={x + 32.0} y={y + 112.0} font-family={SANS} font-weight={DISPLAY_WEIGHT} font-size="42" letter-spacing="-1.6" fill={TEXT}>{c.title}</text>
            <text x={x + 32.0} y={y + 160.0} font-family={SANS} font-weight="400" font-size="24" fill={SECONDARY}>{c.lines[0]}</text>
            <text x={x + 32.0} y={y + 192.0} font-family={SANS} font-weight="400" font-size="24" fill={SECONDARY}>{c.lines[1]}</text>
        </g>
    )
}
