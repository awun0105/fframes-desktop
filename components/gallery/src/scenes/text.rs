//! Text components and the beat grid, one scene each.

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

/// 120 BPM with beat 0 half a second in.
const GRID: Beat = Beat::new(0.5, 0.5);

pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "Beat", 3.0 => |ui| beat(ui),
        "FitText", 2.0 => |ui| fit_text(ui),
        "Letters", 2.0 => |ui| letters(ui),
        "TextReveal", 2.5 => |ui| {
            let reveal = TextReveal {
                text: "Words arrive one by one".into(),
                x: 96.0,
                y: 560.0,
                delay: 0.3,
                stagger: 0.12,
                ..ui.defaults()
            };
            ui.show(reveal)
        },
        "Typewriter", 3.0 => |ui| {
            let typing = Typewriter {
                text: "cargo fframes new my-video".into(),
                style: ui.theme.typography.heading,
                x: 96.0,
                y: 560.0,
                delay: 0.3,
                ..ui.defaults()
            };
            ui.show(typing)
        },
        "Counter", 2.5 => |ui| {
            let count = Counter {
                to: 12_480.0,
                x: 1200.0,
                y: 600.0,
                suffix: " frames".into(),
                ..ui.defaults()
            };
            ui.show(count)
        },
        "Paragraph", 2.0 => |ui| {
            let text = Paragraph {
                text: "A paragraph wraps into lines of the width it is given, measured with \
                       the font of the video, so the editor and the render break it the same way."
                    .into(),
                x: 96.0,
                y: 420.0,
                width: 900.0,
                style: ui.theme.typography.heading,
                ..ui.defaults()
            };
            ui.show(text)
        },
        "HudLabel", 2.0 => |ui| {
            let left = HudLabel { text: "01 / render".into(), x: 96.0, y: 560.0, ..ui.defaults() };
            let right = HudLabel {
                text: "frame 120".into(),
                x: 1824.0,
                y: 560.0,
                anchor: TextAnchor::End,
                fill: ui.theme.colors.accent,
                ..ui.defaults()
            };
            let left = ui.show(left);
            let right = ui.show(right);
            svgr!(<g>{left}{right}</g>)
        },
    ]
}

/// A dot that pulses on every beat, a bar that snaps in on beat 2, and the beat number.
fn beat<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let b = GRID.beat_at(ui.seconds());
    let r = 40.0 + 40.0 * Beat::pulse(b.max(0.0), 6.0);
    let width = 900.0 * GRID.snap(b - 2.0);
    let bar = if width > 0.0 {
        svgr!(<rect x="500" y="520" width={width} height="40" rx="20" fill={c.highlight} />)
    } else {
        Svgr::empty()
    };
    let label = ui.show(HudLabel {
        text: format!("beat {:.0}", b.floor().max(0.0)).into(),
        x: 96.0,
        y: 760.0,
        ..ui.defaults()
    });
    svgr!(
        <g>
            <circle cx="300" cy="540" r={r} fill={c.accent} />
            {bar}
            {label}
        </g>
    )
}

/// The same text in boxes of three widths.
fn fit_text<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    [(1600.0, 420.0), (900.0, 640.0), (420.0, 820.0)]
        .into_iter()
        .map(|(max_width, y)| {
            let text = ui.show(FitText {
                text: "Shrinks to fit the box".into(),
                x: 96.0,
                y,
                max_width,
                ..ui.defaults()
            });
            svgr!(
                <g>
                    <rect x="96" y={y + 12.0} width={max_width} height="4" fill={c.muted} />
                    {text}
                </g>
            )
        })
        .collect()
}

/// Letters that drop in one after another with a spring.
fn letters<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let t = ui.seconds();
    let letters = Letters {
        text: "LETTERS".into(),
        x: 96.0,
        y: 600.0,
        tracking: 8.0,
        look: Box::new(move |i| {
            let s = motion::spring(t - 0.2 - motion::stagger(i, 0.06), 260.0, 18.0);
            LetterLook {
                dy: (1.0 - s) * -120.0,
                opacity: motion::clamp01(s * 2.0),
                scale: 0.6 + 0.4 * s,
                ..LetterLook::default()
            }
        }),
        ..ui.defaults()
    };
    ui.show(letters)
}
