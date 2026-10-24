//! Effects: trails and camera motion blur on moving shapes, a light leak, a focus pull with
//! `Blur`, film grain, and the bench card behind a trail to measure what the copies cost.

use std::f32::consts::TAU;

use fframes_kit::{motion, prelude::*};

use crate::{
    GalleryScene,
    scenes::core::{bench_kit, title},
};

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the ball crosses from 0.4 s to 4 s
        GalleryScene::new("Trail", 4.5, trail),
        // the bar turns 300° a second
        GalleryScene::new("CameraMotionBlur", 2.5, camera_motion_blur),
        // the light flares up and fades over the whole scene
        GalleryScene::new("LightLeak", 3.0, light_leak),
        // the word comes into focus from 0.3 s to 1.5 s
        GalleryScene::new("Blur", 2.0, blur),
        GalleryScene::new("Noise", 1.5, noise),
    ]
}

/// The bench card behind an eight-copy trail and the three area blurs, against `BenchKit` for
/// the cost of the copies and the filters.
pub fn bench_scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("BenchTrail", 5.0, bench_trail),
        GalleryScene::new("BenchRegionBlur", 5.0, bench_region_blur),
        GalleryScene::new(
            "BenchLinearProgressiveBlur",
            5.0,
            bench_linear_progressive_blur,
        ),
        GalleryScene::new(
            "BenchRadialProgressiveBlur",
            5.0,
            bench_radial_progressive_blur,
        ),
    ]
}

/// A ball on a wave across the screen.
fn ball<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let p = motion::cubic_in_out(motion::progress(ui.seconds(), 0.4, 4.0));
    let x = 220.0 + 1480.0 * p;
    let y = 560.0 + 220.0 * (p * TAU).sin();
    svgr!(<circle cx={x} cy={y} r="44" fill={c.accent} />)
}

fn trail<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Trail");
    let figure = ui.show(Trail {
        samples: 8,
        spacing: 0.06,
        children: children(ball),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// A bar turning about the middle and a square sliding under it.
fn spinner<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let t = ui.seconds();
    let turn = Transform::rotate(300.0 * t);
    let x = 260.0 + 1400.0 * motion::cubic_in_out(motion::progress(t, 0.3, 2.2));
    svgr!(
        <g>
            <g transform="translate(960 480)">
                <g transform={turn}>
                    <rect x="-260" y="-22" width="520" height="44" rx="22" fill={c.accent} />
                </g>
            </g>
            <rect x={x - 60.0} y="780" width="120" height="120" rx="16" fill={c.highlight} />
        </g>
    )
}

fn camera_motion_blur<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "CameraMotionBlur");
    let figure = ui.show(CameraMotionBlur {
        shutter_angle: 360.0,
        children: children(spinner),
        ..ui.defaults()
    });
    svgr!(<g>{figure}{head}</g>)
}

/// A dark panel with a word, for the light to fall on.
fn picture<'a>(ui: &mut Ui<'a, '_, '_>, word: &'static str) -> Svgr<'a> {
    let c = ui.theme.colors;
    let s = ui.theme.typography.title;
    svgr!(
        <g>
            <rect x="0" y="0" width="1920" height="1080" fill={c.surface} />
            <text x="960" y="600" text-anchor="middle" font-family={s.family}
                  font-size={s.size * 2} font-weight={s.weight} fill={c.title}>
                {word}
            </text>
        </g>
    )
}

fn light_leak<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let picture = picture(ui, "light leak");
    let leak = ui.show(LightLeak {
        seed: 3,
        ..ui.defaults()
    });
    let head = title(ui, "LightLeak");
    svgr!(<g>{picture}{leak}{head}</g>)
}

fn blur<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let focus = motion::cubic_in_out(motion::progress(ui.seconds(), 0.3, 1.5));
    let figure = ui.show(Blur {
        radius: Anim::Fixed(18.0 * (1.0 - focus)),
        children: children(|ui| picture(ui, "in focus")),
        ..ui.defaults()
    });
    let head = title(ui, "Blur");
    svgr!(<g>{figure}{head}</g>)
}

fn noise<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let picture = picture(ui, "grain");
    let grain = ui.show(Noise {
        opacity: 0.3,
        ..ui.defaults()
    });
    let head = title(ui, "Noise");
    svgr!(<g>{picture}{grain}{head}</g>)
}

fn bench_trail<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Trail {
        samples: 8,
        children: children(bench_kit),
        ..ui.defaults()
    })
}

fn bench_region_blur<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(RegionBlur {
        children: children(bench_kit),
        ..ui.defaults()
    })
}

fn bench_linear_progressive_blur<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(LinearProgressiveBlur {
        children: children(bench_kit),
        ..ui.defaults()
    })
}

fn bench_radial_progressive_blur<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(RadialProgressiveBlur {
        children: children(bench_kit),
        ..ui.defaults()
    })
}
