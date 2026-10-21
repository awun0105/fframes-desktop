//! Act 4: what people build with fframes, as a bento of four cards playing real
//! renders of fframes examples: 3D, a Lottie file, a motion film and shaders.
//! Each card in turn grows to the spotlight while the headline flips to its name.

use std::sync::LazyLock;

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};
use fframes_components::components::word_flip::{WordFlip, WordFlipConfig, typing_end_frame};

use super::comp;
use super::look::*;
use crate::beat::*;

beat_scene!(UseCasesScene, Some(128.0), Some(176.0));

/// Beats (inside the act) each card takes the spotlight; the last one hands back
/// to the grid.
const SPOTLIGHTS: [f32; 4] = [4.0, 16.0, 27.0, 38.0];
const GRID_AGAIN: f32 = 46.0;
/// Beat the headline starts typing.
const TYPE_FROM: f32 = 0.5;
/// The flipping word turns this many beats before its card moves.
const FLIP_LEAD: f32 = 0.5;

/// Video beats the cards land on and take the spotlight.
pub const BLIP_BEATS: [f32; 4] = [128.5, 129.0, 129.5, 130.0];

/// Video beats the spotlight moves on.
pub fn spotlight_beats() -> impl Iterator<Item = f32> {
    SPOTLIGHTS.iter().skip(1).map(|b| 128.0 + b)
}

const PREFIX: &str = "Build";
const WORDS: [&str; 4] = [
    "3D films",
    "Lottie animations",
    "motion design",
    "shader art",
];

static FLIP: LazyLock<WordFlip> = LazyLock::new(|| {
    let fps = 30.0;
    let cps = 16.0;
    let type_start = 2.0;
    let char_fade = 6.0;
    let jitter = 0.18;
    let chars = format!("{PREFIX} {} ", WORDS[0]).chars().count();
    let typing_end = typing_end_frame(chars, cps, type_start, char_fade, jitter, fps);
    // the flips are spaced like the spotlights, the first one lands with the second card
    let frames = |beats: f32| beats * BEAT * fps;
    let first_flip = frames(SPOTLIGHTS[1] - FLIP_LEAD - TYPE_FROM);
    WordFlip::new(WordFlipConfig {
        prefix: PREFIX.to_owned(),
        words: WORDS.iter().map(|w| (*w).to_owned()).collect(),
        suffix: String::new(),
        cps,
        type_start,
        char_fade,
        jitter,
        pause: (first_flip - typing_end).max(0.0),
        cycle: frames(SPOTLIGHTS[2] - SPOTLIGHTS[1]),
        caret: false,
        font_size: 72.0,
        font_weight: DISPLAY_WEIGHT,
        tracking_em: -0.04,
        color: TEXT.to_owned(),
        accent_color: ACTION.to_owned(),
        ..Default::default()
    })
});

struct Case {
    title: &'static str,
    source: &'static str,
    lead: &'static str,
    caption: &'static str,
    clip: &'static str,
    poster: &'static str,
}

const CASES: [Case; 4] = [
    Case {
        title: "3D film",
        source: "examples/overdrive",
        lead: "3D:",
        caption: "a car turns into a robot, lit and rendered by fframes.",
        clip: "overdrive.mp4",
        poster: "overdrive_poster.jpg",
    },
    Case {
        title: "Lottie animation",
        source: "LottieFiles · After Effects",
        lead: "Lottie:",
        caption: "any LottieFiles animation plays in one call, as vectors.",
        clip: "lottie.mp4",
        poster: "lottie_poster.jpg",
    },
    Case {
        title: "Motion design",
        source: "examples/made-of-motion",
        lead: "Motion design:",
        caption: "a pixel-art motion film, every frame drawn in code.",
        clip: "made-of-motion.mp4",
        poster: "made-of-motion_poster.jpg",
    },
    Case {
        title: "Shader art",
        source: "examples/shader-mode",
        lead: "Shader art:",
        caption: "18 shaders: chrome, halftone, gradients and flow.",
        clip: "shader-mode.mp4",
        poster: "shader-mode_poster.jpg",
    },
];

type Rect = (f32, f32, f32, f32);

fn grid_rect(i: usize) -> Rect {
    let (w, h) = (830.0, 300.0);
    (
        120.0 + (i % 2) as f32 * (w + 20.0),
        310.0 + (i / 2) as f32 * (h + 20.0),
        w,
        h,
    )
}

/// Where card `i` sits while card `active` has the spotlight.
fn spot_rect(i: usize, active: usize) -> Rect {
    if i == active {
        return (120.0, 310.0, 1240.0, 620.0);
    }
    let slot = (0..4)
        .filter(|j| *j != active)
        .position(|j| j == i)
        .unwrap_or(0);
    (1380.0, 310.0 + slot as f32 * 213.0, 420.0, 194.0)
}

fn mix(a: Rect, b: Rect, t: f32) -> Rect {
    (
        lerp(a.0, b.0, t),
        lerp(a.1, b.1, t),
        lerp(a.2, b.2, t),
        lerp(a.3, b.3, t),
    )
}

/// The rectangle of card `i` at beat `lb`, gliding between layouts on each change.
fn card_rect(i: usize, lb: f32) -> Rect {
    let layout = |k: Option<usize>| match k {
        Some(active) => spot_rect(i, active),
        None => grid_rect(i),
    };
    // (beat, layout) keys: the grid, each spotlight, the grid again
    let keys: [(f32, Option<usize>); 6] = [
        (0.0, None),
        (SPOTLIGHTS[0], Some(0)),
        (SPOTLIGHTS[1], Some(1)),
        (SPOTLIGHTS[2], Some(2)),
        (SPOTLIGHTS[3], Some(3)),
        (GRID_AGAIN, None),
    ];
    let current = keys.iter().rposition(|(b, _)| lb >= *b).unwrap_or(0);
    let to = layout(keys[current].1);
    if current == 0 {
        return to;
    }
    let from = layout(keys[current - 1].1);
    let t = cubic_in_out(prog(lb, keys[current].0 - 0.5, keys[current].0 + 0.5));
    mix(from, to, t)
}

impl Scene for UseCasesScene {
    fn duration(&self) -> Duration<'_> {
        Self::frames()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let lb = Self::lb(&frame);
        let active = SPOTLIGHTS
            .iter()
            .rposition(|s| lb >= *s && lb < GRID_AGAIN)
            .map(|i| i.min(3));
        let cards: Vec<Svgr> = (0..4)
            .map(|i| card(&frame, ctx, lb, i, active == Some(i)))
            .collect();
        let captions: Vec<Svgr> = CASES
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let until = SPOTLIGHTS.get(i + 1).copied().unwrap_or(GRID_AGAIN) - 0.6;
                caption(lb, SPOTLIGHTS[i] + 0.4, until, c.lead, c.caption, ACTION)
            })
            .collect();
        let (eo, ed) = rise(lb, 0.0, 24.0);
        fframes::svgr!(
            <g opacity={leave(lb, 47.4)}>
                <g opacity={eo} transform={at(0.0, ed)}><text x="960" y="150" text-anchor="middle" font-family={MONO} font-weight="500" font-size="22" letter-spacing="3" fill={ACTION}>"04"<tspan fill={SECONDARY}>" / USE CASES"</tspan></text></g>
                {comp::show(&*FLIP, &frame, (lb - TYPE_FROM) * BEAT, (0.0, -320.0, 1920.0, 1080.0))}
                {cards}
                {captions}
            </g>
        )
    }
}

fn card(
    frame: &Frame,
    ctx: &FFramesContext<'_, '_>,
    lb: f32,
    i: usize,
    active: bool,
) -> Svgr<'static> {
    let land = 0.5 + i as f32 * 0.5;
    if lb < land - 0.3 {
        return Svgr::empty();
    }
    let (o, d) = rise(lb, land - 0.3, 50.0);
    let r = card_rect(i, lb);
    let (x, y, w, h) = r;
    // each clip starts over when its card takes the spotlight
    let body = clip(
        frame,
        ctx,
        CASES[i].clip,
        CASES[i].poster,
        (SPOTLIGHTS[i] - 0.5) * BEAT,
        r,
    );
    let c = &CASES[i];
    let small = w < 600.0;
    let strip_h = if small { 44.0 } else { 64.0 };
    let title_size = if small { 19.0 } else { 28.0 };
    let id = format!("case-{i}");
    let number = format!("0{}", i + 1);
    let label_y = y + h - strip_h / 2.0 + title_size * 0.36;
    fframes::svgr!(
        <g opacity={o} transform={at(0.0, d)}>
            {rounded(&id, r, RADIUS, body)}
            <rect x={x} y={y + h - strip_h} width={w} height={strip_h} fill={BG} opacity="0.94" />
            <rect x={x} y={y + h - strip_h} width={w} height="1.5" fill={RULE} />
            <text x={x + 20.0} y={label_y} font-family={MONO} font-weight="500" font-size={title_size * 0.75} fill={ACTION}>{number}</text>
            <text x={x + 20.0 + title_size * 1.8} y={label_y} font-family={SANS} font-weight="500" font-size={title_size} letter-spacing={-title_size * 0.02} fill={TEXT}>{c.title}</text>
            <text x={x + w - 20.0} y={label_y} text-anchor="end" font-family={MONO} font-weight="400" font-size={title_size * 0.7} fill={SECONDARY} opacity={if small { 0.0 } else { 1.0 }}>{c.source}</text>
            <text x={x + w - 20.0} y={y + 30.0} text-anchor="end" font-family={MONO} font-weight="400" font-size="14" fill={SECONDARY} opacity={if i == 1 && !small { 0.8 } else { 0.0 }}>"Emoji: Noto Animated Emoji, CC BY 4.0 · lottie-ios, Apache 2.0"</text>
            <rect x={x} y={y} width={w} height={h} rx={RADIUS} fill="none" stroke={if active { ACTION } else { RULE }} stroke-width={if active { 2.0 } else { RULE_WIDTH }} />

        </g>
    )
}
