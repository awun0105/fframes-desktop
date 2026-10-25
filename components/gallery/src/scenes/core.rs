//! Time containers, fades, the theme and the cost bench.

use std::fmt::Write;

use fframes_kit::prelude::*;

use crate::GalleryScene;

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("Sequence", 3.0, sequence),
        GalleryScene::new("Series", 3.0, series),
        GalleryScene::new("Loop", 3.0, looping),
        GalleryScene::new("Freeze", 3.0, freeze),
        GalleryScene::new("FadeIn", 2.0, fade_in),
        GalleryScene::new("FadeOut", 2.0, fade_out),
        GalleryScene::new("Theme", 2.0, theme),
        GalleryScene::new("BenchHandWritten", 5.0, bench_hand_written),
        GalleryScene::new("BenchKit", 5.0, bench_kit),
    ]
}

/// A filter with the id `id` that lays a blur of `blur` pixels under what it filters, the
/// neon glow of the showpiece scenes. Use it as `filter="url(#id)"` on a group that is never
/// empty: an empty group has no box to filter.
pub(crate) fn neon<'a>(id: &'static str, blur: f32) -> Svgr<'a> {
    svgr!(
        <filter id={id} x="-25%" y="-25%" width="150%" height="150%">
            <feGaussianBlur in="SourceGraphic" stdDeviation={blur} result="blur" />
            <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
            </feMerge>
        </filter>
    )
}

/// A bright colour `turn` of the way round the colour wheel (0 red, 1/3 green, 2/3 blue).
pub(crate) fn hue(turn: f32) -> Color {
    let channel = |offset: f32| {
        let k = (turn * 6.0 + offset).rem_euclid(6.0);
        let v = 1.0 - (k.min(4.0 - k).clamp(0.0, 1.0));
        // a pastel neon: saturated but never dark
        (255.0 * (0.35 + 0.65 * v)) as u8
    };
    Color::rgb(channel(5.0), channel(3.0), channel(1.0))
}

/// The scene's title in the top left corner.
pub(crate) fn title<'a>(ui: &Ui<'a, '_, '_>, text: &'static str) -> Svgr<'a> {
    let c = ui.theme.colors;
    let s = ui.theme.typography.heading;
    svgr!(
        <text x="96" y="140" font-family={s.family} font-size={s.size} font-weight={s.weight} fill={c.title}>
            {text}
        </text>
    )
}

/// A labelled track whose bar shows how far the local time is through `ui.duration`.
fn track<'a>(ui: &Ui<'a, '_, '_>, y: f32, label: &'static str) -> Svgr<'a> {
    let c = ui.theme.colors;
    let s = ui.theme.typography.caption;
    let p = ui
        .duration_seconds()
        .map_or(0.0, |d| motion::progress(ui.seconds(), 0.0, d));
    let clock = format!("{label}  t = {:.2}s", ui.seconds());
    svgr!(
        <g transform={Transform::translate(96, y)}>
            <rect x="0" y="0" width="1200" height="24" rx="12" fill={c.surface} />
            <rect x="0" y="0" width={24.0 + 1176.0 * p} height="24" rx="12" fill={c.accent} />
            <text x="1240" y="21" font-family={s.family} font-size={s.size} font-weight={s.weight} fill={c.text}>
                {clock}
            </text>
        </g>
    )
}

fn sequence<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Sequence");
    let scene = track(ui, 300.0, "scene");
    let outer = ui.show(Sequence {
        from: 0.5,
        duration: Some(2.0),
        children: children(|ui| {
            let own = track(ui, 420.0, "from 0.5s");
            let inner = ui.show(Sequence {
                from: 0.5,
                duration: None,
                children: children(|ui| track(ui, 540.0, "nested, from 0.5s")),
            });
            svgr!(<g>{own}{inner}</g>)
        }),
    });
    svgr!(<g>{head}{scene}{outer}</g>)
}

fn card<'a>(ui: &Ui<'a, '_, '_>, x: f32, label: &'static str, fill: Color) -> Svgr<'a> {
    let r = ui.theme.radius.card;
    let s = ui.theme.typography.title;
    let c = ui.theme.colors;
    svgr!(
        <g transform={Transform::translate(x, 360)}>
            <rect x="0" y="0" width="480" height="360" rx={r} fill={fill} />
            <text x="40" y="200" font-family={s.family} font-size={s.size} font-weight={s.weight} fill={c.title}>
                {label}
            </text>
        </g>
    )
}

fn series<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Series");
    let c = ui.theme.colors;
    let items = ui.show(Series {
        items: vec![
            (1.0, children(move |ui| card(ui, 96.0, "one", c.surface))),
            (1.0, children(move |ui| card(ui, 720.0, "two", c.accent))),
            (1.0, children(move |ui| card(ui, 1344.0, "three", c.muted))),
        ],
        overlap: 0.0,
    });
    svgr!(<g>{head}{items}</g>)
}

fn looping<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Loop");
    let scene = track(ui, 300.0, "scene");
    let lap = ui.show(Loop {
        duration: 1.0,
        times: None,
        children: children(|ui| track(ui, 420.0, "every 1s")),
    });
    let twice = ui.show(Loop {
        duration: 1.0,
        times: Some(2),
        children: children(|ui| track(ui, 540.0, "1s, twice")),
    });
    svgr!(<g>{head}{scene}{lap}{twice}</g>)
}

fn freeze<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Freeze");
    let scene = track(ui, 300.0, "scene");
    let frozen = ui.show(Freeze {
        at: 0.75,
        children: children(|ui| track(ui, 420.0, "frozen at 0.75s")),
    });
    svgr!(<g>{head}{scene}{frozen}</g>)
}

fn fade_in<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "FadeIn");
    let cards = fade_in_cards(ui);
    svgr!(<g>{head}{cards}</g>)
}

/// The three cards of `FadeIn`, fading in one after another.
pub(crate) fn fade_in_cards<'a>(ui: &mut Ui<'a, '_, '_>) -> Vec<Svgr<'a>> {
    let c = ui.theme.colors;
    let step = ui.theme.motion.stagger.loose * 2.0;
    [
        (96.0, "one", c.surface),
        (720.0, "two", c.accent),
        (1344.0, "three", c.muted),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, label, fill))| {
        let fade = FadeIn {
            shift: (0.0, -60.0),
            delay: 0.2 + motion::stagger(i, step),
            ..ui.defaults()
        };
        fade.apply(ui, card(ui, x, label, fill))
    })
    .collect()
}

fn fade_out<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "FadeOut");
    let c = ui.theme.colors;
    let step = ui.theme.motion.stagger.loose * 2.0;
    let cards: Vec<Svgr<'a>> = [
        (96.0, "one", c.surface),
        (720.0, "two", c.accent),
        (1344.0, "three", c.muted),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, label, fill))| {
        let fade = FadeOut {
            shift: (0.0, 60.0),
            delay: 0.2 + motion::stagger(2 - i, step),
            ..ui.defaults()
        };
        fade.apply(ui, card(ui, x, label, fill))
    })
    .collect();
    svgr!(<g>{head}{cards}</g>)
}

fn theme<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let background = ui.theme.colors.background;
    ui.show(Fill {
        background: Some(background),
        children: children(swatches),
    })
}

/// Every token of the theme.
fn swatches<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Theme");
    let t = ui.theme;
    let c = t.colors;
    let cap = t.typography.caption;

    let colors: Vec<Svgr<'a>> = [
        ("background", c.background),
        ("surface", c.surface),
        ("text", c.text),
        ("title", c.title),
        ("muted", c.muted),
        ("accent", c.accent),
        ("highlight", c.highlight),
        ("caption", c.caption),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (name, fill))| {
        svgr!(
            <g transform={Transform::translate(96.0 + i as f32 * 216.0, 200)}>
                <rect x="0" y="0" width="192" height="120" rx={t.radius.md} fill={fill} stroke={c.muted} stroke-width={t.stroke.divider} />
                <text x="0" y="160" font-family={cap.family} font-size={cap.size} font-weight={cap.weight} fill={c.text}>{name}</text>
            </g>
        )
    })
    .collect();

    let type_scale: Vec<Svgr<'a>> = [
        ("title", t.typography.title, 470.0),
        ("heading", t.typography.heading, 560.0),
        ("body", t.typography.body, 620.0),
        ("caption", t.typography.caption, 670.0),
    ]
    .into_iter()
    .map(|(name, s, y)| {
        svgr!(
            <text x="96" y={y} font-family={s.family} font-size={s.size} font-weight={s.weight} fill={c.title}>{name}</text>
        )
    })
    .collect();

    let sp = t.spacing;
    let spacing: Vec<Svgr<'a>> = [sp.xs, sp.sm, sp.md, sp.lg, sp.xl, sp.gutter, sp.margin]
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            svgr!(<rect x="1000" y={420.0 + i as f32 * 28.0} width={w * 4.0} height="16" fill={c.accent} />)
        })
        .collect();

    let r = t.radius;
    let radii: Vec<Svgr<'a>> = [r.sm, r.md, r.lg, r.card]
        .into_iter()
        .enumerate()
        .map(|(i, rx)| {
            svgr!(<rect x={1000.0 + i as f32 * 150.0} y="660" width="120" height="120" rx={rx} fill={c.surface} />)
        })
        .collect();

    let s = t.stroke;
    let strokes: Vec<Svgr<'a>> = [s.thin, s.regular, s.thick, s.divider]
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            svgr!(<line x1="96" x2="700" y1={760.0 + i as f32 * 36.0} y2={760.0 + i as f32 * 36.0} stroke={c.text} stroke-width={w} />)
        })
        .collect();

    let d = t.motion.duration;
    let durations: Vec<Svgr<'a>> = [d.fast, d.base, d.slow, d.enter, d.exit]
        .into_iter()
        .enumerate()
        .map(|(i, secs)| {
            svgr!(<rect x="1000" y={840.0 + i as f32 * 28.0} width={secs * 600.0} height="16" rx="8" fill={c.highlight} />)
        })
        .collect();

    let card = shadowed(ui, "card", t.shadow.card, 1640.0, 660.0);
    let glow = shadowed(ui, "glow", t.shadow.glow, 1640.0, 860.0);

    let e = t.motion.easing;
    let easings: Vec<Svgr<'a>> = [e.standard, e.enter, e.exit, e.emphasis]
        .into_iter()
        .enumerate()
        .map(|(i, rate)| {
            let mut points = String::new();
            for k in 0..=24 {
                let x = k as f32 / 24.0;
                let y = 120.0 - rate.apply(x, 1.0) * 120.0;
                let _ = write!(points, "{},{y} ", x * 120.0);
            }
            svgr!(
                <g transform={Transform::translate(96.0 + i as f32 * 150.0, 910)}>
                    <rect x="0" y="0" width="120" height="120" fill={c.surface} />
                    <polyline points={points} fill="none" stroke={c.accent} stroke-width={t.stroke.thin} />
                </g>
            )
        })
        .collect();

    let st = t.motion.stagger;
    let staggers: Vec<Svgr<'a>> = [st.tight, st.loose]
        .into_iter()
        .enumerate()
        .flat_map(|(row, step)| {
            (0..5).map(move |i| {
                let x = 1008.0 + motion::stagger(i, step) * 1000.0;
                let y = 1000.0 + row as f32 * 32.0;
                svgr!(<circle cx={x} cy={y} r="8" fill={c.highlight} />)
            })
        })
        .collect();

    svgr!(<g>{head}{colors}{type_scale}{spacing}{radii}{strokes}{durations}{card}{glow}{easings}{staggers}</g>)
}

/// The bench card: an accent bar, a name and a role, as a lower third.
const BENCH_NAME: &str = "Ada Lovelace";
const BENCH_ROLE: &str = "Mathematician";

fn bench_bar<'a>(c: Colors) -> Svgr<'a> {
    svgr!(<rect x="96" y="760" width="12" height="200" rx="6" fill={c.accent} />)
}

fn bench_name<'a>(c: Colors) -> Svgr<'a> {
    svgr!(<text x="140" y="850" font-family="DM Sans" font-size="96" font-weight="700" fill={c.title}>{BENCH_NAME}</text>)
}

fn bench_role<'a>(c: Colors) -> Svgr<'a> {
    svgr!(<text x="140" y="930" font-family="DM Sans" font-size="48" font-weight="400" fill={c.muted}>{BENCH_ROLE}</text>)
}

/// The card with three staggered fades in plain `svgr!` and `timeline!`.
fn bench_hand_written<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let frame = &ui.frame;
    let fade = |start: f32| {
        let opacity = frame.animate(&timeline!(
            at start => start + 0.6, animate 0.0_f32 => 1.0, Easing::EaseInOut
        ));
        let rise = frame.animate(&timeline!(
            at start => start + 0.6, animate 30.0_f32 => 0.0, Easing::EaseInOut
        ));
        (opacity, Transform::translate(0, rise))
    };
    let (o1, t1) = fade(0.0);
    let (o2, t2) = fade(0.1);
    let (o3, t3) = fade(0.2);
    let extra = bench_extra_hand_written(ui);
    svgr!(
        <g>
            <g opacity={o1} transform={t1}>{bench_bar(c)}</g>
            <g opacity={o2} transform={t2}>{bench_name(c)}</g>
            <g opacity={o3} transform={t3}>{bench_role(c)}</g>
            {extra}
        </g>
    )
}

/// The same card with `Sequence` and `FadeIn`.
pub(crate) fn bench_kit<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let fade = FadeIn {
        shift: (0.0, -30.0),
        run_time: 0.6,
        rate_func: RateFunc::Easing(Easing::EaseInOut),
        ..ui.defaults()
    };
    let parts: [fn(Colors) -> Svgr<'a>; 3] = [bench_bar, bench_name, bench_role];
    let card: Svgr<'a> = parts
        .into_iter()
        .enumerate()
        .map(|(i, part)| {
            ui.show(Sequence {
                from: motion::stagger(i, 0.1),
                duration: None,
                children: children(move |ui| fade.apply(ui, part(c))),
            })
        })
        .collect();
    let extra = bench_extra_kit(ui);
    svgr!(<g>{card}{extra}</g>)
}

/// A lower third and a counter next to the card, in plain `svgr!` and `timeline!`: the
/// same work `LowerThird` and `Counter` do in `BenchKit`.
fn bench_extra_hand_written<'a>(ui: &Ui<'a, '_, '_>) -> Svgr<'a> {
    let c = ui.theme.colors;
    let frame = &ui.frame;
    let ramp = |start: f32, len: f32| {
        frame
            .animate(&timeline!(at start => start + len, animate 0.0_f32 => 1.0, Easing::EaseInOut))
    };
    let shown = |i: f32| ramp(i * 0.12, 0.6) * (1.0 - ramp(5.0 - (2.0 - i) * 0.06 - 0.4, 0.4));
    let (bar, name, role) = (shown(0.0), shown(1.0), shown(2.0));
    // a rectangle without height is invalid SVG, so the bar is left out until it grows
    let accent_bar = if bar > 0.0 {
        let bar_height = 140.4 * bar;
        svgr!(<rect x="1100" y="760" width="6" height={bar_height} fill={c.accent} />)
    } else {
        Svgr::empty()
    };
    let slide = |p: f32| Transform::translate(-40.0 * (1.0 - p), 0.0);
    let (name_t, role_t) = (slide(name), slide(role));
    let digits = ((12_480.0 * ramp(0.0, 1.6)).round() as u64).to_string();
    let mut count = String::new();
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            count.push(',');
        }
        count.push(d);
    }
    svgr!(
        <g>
            {accent_bar}
            <g transform={name_t} opacity={name}>
                <text x="1122" y="824" font-family="DM Sans" font-size="56" font-weight="600" fill={c.title}>"Grace Hopper"</text>
            </g>
            <g transform={role_t} opacity={role}>
                <text x="1122" y="874.4" font-family="DM Sans" font-size="36" font-weight="400" fill={c.muted}>"Rear admiral"</text>
            </g>
            <text x="1800" y="400" text-anchor="end" font-family="DM Sans" font-size="96" font-weight="700" fill={c.title}>{count}</text>
        </g>
    )
}

/// `LowerThird` and `Counter`, the kit half of the bench's second part.
fn bench_extra_kit<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let lower = ui.show(LowerThird {
        name: "Grace Hopper".into(),
        role: "Rear admiral".into(),
        x: 1100.0,
        y: 760.0,
        ..ui.defaults()
    });
    let counter = ui.show(Counter {
        to: 12_480.0,
        x: 1800.0,
        y: 400.0,
        ..ui.defaults()
    });
    svgr!(<g>{lower}{counter}</g>)
}

/// A surface card at (x, y) with a drop shadow, its filter id built from `key`.
fn shadowed<'a>(ui: &Ui<'a, '_, '_>, key: &str, shadow: Shadow, x: f32, y: f32) -> Svgr<'a> {
    let c = ui.theme.colors;
    let filter = ui.id(Some(key), 0, "shadow");
    let filter_url = format!("url(#{filter})");
    svgr!(
        <g>
            <defs>
                <filter id={filter} x="-50%" y="-50%" width="200%" height="200%">
                    <feDropShadow dx={shadow.dx} dy={shadow.dy} stdDeviation={shadow.blur} flood-color={shadow.color} />
                </filter>
            </defs>
            <rect x={x} y={y} width="180" height="120" rx={ui.theme.radius.card} fill={c.surface} filter={filter_url} />
        </g>
    )
}
