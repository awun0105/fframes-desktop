//! Transitions: a `TransitionSeries` per presentation, the progress a sequence reads,
//! `SceneFade`, and two core scenes that slide over each other through an `Overlap`.

use fframes_kit::prelude::*;

use crate::{GalleryScene, scenes::core::title};

/// Seconds the two `SceneOverlap` scenes share; 15 frames at 30 fps.
const OVERLAP: f32 = 0.5;

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // 1.5 + 1.5 seconds of sequences minus the 0.8 second transition
        GalleryScene::new("Fade", 2.2, fade),
        GalleryScene::new("Slide", 2.2, slide),
        GalleryScene::new("Wipe", 2.2, wipe),
        GalleryScene::new("Flip", 2.2, flip),
        GalleryScene::new("ClockWipe", 2.2, clock_wipe),
        GalleryScene::new("Iris", 2.2, iris),
        GalleryScene::new("CircleWipe", 2.2, circle_wipe),
        GalleryScene::new("SlantWipe", 2.2, slant_wipe),
        GalleryScene::new("NoTransition", 2.2, no_transition),
        GalleryScene::new("PushCut", 2.2, push_cut),
        GalleryScene::new("ZoomInOut", 2.2, zoom_in_out),
        GalleryScene::new("DreamyZoom", 2.2, dreamy_zoom),
        GalleryScene::new("LinearBlur", 2.2, linear_blur),
        GalleryScene::new("BlurSlide", 2.2, blur_slide),
        GalleryScene::new("CrossZoom", 2.2, cross_zoom),
        GalleryScene::new("Dissolve", 2.2, dissolve),
        GalleryScene::new("FilmBurn", 2.2, film_burn),
        GalleryScene::new("SwapCards", 2.2, swap_cards),
        GalleryScene::new("TransitionProgress", 2.2, transition_progress),
        GalleryScene::new("SceneFade", 2.0, scene_fade),
        GalleryScene::new("BeforeOverlap", 1.5, before_overlap),
        GalleryScene::new("SceneOverlap", 1.5, scene_overlap).with_overlap(OVERLAP),
    ]
}

/// A full-canvas panel with a big label, so the transition covers what is behind it.
fn panel<'a>(ui: &mut Ui<'a, '_, '_>, background: Color, label: &'static str) -> Svgr<'a> {
    let s = ui.theme.typography.title;
    let c = ui.theme.colors;
    ui.show(Fill {
        background: Some(background),
        children: children(move |_| {
            svgr!(
                <text x="96" y="620" font-family={s.family} font-size={s.size} font-weight={s.weight} fill={c.title}>
                    {label}
                </text>
            )
        }),
    })
}

/// Two panels of 1.5 seconds with `presentation` between them.
fn series<'a>(
    ui: &mut Ui<'a, '_, '_>,
    name: &'static str,
    timing: Timing,
    presentation: impl Presentation + 'a,
) -> Svgr<'a> {
    let c = ui.theme.colors;
    let items = ui.show(TransitionSeries {
        key: Some(name),
        items: vec![
            TransitionSeries::sequence(1.5, children(move |ui| panel(ui, c.surface, "one"))),
            TransitionSeries::transition(timing, presentation),
            TransitionSeries::sequence(1.5, children(move |ui| panel(ui, c.accent, "two"))),
        ],
    });
    let head = title(ui, name);
    svgr!(<g>{items}{head}</g>)
}

fn fade<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "Fade", timing, Fade::default())
}

fn slide<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = Timing::Spring {
        mass: 1.0,
        stiffness: 200.0,
        damping: 26.0,
        duration: Some(0.8),
    };
    let slide = Slide {
        direction: Direction::FromRight,
        ..Slide::default()
    };
    series(ui, "Slide", timing, slide)
}

fn wipe<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "Wipe", timing, Wipe::default())
}

fn flip<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "Flip", timing, Flip::default())
}

fn clock_wipe<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "ClockWipe", timing, ClockWipe::default())
}

fn iris<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "Iris", timing, Iris::default())
}

fn circle_wipe<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    let wipe = CircleWipe {
        center: (0.7, 0.4),
        // the highlight colour, so the white title stays readable over the disc
        color: ui.theme.colors.highlight,
        ..ui.defaults()
    };
    series(ui, "CircleWipe", timing, wipe)
}

fn slant_wipe<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    // the back stripe in the title colour, so it shows against the accent panel
    let wipe = SlantWipe {
        back: ui.theme.colors.title,
        ..ui.defaults()
    };
    series(ui, "SlantWipe", timing, wipe)
}

fn no_transition<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "NoTransition", timing, NoTransition)
}

fn push_cut<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    // linear: a spring would cross the cut more than once
    let timing = Timing::Linear {
        duration: 0.8,
        easing: RateFunc::Linear,
    };
    series(ui, "PushCut", timing, PushCut::default())
}

fn zoom_in_out<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "ZoomInOut", timing, ZoomInOut::default())
}

fn dreamy_zoom<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    // a spring is past the white flash at the middle frame, so that frame shows the zoom
    let timing = Timing::Spring {
        mass: 1.0,
        stiffness: 120.0,
        damping: 20.0,
        duration: Some(0.8),
    };
    series(ui, "DreamyZoom", timing, DreamyZoom::default())
}

fn linear_blur<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    let blur = LinearBlur {
        axes: (1.0, 0.25),
        ..LinearBlur::default()
    };
    series(ui, "LinearBlur", timing, blur)
}

fn blur_slide<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = Timing::Linear {
        duration: 0.8,
        easing: RateFunc::Linear,
    };
    let slide = BlurSlide {
        direction: Direction::FromRight,
        ..BlurSlide::default()
    };
    series(ui, "BlurSlide", timing, slide)
}

fn cross_zoom<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    series(ui, "CrossZoom", timing, CrossZoom::default())
}

fn dissolve<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = Timing::Linear {
        duration: 0.8,
        easing: RateFunc::Linear,
    };
    series(ui, "Dissolve", timing, Dissolve::default())
}

fn film_burn<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = Timing::Linear {
        duration: 0.8,
        easing: RateFunc::Linear,
    };
    series(ui, "FilmBurn", timing, FilmBurn::default())
}

fn swap_cards<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let timing = ui.defaults();
    let swap: SwapCards = ui.defaults();
    series(ui, "SwapCards", timing, swap)
}

/// A bar per sequence that fills with how far its transition has got, under a cut that
/// leaves the whole overlap to the scenes.
fn transition_progress<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let scene = move |background: Color, label: &'static str| {
        children(move |ui| {
            let p = ui.transition_progress();
            let panel = panel(ui, background, label);
            // at least a pixel: an empty rect is invalid
            let entering = (1200.0 * p.entering).max(1.0);
            let exiting = (1200.0 * p.exiting).max(1.0);
            svgr!(
                <g>
                    {panel}
                    <rect x="96" y="700" width="1200" height="24" fill={c.muted} />
                    <rect x="96" y="700" width={entering} height="24" fill={c.title} />
                    <rect x="96" y="760" width="1200" height="24" fill={c.muted} />
                    <rect x="96" y="760" width={exiting} height="24" fill={c.highlight} />
                </g>
            )
        })
    };
    let items = ui.show(TransitionSeries {
        key: Some("TransitionProgress"),
        items: vec![
            TransitionSeries::sequence(1.5, scene(c.surface, "exiting")),
            TransitionSeries::transition(ui.defaults(), NoTransition),
            TransitionSeries::sequence(1.5, scene(c.accent, "entering")),
        ],
    });
    let head = title(ui, "TransitionProgress");
    svgr!(<g>{items}{head}</g>)
}

fn scene_fade<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let fade = SceneFade::new(
        ui,
        children(move |ui| panel(ui, c.surface, "fades at both ends")),
    );
    let content = ui.show(fade);
    let head = title(ui, "SceneFade");
    svgr!(<g>{content}{head}</g>)
}

fn overlap_timing() -> Timing {
    Timing::Linear {
        duration: OVERLAP,
        easing: RateFunc::Smooth,
    }
}

/// Pushed out to the left by `SceneOverlap` in its last `OVERLAP` seconds.
fn before_overlap<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let content = panel(ui, c.surface, "scene one");
    let slide = Slide {
        direction: Direction::FromRight,
        ..Slide::default()
    };
    scene_transition(
        ui,
        &slide,
        overlap_timing(),
        Side::Exiting,
        Some("before"),
        content,
    )
}

/// Starts `OVERLAP` seconds early (`Overlap::Previous`) and slides in from the right.
fn scene_overlap<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let panel = panel(ui, c.accent, "scene two");
    let head = title(ui, "SceneOverlap");
    let content = svgr!(<g>{panel}{head}</g>);
    let slide = Slide {
        direction: Direction::FromRight,
        ..Slide::default()
    };
    scene_transition(
        ui,
        &slide,
        overlap_timing(),
        Side::Entering,
        Some("overlap"),
        content,
    )
}
