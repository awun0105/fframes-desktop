//! Overlay components, one scene each.

use std::sync::LazyLock;

use fframes_kit::{overlay::tiktok_pages, prelude::*, text::from_whisper_json};

use super::core::title;
use crate::GalleryScene;

/// The pages of the gallery's Whisper transcript, timed to `tones.wav`, read once.
static PAGES: LazyLock<Vec<TikTokPage>> = LazyLock::new(|| {
    let words = from_whisper_json(include_str!("../../captions/words.json")).unwrap_or_default();
    tiktok_pages(&words, 1200.0)
});

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

pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "Appear", 2.5 => |ui| appear(ui),
        "LowerThird", 3.0 => |ui| {
            let lower = LowerThird {
                name: "Ada Lovelace".into(),
                role: "Analyst, Analytical Engine".into(),
                background: Some(ui.theme.colors.surface),
                delay: 0.2,
                ..ui.defaults()
            };
            ui.show(lower)
        },
        "Captions", 3.0 => |ui| {
            let captions = Captions { file: "captions.vtt", video_time: false, ..ui.defaults() };
            ui.show(captions)
        },
        "WordCaptions", 3.5 => |ui| {
            let captions = WordCaptions {
                pages: PAGES.clone(),
                y: 600.0,
                pill: Some(ui.theme.colors.surface),
                ..ui.defaults()
            };
            ui.show(captions)
        },
        "CaptionStack", 3.0 => |ui| {
            let stack = CaptionStack {
                items: vec![
                    Caption::new(0.2, "STEP 1 / ASK", "Say what to change."),
                    Caption::new(1.4, "STEP 2 / EDIT", "It edits only that."),
                ],
                ..ui.defaults()
            };
            ui.show(stack)
        },
        "ChapterList", 3.0 => |ui| {
            let list = ChapterList {
                chapters: vec![
                    (0.0, "Introduction".into()),
                    (1.0, "Scenes and time".into()),
                    (2.0, "Rendering".into()),
                ],
                ..ui.defaults()
            };
            ui.show(list)
        },
        "Annotation", 2.0 => |ui| annotation(ui),
        "ProgressBar", 2.0 => |ui| {
            let bar = ProgressBar { y: 560.0, height: 12.0, radius: 6.0, ..ui.defaults() };
            ui.show(bar)
        },
        "Check", 2.0 => |ui| {
            let check = Check { x: 900.0, y: 520.0, size: 3.0, stroke_width: 12.0, delay: 0.3, ..ui.defaults() };
            ui.show(check)
        },
    ]
}

/// A card that appears half a second in and leaves at 2 seconds.
fn appear<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let r = ui.theme.radius.card;
    let card = Appear {
        until: Some(2.0),
        ..Appear::new(
            ui,
            0.5,
            (960.0, 560.0),
            children(move |ui| {
                let label = ui.show(HudLabel {
                    text: "appears and leaves".into(),
                    x: 960.0,
                    y: 570.0,
                    anchor: TextAnchor::Middle,
                    fill: c.title,
                    ..ui.defaults()
                });
                svgr!(
                    <g>
                        <rect x="720" y="440" width="480" height="240" rx={r} fill={c.surface} />
                        {label}
                    </g>
                )
            }),
        )
    };
    ui.show(card)
}

/// A dot on a box with the label off to the right.
fn annotation<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let note = ui.show(Annotation {
        target: (760.0, 600.0),
        label: (1100.0, 420.0),
        text: "the part that changed".into(),
        delay: 0.2,
        ..ui.defaults()
    });
    svgr!(
        <g>
            <rect x="560" y="500" width="400" height="200" rx="16" fill={c.surface} />
            {note}
        </g>
    )
}
