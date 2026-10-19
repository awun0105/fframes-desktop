//! Act 7: get it. On the last drop a wall of everything shown so far plays at
//! once, collapses into the middle, and digital rain writes the end card's code
//! to scan for the download.

use std::fmt::Write;
use std::sync::LazyLock;

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};
use fframes_components::components::tracking_in::{TrackingIn, TrackingInConfig};

use super::comp;
use super::look::*;
use crate::beat::*;
use crate::short::qr::{MODULES, dark, qr};

beat_scene!(FinaleScene, Some(240.0), Some(LONG_END));

/// Beat (inside the act) the wall collapses.
const COLLAPSE: f32 = 8.0;
/// Beat the end card is up.
const CARD: f32 = 9.0;
/// Beats from the end the picture fades out over.
const FADE: f32 = 2.5;

/// Video beats the wall's tiles and the end card's lines land on.
pub const BLIP_BEATS: [f32; 3] = [249.0, 251.0, 252.0];

/// Beats the digital rain takes to write the whole code.
const MATRIX_BEATS: f32 = 1.8;
/// Glyphs in a falling column of the rain, head included.
const TRAIL: usize = 7;
/// What the rain is made of.
const GLYPHS: [&str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
];
static NAME: LazyLock<TrackingIn> = LazyLock::new(|| {
    TrackingIn::new(TrackingInConfig {
        text: "FFrames Studio".to_owned(),
        font_size: 88.0,
        color: TEXT.to_owned(),
        font_weight: 400,
        start_tracking: 0.22,
        ..Default::default()
    })
});

enum Tile {
    Clip(&'static str, &'static str),
    Still(&'static str),
}

/// The wall, row by row: four columns, three rows.
const WALL: [Tile; 12] = [
    Tile::Clip("overdrive.mp4", "overdrive_poster.jpg"),
    Tile::Still("shader-chrome.jpg"),
    Tile::Clip("lottie.mp4", "lottie_poster.jpg"),
    Tile::Still("motion-wordmark.jpg"),
    Tile::Clip("made-of-motion.mp4", "made-of-motion_poster.jpg"),
    Tile::Still("overdrive-robot.jpg"),
    Tile::Clip("shader-mode.mp4", "shader-mode_poster.jpg"),
    Tile::Still("lottie-grid.jpg"),
    Tile::Still("shader-gradient.jpg"),
    Tile::Still("overdrive-fire.jpg"),
    Tile::Still("motion-vinyl.jpg"),
    Tile::Still("shader-title.jpg"),
];

impl Scene for FinaleScene {
    fn duration(&self) -> Duration<'_> {
        Self::frames()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let lb = Self::lb(&frame);
        let length = LONG_END - 240.0;
        let wall = if lb < COLLAPSE + 1.5 {
            wall(&frame, ctx, lb)
        } else {
            Svgr::empty()
        };
        let card = if lb >= CARD - 0.5 {
            end_card(&frame, lb)
        } else {
            Svgr::empty()
        };
        fframes::svgr!(
            <g opacity={leave(lb, length - FADE)}>
                {wall}
                {card}
            </g>
        )
    }
}

/// Every render shown in the video at once, slowly pushing in, then pulled into
/// the middle.
fn wall(frame: &Frame, ctx: &FFramesContext<'_, '_>, lb: f32) -> Svgr<'static> {
    let (cols, gap) = (4, 12.0);
    let (tw, th) = (
        (1920.0 - 80.0 - gap * 3.0) / cols as f32,
        (1080.0 - 80.0 - gap * 2.0) / 3.0,
    );
    let collapse = expo_in(prog(lb, COLLAPSE, COLLAPSE + 1.2));
    let push = 1.0 + 0.06 * prog(lb, 0.0, COLLAPSE);
    let tiles: Vec<Svgr> = WALL
        .iter()
        .enumerate()
        .map(|(i, tile)| {
            let land = (hash(i as f32 * 7.3) * 3.0).floor() * 0.5;
            if lb < land {
                return Svgr::empty();
            }
            let x = 40.0 + (i % cols) as f32 * (tw + gap);
            let y = 40.0 + (i / cols) as f32 * (th + gap);
            let rect = (x, y, tw, th);
            let body = match tile {
                Tile::Clip(name, poster) => clip(frame, ctx, name, poster, 0.0, rect),
                Tile::Still(name) => still(ctx, name, rect),
            };
            let pop = 0.9 + 0.1 * snap(lb - land);
            let (mx, my) = (lerp(x + tw / 2.0, 960.0, collapse), lerp(y + th / 2.0, 540.0, collapse));
            let s = pop * (1.0 - 0.9 * collapse);
            let transform = format!(
                "translate({mx} {my}) scale({s}) translate({} {})",
                -(x + tw / 2.0),
                -(y + th / 2.0)
            );
            let id = format!("wall-{i}");
            fframes::svgr!(
                <g transform={transform} opacity={prog(lb, land, land + 0.25) * (1.0 - collapse)}>
                    {rounded(&id, rect, 10.0, body)}
                    <rect x={x} y={y} width={tw} height={th} rx="10" fill="none" stroke={LINE} stroke-width="2" />
                </g>
            )
        })
        .collect();
    fframes::svgr!(<g transform={zoom(960.0, 540.0, push)}>{tiles}</g>)
}

/// The name and a code to scan for the installers.
fn end_card(frame: &Frame, lb: f32) -> Svgr<'static> {
    let local = lb - CARD;
    let (to, td) = rise(lb, CARD + 1.2, 20.0);
    let (ro, rd) = rise(lb, CARD + 2.6, 20.0);
    // an orange hairline traced around the whole card
    let trace = cubic_in_out(prog(lb, CARD, CARD + 3.0));
    let perimeter = 2.0 * (1680.0 + 840.0);
    // the code springs up as a dark tile, digital rain falls through it and
    // leaves the code behind, light tile and quiet zone included
    let (qx, qy, qs) = (560.0, 545.0, 330.0);
    let land = CARD + 2.0;
    let pop = 0.85 + 0.15 * spring((lb - land).max(0.0) * BEAT, 220.0, 18.0);
    let qo = prog(lb, land, land + 0.3);
    let tx = 960.0;
    fframes::svgr!(
        <g opacity={prog(lb, CARD - 0.5, CARD)}>
            {rain(lb, land - 1.0, land + MATRIX_BEATS + 0.6)}
            <rect x="120" y="120" width="1680" height="840" rx="20" fill="none" stroke={ACCENT} stroke-width="2"
                stroke-dasharray={format!("{} {perimeter}", perimeter * trace)} />
            <g transform={at(0.0, -190.0)}>{comp::show(&*NAME, frame, local * BEAT, (0.0, -60.0, 1920.0, 1080.0))}</g>
            <g opacity={to} transform={at(0.0, td)}>
                <text x="960" y="490" text-anchor="middle" font-family={MONO} font-weight="500" font-size="28" fill={SLATE}>"Programmatic video, with a desktop studio."</text>
            </g>
            <g opacity={qo} transform={zoom(qx + qs / 2.0, qy + qs / 2.0, pop)}>
                {rounded("qr-tile", (qx, qy, qs, qs), 18.0, matrix_code(lb - land, (qx, qy, qs)))}
                <rect x={qx} y={qy} width={qs} height={qs} rx="18" fill="none" stroke={ACCENT} stroke-width="2" opacity={1.0 - prog(lb, land + MATRIX_BEATS, land + MATRIX_BEATS + 0.5)} />
            </g>
            <g opacity={ro} transform={at(0.0, rd)}>
                <text x={tx} y="625" font-family={DISPLAY} font-size="52" letter-spacing="-1.5" fill={TEXT}>"Scan to download."</text>
                <text x={tx} y="675" font-family={MONO} font-weight="500" font-size="24" fill={SLATE}>"Download for Windows, macOS and Linux."</text>
                <text x={tx} y="755" font-family={MONO} font-weight="500" font-size="21" fill={TEXT}>"dub.sh/fframes"</text>
                <text x={tx} y="830" font-family={MONO} font-weight="600" font-size="18" letter-spacing="4" fill={ACCENT}>"OPEN SOURCE · MIT · BUILT ON FFRAMES"</text>
                <text x={tx} y="905" font-family={MONO} font-weight="500" font-size="14" fill={MUTED}>"Lottie emoji: Noto Animated Emoji, CC BY 4.0 · lottie-ios samples, Apache 2.0"</text>
            </g>
        </g>
    )
}

/// The code written by digital rain: each column of cells has a falling trail
/// of glyphs, brighter where the code has a dark module, and the code settles
/// in behind it. `t` is in beats from when the tile lands.
fn matrix_code(t: f32, (qx, qy, qs): (f32, f32, f32)) -> Svgr<'static> {
    let n = MODULES as usize;
    let unit = qs / MODULES;
    // `assets/fframes-qrcode.svg`: the code with its quiet zone, and the short
    // link's mark in the middle where the code leaves room for it
    let code = fframes::svgr!(
        <g>
            <rect x={qx} y={qy} width={qs} height={qs} fill="#ffffff" />
            <g transform={format!("translate({qx} {qy}) scale({unit})")}>
                {qr()}
                <clipPath id="qr-mark"><circle cx="16.5" cy="16.5" r="3.625" /></clipPath>
                <g clip-path="url(#qr-mark)">
                    <circle cx="16.5" cy="16.5" r="3.625" fill="#000000" />
                    <circle cx="16.5" cy="16.5" r="1.8125" fill="#ffffff" />
                    <rect x="17.406" y="12.875" width="0.906" height="5.4375" fill="#ffffff" />
                </g>
            </g>
        </g>
    );
    if t >= MATRIX_BEATS {
        return code;
    }
    // every column starts a little later than the last one hashed, and falls
    // until its whole trail has left the tile
    let fall = MATRIX_BEATS - 1.0;
    let speed = (n + TRAIL) as f32 / fall;
    let tick = (t * 8.0).floor();
    let mut revealed = String::new();
    let mut glyphs = Vec::new();
    for c in 0..n {
        let head = (t - hash(c as f32 * 13.1) * 0.9) * speed;
        if head <= 0.0 {
            continue;
        }
        let settled = (head - TRAIL as f32).clamp(0.0, n as f32);
        if settled > 0.0 {
            let _ = write!(
                revealed,
                "M{} {qy}h{unit}v{}h-{unit}z",
                qx + c as f32 * unit,
                settled.floor() * unit
            );
        }
        for k in 0..TRAIL {
            let row = head.floor() as i64 - k as i64;
            if row < 0 || row >= n as i64 {
                continue;
            }
            let row = row as usize;
            let on_module = dark(c, row);
            let glyph = GLYPHS
                [(hash(c as f32 * 31.7 + row as f32 * 7.13 + tick * 1.31) * 16.0) as usize % 16];
            let fill = match (k, on_module) {
                (0, _) => "#eaf2ff",
                (_, true) => SKY,
                _ => ACCENT,
            };
            let opacity = (1.0 - k as f32 / TRAIL as f32 * 0.85)
                * if on_module || k == 0 { 1.0 } else { 0.55 };
            glyphs.push(fframes::svgr!(
                <text x={qx + (c as f32 + 0.5) * unit} y={qy + (row as f32 + 0.85) * unit} text-anchor="middle"
                    font-family={MONO} font-weight="600" font-size={unit * 1.15} fill={fill} opacity={opacity}>{glyph}</text>
            ));
        }
    }
    let settled = if revealed.is_empty() {
        Svgr::empty()
    } else {
        fframes::svgr!(
            <g>
                <clipPath id="qr-settled"><path d={revealed} /></clipPath>
                <g clip-path="url(#qr-settled)">{code}</g>
            </g>
        )
    };
    fframes::svgr!(
        <g>
            <rect x={qx} y={qy} width={qs} height={qs} fill="#050b16" />
            {settled}
            {glyphs}
        </g>
    )
}

/// Digital rain across the whole card while the code is written, from `start`
/// to `end` (beats of the act), faint enough to read the card through.
fn rain(lb: f32, start: f32, end: f32) -> Svgr<'static> {
    let o = prog(lb, start, start + 0.5) * (1.0 - prog(lb, end - 1.0, end));
    if o <= 0.0 {
        return Svgr::empty();
    }
    let (cell, rows, period) = (30.0, 28, 40.0);
    let t = lb - start;
    let tick = (t * 6.0).floor();
    let mut glyphs = Vec::new();
    for c in 0..56 {
        let speed = 14.0 + hash(c as f32 * 1.7) * 12.0;
        let head = (t * speed + hash(c as f32 * 3.3) * period) % period;
        for k in 0..10 {
            let row = head.floor() as i64 - k;
            if !(0..rows).contains(&row) {
                continue;
            }
            let glyph =
                GLYPHS[(hash(c as f32 * 5.9 + row as f32 * 2.3 + tick) * 16.0) as usize % 16];
            let (fill, alpha) = if k == 0 {
                (SKY, 0.5)
            } else {
                (ACCENT, 0.32 * (1.0 - k as f32 / 10.0))
            };
            let (x, y) = (135.0 + c as f32 * cell, 142.0 + row as f32 * cell);
            // fainter behind the name, the tagline and the download lines
            let behind_text = (390.0..1570.0).contains(&x) && (280.0..510.0).contains(&y)
                || (930.0..1620.0).contains(&x) && (560.0..850.0).contains(&y);
            let alpha = alpha * if behind_text { 0.3 } else { 1.0 };
            glyphs.push(fframes::svgr!(
                <text x={x} y={y} text-anchor="middle"
                    font-family={MONO} font-weight="500" font-size="20" fill={fill} opacity={alpha * o}>{glyph}</text>
            ));
        }
    }
    fframes::svgr!(<g>{glyphs}</g>)
}
