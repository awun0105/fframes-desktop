//! Asset packs: cursors with their clicks and drags, animated emoji, the sound effects and
//! a GIF. The scenes with sound place it with the `sfx` helpers.

use std::sync::LazyLock;

use fframes::AudioTrack;
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

/// The click of the `Click` scene.
const CLICK_AT: f32 = 0.9;

/// The drag of the `Drag` scene: it starts at 0.9 s and lasts a second.
const DRAG: (f32, f32) = (0.9, 1.0);

/// When the `TypeAt` scene starts typing; it clicks `lead` (0.4 s) before.
const TYPE_DELAY: f32 = 1.0;

/// The sequences of the `TransitionSfx` series and the fade between them.
const PANEL: f32 = 1.5;
const FADE: Timing = Timing::Linear {
    duration: 0.5,
    easing: RateFunc::Smooth,
};

/// When the `SoundEffects` scene plays each sound of the pack.
fn effect_time(i: usize) -> f32 {
    0.4 + 0.5 * i as f32
}

/// The GIF of the gallery, drawn for it (twelve frames of bouncing squares), decoded once.
static BOUNCE: LazyLock<Option<GifData>> =
    LazyLock::new(|| Gif::decode(include_bytes!("../../media/gif/bounce.gif")).ok());

pub fn scenes() -> Vec<GalleryScene> {
    let mut scenes = scenes![
        "Cursor", 3.5 => |ui| cursor(ui),
        "Click", 2.0 => |ui| click(ui),
        "Drag", 2.6 => |ui| drag(ui),
        "TypeAt", 3.0 => |ui| type_at(ui),
        "AnimatedEmoji", 3.0 => |ui| animated_emoji(ui),
        "SoundEffects", 4.2 => |ui| sound_effects(ui),
        "TransitionSfx", 3.5 => |ui| transition_sfx(ui),
        "Gif", 2.5 => |ui| gif(ui),
    ];
    let tracks: [(&str, crate::Tracks); 6] = [
        ("Cursor", || sfx::on_clicks(&cursor_path())),
        ("Click", || sfx::at(&[CLICK_AT], Sfx::Click)),
        ("Drag", || sfx::at(&[DRAG.0 + DRAG.1], Sfx::Pop)),
        ("TypeAt", || sfx::at(&[TYPE_DELAY - 0.4], Sfx::Click)),
        ("SoundEffects", sound_effect_tracks),
        ("TransitionSfx", || {
            sfx::on_transitions(&transition_lengths(), 30, Sfx::Whoosh)
        }),
    ];
    for scene in &mut scenes {
        if let Some((_, tracks)) = tracks.iter().find(|(name, _)| *name == scene.name) {
            scene.tracks = Some(*tracks);
        }
    }
    scenes
}

/// A rounded card with a label in its middle.
fn card<'a>(
    ui: &Ui<'a, '_, '_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fill: Color,
    label: &'a str,
) -> Svgr<'a> {
    let s = ui.theme.typography.caption;
    let c = ui.theme.colors;
    let (tx, ty) = (x + w / 2.0, y + h / 2.0 + s.size as f32 * 0.35);
    svgr!(
        <g>
            <rect x={x} y={y} width={w} height={h} rx="18" fill={fill} />
            <text x={tx} y={ty} text-anchor="middle" font-family={s.family} font-size={s.size}
                  font-weight={s.weight} fill={c.title}>{label}</text>
        </g>
    )
}

/// A caption centred under `x`.
fn caption<'a>(ui: &Ui<'a, '_, '_>, x: f32, y: f32, text: impl std::fmt::Display) -> Svgr<'a> {
    let text = text.to_string();
    let s = ui.theme.typography.caption;
    let c = ui.theme.colors;
    svgr!(
        <text x={x} y={y} text-anchor="middle" font-family={s.family} font-size="24"
              font-weight={s.weight} fill={c.muted}>{text}</text>
    )
}

/// The path of the `Cursor` scene, shared with its click sounds.
fn cursor_path() -> CursorPath {
    CursorPath {
        points: vec![
            CursorPoint::new(0.0, 640.0, 900.0),
            CursorPoint::new(1.0, 820.0, 690.0).click(),
            CursorPoint::new(2.0, 1180.0, 690.0).click(),
            CursorPoint::new(3.0, 1040.0, 900.0),
        ],
        shape: CursorShape::Pointer,
        size: 44.0,
        travel: 0.9,
        ..CursorPath::default()
    }
}

/// Every shape in both styles, and a pointer that clicks two buttons.
fn cursor<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let names = ["Arrow", "Pointer", "IBeam", "Grab", "Grabbing", "Crosshair"];
    let mut shapes = Vec::new();
    for (i, (shape, name)) in CursorShape::ALL.iter().zip(names).enumerate() {
        let x = 330.0 + 250.0 * i as f32;
        // the middle of each drawing, in the units of its 24 unit design
        let (cx, cy) = match shape {
            CursorShape::Arrow => (6.4, 10.0),
            CursorShape::Pointer => (10.0, 11.0),
            CursorShape::Grab => (9.0, 11.0),
            CursorShape::Grabbing => (10.5, 14.0),
            CursorShape::IBeam | CursorShape::Crosshair => (0.0, 0.0),
        };
        let (hx, hy) = shape.hotspot();
        let k = 48.0 / 24.0;
        for (style, dx) in [(CursorStyle::Dark, -40.0), (CursorStyle::Light, 40.0)] {
            shapes.push(ui.show(Cursor {
                shape: *shape,
                style,
                x: x + dx + (hx - cx) * k,
                y: 300.0 + (hy - cy) * k,
                size: 48.0,
                ..ui.defaults()
            }));
        }
        shapes.push(caption(ui, x, 380.0, name));
    }
    let panel = svgr!(<rect x="560" y="520" width="800" height="440" rx="24" fill={c.surface} />);
    let first = card(ui, 700.0, 650.0, 240.0, 80.0, c.accent, "Open");
    let second = card(ui, 1060.0, 650.0, 240.0, 80.0, c.muted, "Share");
    let path = ui.show(cursor_path());
    svgr!(<g>{shapes}{panel}{first}{second}{path}</g>)
}

/// A button that the pointer clicks.
fn click<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let pressed = motion::progress(ui.seconds(), CLICK_AT, CLICK_AT + 0.2);
    let glow = svgr!(
        <rect x="830" y="490" width="260" height="100" rx="26" fill="none" stroke={c.highlight}
              stroke-width="4" opacity={pressed} />
    );
    let button = card(ui, 840.0, 500.0, 240.0, 80.0, c.accent, "Render");
    let click = ui.show(Click {
        x: 1000.0,
        y: 548.0,
        at: CLICK_AT,
        ..ui.defaults()
    });
    svgr!(<g>{glow}{button}{click}</g>)
}

/// A card carried to an outlined slot.
fn drag<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let slot = svgr!(
        <rect x="1060" y="440" width="320" height="200" rx="18" fill="none" stroke={c.muted}
              stroke-width="3" stroke-dasharray="14 10" />
    );
    let drag = ui.show(Drag {
        from: (700.0, 560.0),
        to: (1240.0, 560.0),
        start: DRAG.0,
        run_time: DRAG.1,
        children: children(move |ui| card(ui, 520.0, 440.0, 320.0, 200.0, c.accent, "clip.mp4")),
        ..ui.defaults()
    });
    svgr!(<g>{slot}{drag}</g>)
}

/// A text field that the I-beam clicks into before the text is typed.
fn type_at<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let field = svgr!(<rect x="560" y="490" width="800" height="100" rx="16" fill={c.surface} stroke={c.muted} stroke-width="2" />);
    let defaults: TypeAt = ui.defaults();
    let typing = ui.show(TypeAt {
        typewriter: Typewriter {
            text: "render the gallery".into(),
            x: 620.0,
            y: 553.0,
            delay: TYPE_DELAY,
            ..defaults.typewriter
        },
        ..defaults
    });
    svgr!(<g>{field}{typing}</g>)
}

/// Two emoji over dashed slots, and the credit line. The files are not in the gallery, so
/// `inspect` warns with their addresses.
fn animated_emoji<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let items = [
        ("grinning", None, 620.0),
        ("thumbs-up", Some(SkinTone::Medium), 1060.0),
    ];
    let mut parts = Vec::new();
    for (name, skin_tone, x) in items {
        parts.push(svgr!(
            <rect x={x} y="340" width="240" height="240" rx="24" fill="none" stroke={c.muted}
                  stroke-width="3" stroke-dasharray="14 10" />
        ));
        parts.push(ui.show(AnimatedEmoji {
            name,
            skin_tone,
            x: x + 20.0,
            y: 360.0,
            size: 200.0,
            start: 0.3,
            ..ui.defaults()
        }));
        let file = emoji::file_name(name, skin_tone);
        parts.push(caption(ui, x + 120.0, 630.0, file));
    }
    let credit = ui.show(Paragraph {
        text: emoji::ATTRIBUTION.into(),
        x: 360.0,
        y: 780.0,
        width: 1200.0,
        style: TextStyle {
            size: 26,
            ..ui.theme.typography.caption
        },
        fill: c.muted,
        ..ui.defaults()
    });
    svgr!(<g>{parts}{credit}</g>)
}

/// The tracks of the `SoundEffects` scene: every sound of the pack, one after another.
fn sound_effect_tracks() -> Vec<AudioTrack<'static>> {
    Sfx::PACK
        .iter()
        .enumerate()
        .flat_map(|(i, sound)| sfx::at(&[effect_time(i)], *sound))
        .collect()
}

/// A tile per sound of the pack that lights up while it plays.
fn sound_effects<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let t = ui.seconds();
    let mut tiles = Vec::new();
    for (i, sound) in Sfx::PACK.iter().enumerate() {
        let x = 150.0 + 240.0 * i as f32;
        let at = effect_time(i);
        let lit =
            motion::progress(t, at, at + 0.1) * (1.0 - motion::progress(t, at + 0.35, at + 0.6));
        tiles.push(svgr!(
            <g>
                <rect x={x} y="440" width="200" height="200" rx="24" fill={c.surface} />
                <rect x={x} y="440" width="200" height="200" rx="24" fill={c.accent} opacity={lit} />
            </g>
        ));
        tiles.push(caption(ui, x + 100.0, 690.0, sound.file()));
    }
    svgr!(<g>{tiles}</g>)
}

/// The items of the `TransitionSfx` series, for its whooshes.
fn transition_lengths() -> [SeriesLength; 5] {
    [
        SeriesLength::Sequence(PANEL),
        SeriesLength::Transition(FADE),
        SeriesLength::Sequence(PANEL),
        SeriesLength::Transition(FADE),
        SeriesLength::Sequence(PANEL),
    ]
}

/// Three panels that fade into each other, each fade with a whoosh.
fn transition_sfx<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let panel = move |background: Color, label: &'static str| {
        children(move |ui| {
            let s = ui.theme.typography.title;
            ui.show(Fill {
                background: Some(background),
                children: children(move |_| {
                    svgr!(
                        <text x="96" y="620" font-family={s.family} font-size={s.size}
                              font-weight={s.weight} fill={c.title}>{label}</text>
                    )
                }),
            })
        })
    };
    ui.show(TransitionSeries {
        key: Some("sfx"),
        items: vec![
            TransitionSeries::sequence(PANEL, panel(c.surface, "one")),
            TransitionSeries::transition(FADE, Fade::default()),
            TransitionSeries::sequence(PANEL, panel(c.accent, "two (whoosh)")),
            TransitionSeries::transition(FADE, Fade::default()),
            TransitionSeries::sequence(PANEL, panel(c.muted, "three (whoosh)")),
        ],
    })
}

/// The gallery GIF stretched, contained and covering a box of another shape.
fn gif<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let Some(bounce) = BOUNCE.as_ref() else {
        return Svgr::empty();
    };
    let c = ui.theme.colors;
    let mut parts = Vec::new();
    for (i, (fit, name)) in [
        (GifFit::Fill, "Fill"),
        (GifFit::Contain, "Contain"),
        (GifFit::Cover, "Cover"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 300.0 + 480.0 * i as f32;
        parts.push(svgr!(<rect x={x} y="360" width="360" height="240" fill="none" stroke={c.muted} stroke-width="2" />));
        parts.push(ui.show(Gif {
            gif: bounce,
            x,
            y: 360.0,
            width: 360.0,
            height: 240.0,
            fit,
            ..ui.defaults()
        }));
        parts.push(caption(ui, x + 180.0, 650.0, name));
    }
    svgr!(<g>{parts}</g>)
}
