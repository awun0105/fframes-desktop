//! Voiceover timing: a block of narration with bookmarks, timed from a subtitle file.

use std::sync::LazyLock;

use fframes_kit::{
    motion::{cubic_in_out, progress},
    overlay::tiktok_pages,
    prelude::*,
    text::parse_srt,
};

use super::core::title;
use crate::GalleryScene;

const TEXT: &str = "Watch the <bookmark mark=\"circle\"/>circle grow, then <bookmark mark=\"square\"/>the square turn.";

/// One block on `tones.wav`, its words from the gallery's SRT, read once.
static SCRIPT: LazyLock<VoiceoverScript<'static>> = LazyLock::new(|| {
    let words = parse_srt(include_str!("../../captions/voiceover.srt"));
    VoiceoverScript::new(vec![VoiceoverBlock::new(TEXT, "tones.wav", words)])
});

static PAGES: LazyLock<Vec<TikTokPage>> =
    LazyLock::new(|| tiktok_pages(&SCRIPT.subcaptions(), 1200.0));

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("Voiceover", 3.5, |ui| {
            let head = title(ui, "Voiceover");
            let body = voiceover(ui);
            svgr!(<g>{head}{body}</g>)
        })
        .with_audio("tones.wav"),
    ]
}

/// A circle that grows at the `circle` bookmark, a square that turns at `square`, the block
/// on a timeline with its bookmarks, and the narration as word captions.
fn voiceover<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let script = &*SCRIPT;
    let c = ui.theme.colors;
    let t = ui.seconds();
    let appear = progress(t, 0.0, 0.3);

    let circle_at = script.bookmark("circle").unwrap_or(0.0);
    let grow = cubic_in_out(progress(t, circle_at, circle_at + 0.5));
    let r = 110.0 * grow;
    let square_at = script.bookmark("square").unwrap_or(0.0);
    let turn = cubic_in_out(progress(t, square_at, square_at + 0.8));
    let square = Transform::translate(1240.0, 400.0);
    let spin = Transform::rotate(90.0 * turn);
    let circle = if r > 0.5 {
        svgr!(<circle cx="680" cy="400" r={r} fill={c.accent} />)
    } else {
        Svgr::empty()
    };
    let shapes = svgr!(
        <g>
            {circle}
            <g transform={square} opacity={progress(t, square_at, square_at + 0.3)}>
                <g transform={spin}>
                    <rect x="-95" y="-95" width="190" height="190" rx="12" fill={c.highlight} />
                </g>
            </g>
        </g>
    );

    // the block on a timeline from 0 to its end, with its bookmarks and the playhead
    let (left, right, y) = (360.0, 1560.0, 660.0);
    let end = script.end().max(0.1);
    let at = |seconds: f32| left + (right - left) * (seconds / end).clamp(0.0, 1.0);
    let block = script.block(0).unwrap_or(BlockTime {
        start: 0.0,
        duration: 0.0,
        end: 0.0,
    });
    let (start_x, end_x) = (at(block.start), at(block.end));
    let ticks: Vec<Svgr<'a>> = script
        .bookmarks()
        .iter()
        .map(|mark| {
            let x = at(mark.seconds);
            let label = ui.show(HudLabel {
                text: mark.mark.clone().into(),
                x,
                y: y - 30.0,
                anchor: TextAnchor::Middle,
                fill: c.muted,
                ..ui.defaults()
            });
            svgr!(
                <g>
                    <line x1={x} y1={y - 16.0} x2={x} y2={y + 16.0} stroke={c.muted} stroke-width="3" />
                    {label}
                </g>
            )
        })
        .collect();
    // what is left of the block shrinks towards its end
    let left_now = script.remaining(0, t).unwrap_or(0.0);
    let left_x = end_x - (end_x - start_x) * left_now / block.duration.max(0.1);
    let label = ui.show(HudLabel {
        text: "left".into(),
        x: end_x,
        y: y + 64.0,
        anchor: TextAnchor::End,
        fill: c.muted,
        ..ui.defaults()
    });
    let remaining = svgr!(
        <g>
            <line x1={left_x} y1={y + 30.0} x2={end_x} y2={y + 30.0} stroke={c.accent}
                  stroke-width="6" stroke-linecap="round" opacity={(left_now / 0.2).min(1.0)} />
            {label}
        </g>
    );
    let playhead = at(t);
    let timeline = svgr!(
        <g opacity={appear}>
            <line x1={left} y1={y} x2={right} y2={y} stroke={c.surface} stroke-width="6" stroke-linecap="round" />
            <line x1={start_x} y1={y} x2={end_x} y2={y} stroke={c.muted} stroke-width="6" stroke-linecap="round" />
            {ticks}
            <circle cx={playhead} cy={y} r="10" fill={c.title} />
            {remaining}
        </g>
    );

    let captions = ui.show(WordCaptions {
        pages: PAGES.clone(),
        y: 880.0,
        style: ui.theme.typography.heading,
        ..ui.defaults()
    });
    svgr!(<g>{shapes}{timeline}{captions}</g>)
}
