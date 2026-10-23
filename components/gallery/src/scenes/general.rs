//! Data, audio, decoration and media components, one scene each.

use fframes::SampleSize;
use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

/// The scene with `title`, as in the other gallery files.
macro_rules! scenes {
    ($($name:literal, $seconds:expr => |$ui:ident| $body:expr,)*) => {
        vec![$(GalleryScene::new($name, $seconds, |$ui| {
            let head = title($ui, $name);
            let body: Svgr<'_> = $body;
            svgr!(<g>{head}{body}</g>)
        }),)*]
    };
}

/// The spectrum of the gallery's tone file, `step` bins apart from the first band up.
fn tones(ui: &Ui<'_, '_, '_>, step: usize) -> Vec<f32> {
    audio::spectrum(ui, "tones.wav", SampleSize::S256, 2)
        .map(|s| s.into_iter().skip(1).step_by(step).take(32).collect())
        .unwrap_or_default()
}

pub fn scenes() -> Vec<GalleryScene> {
    let scenes = scenes![
        "BarChart", 2.5 => |ui| {
            let chart = BarChart {
                bars: vec![("01".into(), 24.0), ("02".into(), 41.0), ("03".into(), 58.0), ("04".into(), 76.0)],
                x: 600.0,
                y: 820.0,
                highlight: Some(3),
                delay: 0.25,
                ..ui.defaults()
            };
            ui.show(chart)
        },
        "SpectrumBars", 3.0 => |ui| {
            let bars = SpectrumBars { values: tones(ui, 1), x: 360.0, y: 600.0, width: 1200.0, bar_width: 22.0, ..ui.defaults() };
            ui.show(bars)
        },
        "Waveform", 3.0 => |ui| {
            let c = ui.theme.colors;
            let back = Waveform { values: tones(ui, 3), fill: Some(c.highlight), opacity: 0.6, gain: 80.0, ..ui.defaults() };
            let front = Waveform { values: tones(ui, 2), gain: 60.0, ..ui.defaults() };
            let back = ui.show(back);
            let front = ui.show(front);
            svgr!(<g>{back}{front}</g>)
        },
        "DrawOn", 2.0 => |ui| {
            let c = ui.theme.colors;
            let path = BezPath::from_svg("M560 640 C760 520 1000 760 1360 600").unwrap_or_default();
            let shape = Shape { path, fill: None, stroke: Some(c.accent), stroke_width: 10.0, dash: None, opacity: 1.0 };
            let line = DrawOn { delay: 0.2, run_time: 1.2, ..DrawOn::new(ui, shape) };
            ui.show(line)
        },
        "CornerMarks", 2.0 => |ui| {
            let c = ui.theme.colors;
            let still = CornerMarks { x: 360.0, y: 400.0, width: 480.0, height: 320.0, arm: 32.0, stroke_width: 3.0, ..ui.defaults() };
            let drawn = CornerMarks {
                x: 1080.0,
                color: c.accent,
                enter: Some(Create { delay: 0.2, ..ui.defaults() }.into()),
                ..still.clone()
            };
            let still = ui.show(still);
            let drawn = ui.show(drawn);
            svgr!(<g>{still}{drawn}</g>)
        },
        "Glow", 2.0 => |ui| {
            let c = ui.theme.colors;
            let glow = Glow::default();
            let url = glow.url();
            let defs = ui.show(glow);
            svgr!(
                <g>
                    <defs>{defs}</defs>
                    <circle cx="960" cy="580" r="120" fill={c.title} filter={url} />
                </g>
            )
        },
        "DropShadow", 2.0 => |ui| {
            let c = ui.theme.colors;
            let r = ui.theme.radius.card;
            let shadow = DropShadow::default();
            let url = shadow.url();
            let defs = ui.show(shadow);
            svgr!(
                <g>
                    <defs>{defs}</defs>
                    <rect x="720" y="440" width="480" height="280" rx={r} fill={c.surface} filter={url} />
                </g>
            )
        },
        "PhotoFrame", 2.0 => |ui| {
            let frame = PhotoFrame { image: "photo.png", x: 720.0, y: 420.0, ..ui.defaults() };
            ui.show(frame)
        },
        "KenBurns", 3.0 => |ui| {
            let photo = KenBurns { image: "photo.png", x: 480.0, y: 300.0, width: 960.0, height: 560.0, drift: 40.0, ..ui.defaults() };
            ui.show(photo)
        },
    ];
    scenes
        .into_iter()
        .map(|s| match s.name {
            "SpectrumBars" | "Waveform" => s.with_audio("tones.wav"),
            _ => s,
        })
        .collect()
}
