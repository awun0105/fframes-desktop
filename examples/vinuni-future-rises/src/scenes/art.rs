//! Pieces several scenes draw: the type styles (kicker, headline, body, lines on beats),
//! rolling counters and statistics, and the dark washes that keep text readable over
//! footage; and the brand pieces: the logo, a college lockup and framed site photos.

use fframes::media::ImageData;
use fframes::{FFramesContext, Svgr};

use crate::{
    AMBER, DISPLAY, LABEL, NAVY, SANS, SKY, ease_in_out, ease_out, progress, punch, spring,
};

/// Text alignment of a block: its `text-anchor`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Align {
    Start,
    Middle,
}

impl Align {
    fn anchor(self) -> &'static str {
        match self {
            Align::Start => "start",
            Align::Middle => "middle",
        }
    }
}

/// A small label in spaced Montserrat capitals under a short amber rule, sliding in
/// from 20 px below and tracking in from `at`; the rule grows from the text's anchor.
pub(crate) fn kicker(
    text: &'static str,
    (x, y): (f32, f32),
    align: Align,
    t: f32,
    at: f32,
) -> Svgr<'static> {
    kicker_in(text, (x, y), align, t, at, AMBER)
}

/// A [`kicker`] whose rule is `accent` (a college's colour).
pub(crate) fn kicker_in(
    text: &'static str,
    (x, y): (f32, f32),
    align: Align,
    t: f32,
    at: f32,
    accent: &'static str,
) -> Svgr<'static> {
    let k = ease_out(progress(t, at, 0.5));
    let rule = 44.0 * ease_out(progress(t, at + 0.1, 0.4));
    let rule_x = match align {
        Align::Start => x,
        Align::Middle => x - rule / 2.0,
    };
    // the letters track in from wide spacing as the label rises
    let spacing = 6.0 + 14.0 * (1.0 - ease_out(progress(t, at, 0.8)));
    let y = y + (1.0 - k) * 20.0;
    fframes::svgr!(<g opacity={k}>
        <rect x={rule_x} y={y - 38.0} width={rule.max(0.01)} height="4" fill={accent} />
        <text x={x} y={y} text-anchor={align.anchor()} font-family={DISPLAY} font-weight={LABEL} font-size="20" letter-spacing={spacing} fill={SKY}>{text}</text>
    </g>)
}

/// A line of text rising on a spring from under a mask at its own baseline from `at`, so
/// it appears to come up out of the line. The mask is named after the baseline and must
/// be unique in the frame.
fn rise_line(
    text: Svgr<'static>,
    (x, baseline): (f32, f32),
    size: f32,
    t: f32,
    at: f32,
) -> Svgr<'static> {
    let k = spring(t, at, 1.0, 170.0, 19.0);
    if k <= 0.0 {
        return Svgr::empty();
    }
    let id = format!("rise-{}-{}", x.round(), baseline.round());
    let drop = (1.0 - k) * size * 1.15;
    fframes::svgr!(<g opacity={progress(t, at, 0.12)}>
        <defs><clipPath id={id.clone()}><rect x="0" y={baseline - size * 1.05} width="1920" height={size * 1.4} /></clipPath></defs>
        <g clip-path={format!("url(#{id})")}>
            <g transform={format!("translate(0 {drop})")}>{text}</g>
        </g>
    </g>)
}

/// Bold display lines, `size` px, each line rising out of its mask 0.1 s after the previous
/// from `at`.
pub(crate) fn headline(
    lines: &[&'static str],
    (x, y): (f32, f32),
    size: f32,
    align: Align,
    t: f32,
    at: f32,
) -> Svgr<'static> {
    let nodes: Vec<Svgr<'static>> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            let ly = y + i as f32 * size * 1.12;
            let text = fframes::svgr!(<text x={x} y={ly} text-anchor={align.anchor()} font-family={DISPLAY} font-weight="700" font-size={size} fill="#ffffff">{line}</text>);
            rise_line(text, (x, ly), size, t, at + i as f32 * 0.1)
        })
        .collect();
    fframes::svgr!(<g>{nodes}</g>)
}

/// A plain line of body text, rising out of its mask from `at`.
pub(crate) fn body(
    text: &'static str,
    (x, y): (f32, f32),
    size: f32,
    align: Align,
    t: f32,
    at: f32,
) -> Svgr<'static> {
    let line = fframes::svgr!(<text x={x} y={y} text-anchor={align.anchor()} font-family={SANS} font-size={size} fill={SKY}>{text}</text>);
    rise_line(line, (x, y), size, t, at)
}

/// A slot-machine counter in Montserrat Bold, every character in its own slot (digits
/// tabular, punctuation narrower): every digit rolls through whole turns and lands on its
/// value on a spring from `at`, the rightmost last; other characters stay put. `id`
/// names the clip path and must be unique in the frame.
pub(crate) fn counter(
    value: &'static str,
    id: &'static str,
    (cx, baseline): (f32, f32),
    size: f32,
    fill: &'static str,
    t: f32,
    at: f32,
) -> Svgr<'static> {
    let chars: Vec<char> = value.chars().collect();
    let slot = |c: char| {
        size * match c {
            ',' | '.' => 0.32,
            '0'..='9' => 0.72,
            _ => 0.8,
        }
    };
    let total: f32 = chars.iter().map(|&c| slot(c)).sum();
    let left = cx - total / 2.0;
    let mut nodes = Vec::new();
    let mut x0 = left;
    for (j, &c) in chars.iter().enumerate() {
        let x = x0 + slot(c) / 2.0;
        x0 += slot(c);
        let Some(target) = c.to_digit(10) else {
            nodes.push(fframes::svgr!(<text x={x} y={baseline} text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size={size} fill={fill}>{c.to_string()}</text>));
            continue;
        };
        let turns = 2.0 + j as f32 * 0.5;
        let start = at + j as f32 * 0.04;
        let position = (target as f32 + 10.0 * turns.floor()) * spring(t, start, 1.0, 12.0, 6.0);
        let base = position.floor();
        let frac = position - base;
        for k in 0..2 {
            let digit = ((base as i64 + k) % 10) as u32;
            let y = baseline + (k as f32 - frac) * size * 1.1;
            nodes.push(fframes::svgr!(<text x={x} y={y} text-anchor="middle" font-family={DISPLAY} font-weight="700" font-size={size} fill={fill}>{digit.to_string()}</text>));
        }
    }
    let width = total + 20.0;
    fframes::svgr!(<g opacity={progress(t, at - 0.1, 0.2)}>
        <defs><clipPath id={id}><rect x={left - 10.0} y={baseline - size * 0.95} width={width} height={size * 1.18} /></clipPath></defs>
        <g clip-path={format!("url(#{id})")}>{nodes}</g>
    </g>)
}

/// A dark wash over footage, deepest on the left, so left-aligned text stays readable.
pub(crate) fn wash_left(opacity: f32) -> Svgr<'static> {
    fframes::svgr!(<g opacity={opacity}>
        <defs>
            <linearGradient id="wash-left" x1="0" y1="0" x2="1" y2="0">
                <stop offset="0" stop-color={NAVY} stop-opacity="0.92" />
                <stop offset="0.6" stop-color={NAVY} stop-opacity="0.35" />
                <stop offset="1" stop-color={NAVY} stop-opacity="0.1" />
            </linearGradient>
        </defs>
        <rect width="1920" height="1080" fill="url(#wash-left)" />
    </g>)
}

/// A dark wash over footage, deepest at the bottom, for centred captions.
pub(crate) fn wash_bottom(opacity: f32) -> Svgr<'static> {
    fframes::svgr!(<g opacity={opacity}>
        <defs>
            <linearGradient id="wash-bottom" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color={NAVY} stop-opacity="0.15" />
                <stop offset="0.55" stop-color={NAVY} stop-opacity="0.35" />
                <stop offset="1" stop-color={NAVY} stop-opacity="0.92" />
            </linearGradient>
        </defs>
        <rect width="1920" height="1080" fill="url(#wash-bottom)" />
    </g>)
}

/// A labelled statistic: a rolling `counter` with a small spaced label under it.
pub(crate) fn stat(
    (value, label): (&'static str, &'static str),
    id: &'static str,
    (cx, baseline): (f32, f32),
    size: f32,
    fill: &'static str,
    t: f32,
    at: f32,
) -> Svgr<'static> {
    let k = ease_out(progress(t, at + 0.3, 0.5));
    fframes::svgr!(<g>
        {counter(value, id, (cx, baseline), size, fill, t, at)}
        <text x={cx} y={baseline + 44.0} text-anchor="middle" font-family={DISPLAY} font-weight={LABEL} font-size="17" letter-spacing="4" fill={SKY} opacity={k}>{label}</text>
    </g>)
}

/// Display lines like [`headline`], each rising in at its own second.
pub(crate) fn lines_at(
    lines: &[(&'static str, f32)],
    (x, y): (f32, f32),
    size: f32,
    align: Align,
    t: f32,
) -> Svgr<'static> {
    let nodes: Vec<Svgr<'static>> = lines
        .iter()
        .enumerate()
        .map(|(i, &(line, at))| {
            headline(&[line], (x, y + i as f32 * size * 1.12), size, align, t, at)
        })
        .collect();
    fframes::svgr!(<g>{nodes}</g>)
}

/// Width over height of `logo.png`, the college sites' header logo (711x153).
const LOGO_RATIO: f32 = 711.0 / 153.0;
/// Width over height of the college marks (832x668).
const MARK_RATIO: f32 = 832.0 / 668.0;

/// The `VinUniversity` logo, `height` px tall with its left edge at `x`, centred on `cy`.
/// Its wordmark is black, so it only goes on white.
fn logo<'a>(ctx: &FFramesContext<'a, '_>, (x, cy): (f32, f32), height: f32) -> Svgr<'a> {
    let Some(image) = ctx.get_image("logo.png") else {
        return Svgr::empty();
    };
    fframes::svgr!(<image href={image.href()} x={x} y={cy - height / 2.0} width={height * LOGO_RATIO} height={height} />)
}

/// The logo centred on `(cx, cy)` on a white plate, for footage: the sites publish no
/// reversed logo.
pub(crate) fn logo_plate<'a>(
    ctx: &FFramesContext<'a, '_>,
    (cx, cy): (f32, f32),
    height: f32,
) -> Svgr<'a> {
    let pad = height * 0.4;
    let width = height * LOGO_RATIO;
    fframes::svgr!(<g>
        <rect x={cx - width / 2.0 - pad} y={cy - height / 2.0 - pad} width={width + 2.0 * pad} height={height + 2.0 * pad} fill="#ffffff" />
        {logo(ctx, (cx - width / 2.0, cy), height)}
    </g>)
}

/// A college's mark from its site, the faceted triangle in its colour, `height` px tall
/// with its bottom edge from `x` at `bottom`; the right angle is bottom left, or bottom
/// right when `mirror`.
fn mark<'a>(
    ctx: &FFramesContext<'a, '_>,
    file: &str,
    (x, bottom): (f32, f32),
    height: f32,
    mirror: bool,
) -> Svgr<'a> {
    let Some(image) = ctx.get_image(file) else {
        return Svgr::empty();
    };
    let width = height * MARK_RATIO;
    let (left, flip) = if mirror { (x + width, -1.0) } else { (x, 1.0) };
    fframes::svgr!(<g transform={format!("translate({left} {}) scale({flip} 1)", bottom - height)}>
        <image href={image.href()} x="0" y="0" width={width} height={height} />
    </g>)
}

/// The size of an image fitted by area: about `area` px², within `(max_w, max_h)`, so wide
/// wordmarks and square crests carry a similar weight.
fn fit_by_area(image: &ImageData<'_>, area: f32, (max_w, max_h): (f32, f32)) -> (f32, f32) {
    let ratio = image.metadata.width as f32 / image.metadata.height.max(1) as f32;
    let h = (area / ratio).sqrt();
    let scale = (max_w / (h * ratio)).min(max_h / h).min(1.0);
    (h * ratio * scale, h * scale)
}

/// A college's lockup (`tools/fetch-media.py` builds all four in one layout and one size:
/// the `VinUniversity` logo with the college's name line), at one height on a white plate
/// with an edge in the college's colour and its mark sliding in at the right. `(color, mark, lockup)` are the college's colour, mark file and lockup
/// file. The plate opens from its left edge from `at`; `(x, y)` is its top left corner.
pub(crate) fn lockup<'a>(
    ctx: &FFramesContext<'a, '_>,
    (color, mark_file, lockup_file): (&'static str, &'static str, &'static str),
    (x, y): (f32, f32),
    t: f32,
    at: f32,
) -> Svgr<'a> {
    const PAD: f32 = 34.0;
    const H: f32 = 136.0;
    let Some(image) = ctx.get_image(lockup_file) else {
        return Svgr::empty();
    };
    let (w, h) = (
        H * image.metadata.width as f32 / image.metadata.height.max(1) as f32,
        H,
    );
    let height = h + 2.0 * PAD;
    let mark_h = height * 0.62;
    let width = PAD + 8.0 + w + PAD + mark_h * MARK_RATIO;
    let open = ease_out(progress(t, at, 0.6));
    let shown = ease_out(progress(t, at + 0.25, 0.5));
    fframes::svgr!(<g opacity={progress(t, at, 0.15)}>
        <defs><clipPath id="lockup-open"><rect x={x} y={y} width={(width * open).max(0.01)} height={height} /></clipPath></defs>
        <g clip-path="url(#lockup-open)">
            <rect x={x} y={y} width={width} height={height} fill="#ffffff" />
            <rect x={x} y={y} width="8" height={height} fill={color} />
            <g opacity={shown} transform={format!("translate({} 0)", (1.0 - shown) * -24.0)}>
                <image href={image.href()} x={x + 8.0 + PAD} y={y + PAD} width={w} height={h} />
            </g>
            <g transform={format!("translate({} 0)", (1.0 - ease_out(progress(t, at + 0.45, 0.5))) * mark_h * MARK_RATIO)}>
                {mark(ctx, mark_file, (x + width - mark_h * MARK_RATIO, y + height), mark_h, true)}
            </g>
        </g>
    </g>)
}

/// Logos from `dynamic_media/` on white chips (`CHIP`), centred rows of `per_row` from
/// `top`, each fitted by area. They pop in one per `step` from `at` and fade out over the
/// 0.25 s before `until`.
pub(crate) fn logo_wall<'a>(
    ctx: &FFramesContext<'a, '_>,
    files: &[&'static str],
    per_row: usize,
    top: f32,
    (t, at, step, until): (f32, f32, f32, f32),
) -> Svgr<'a> {
    const GAP: f32 = 24.0;
    let out = 1.0 - progress(t, until - 0.25, 0.25);
    if out <= 0.0 || t < at {
        return Svgr::empty();
    }
    let mut nodes = Vec::new();
    for (row, chunk) in files.chunks(per_row).enumerate() {
        let total = chunk.len() as f32 * (CHIP.0 + GAP) - GAP;
        for (j, &file) in chunk.iter().enumerate() {
            let cx = 960.0 - total / 2.0 + j as f32 * (CHIP.0 + GAP) + CHIP.0 / 2.0;
            let cy = top + row as f32 * (CHIP.1 + GAP) + CHIP.1 / 2.0;
            let start = at + (row * per_row + j) as f32 * step;
            nodes.push(logo_chip(ctx, file, (cx, cy), t, start, out));
        }
    }
    fframes::svgr!(<g>{nodes}</g>)
}

/// The size of a logo chip.
pub(crate) const CHIP: (f32, f32) = (300.0, 124.0);

/// One logo on a white [`CHIP`] centred on `(cx, cy)`, fitted by area, popping up into
/// place from `at`; `out` is the opacity it fades out with.
pub(crate) fn logo_chip<'a>(
    ctx: &FFramesContext<'a, '_>,
    file: &'static str,
    (cx, cy): (f32, f32),
    t: f32,
    at: f32,
    out: f32,
) -> Svgr<'a> {
    let k = punch(t, at);
    let Some(image) = ctx.get_image(file).filter(|_| k > 0.0 && out > 0.0) else {
        return Svgr::empty();
    };
    let (w, h) = fit_by_area(image, 17_000.0, (CHIP.0 - 36.0, CHIP.1 - 30.0));
    let cy = cy + (1.0 - k) * 40.0;
    fframes::svgr!(<g opacity={progress(t, at, 0.12) * out} transform={format!("translate({cx} {cy}) scale({})", 0.6 + 0.4 * k)}>
        <rect x={-CHIP.0 / 2.0 + 8.0} y={-CHIP.1 / 2.0 + 10.0} width={CHIP.0} height={CHIP.1} rx="14" fill={NAVY} opacity="0.4" />
        <rect x={-CHIP.0 / 2.0} y={-CHIP.1 / 2.0} width={CHIP.0} height={CHIP.1} rx="14" fill="#ffffff" />
        <image href={image.href()} x={-w / 2.0} y={-h / 2.0} width={w} height={h} />
    </g>)
}

/// A site photo on a white card with a caption strip: `size` is the photo's, `center`
/// the card's, tilted by `tilt` degrees. It springs in from `at` and fades out over the
/// 0.25 s before `until`; `id` names the clip path and must be unique in the frame. The
/// caption's bar is `accent`; with `logo` the strip ends in the `VinUniversity` logo (keep
/// the caption short), with `mark` a college's mark sits in the photo's bottom left corner.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Card {
    pub file: &'static str,
    pub id: &'static str,
    pub caption: &'static str,
    pub center: (f32, f32),
    pub size: (f32, f32),
    pub tilt: f32,
    pub at: f32,
    pub until: f32,
    pub accent: &'static str,
    pub logo: bool,
    pub mark: Option<&'static str>,
}

impl Card {
    pub(crate) fn draw<'a>(&self, ctx: &FFramesContext<'a, '_>, t: f32) -> Svgr<'a> {
        const BORDER: f32 = 10.0;
        const STRIP: f32 = 50.0;
        let shown = progress(t, self.at, 0.2) * (1.0 - progress(t, self.until - 0.25, 0.25));
        if shown <= 0.0 {
            return Svgr::empty();
        }
        let Some(image) = ctx.get_image(self.file) else {
            return Svgr::empty();
        };
        let ((w, h), (cx, cy)) = (self.size, self.center);
        let pop = punch(t, self.at);
        let scale = 0.82 + 0.18 * pop;
        // it turns towards the viewer like a card being flipped over
        let flip = 0.05 + 0.95 * ease_out(progress(t, self.at, 0.4));
        let tilt = self.tilt * (0.4 + 0.6 * pop);
        let push = 1.0 + 0.015 * (t - self.at).max(0.0);
        let (fw, fh) = (w + 2.0 * BORDER, h + 2.0 * BORDER + STRIP);
        let (left, top) = (-fw / 2.0, -fh / 2.0);
        let (px, py) = (left + BORDER, top + BORDER);
        // once it has landed, a band of light sweeps across the photo
        let sweep = progress(t, self.at + 0.35, 0.7);
        let shine_x = px - 140.0 + (w + 280.0) * ease_in_out(sweep);
        let shine = if sweep > 0.0 && sweep < 1.0 {
            fframes::svgr!(<rect x={shine_x} y={py - 60.0} width="70" height={h + 120.0} fill="#ffffff" opacity={0.28 * (std::f32::consts::PI * sweep).sin()} transform={format!("rotate(18 {shine_x} {})", py + h / 2.0)} />)
        } else {
            Svgr::empty()
        };
        fframes::svgr!(<g opacity={shown} transform={format!("translate({cx} {}) rotate({tilt}) scale({} {scale})", cy + (1.0 - pop) * 60.0, scale * flip)}>
            <rect x={left + 12.0} y={top + 18.0} width={fw} height={fh} fill={NAVY} opacity="0.45" />
            <rect x={left} y={top} width={fw} height={fh} fill="#ffffff" />
            <defs><clipPath id={self.id}><rect x={px} y={py} width={w} height={h} /></clipPath></defs>
            <g clip-path={format!("url(#{})", self.id)}>
                <g transform={format!("translate({} {}) scale({push}) translate({} {})", px + w / 2.0, py + h / 2.0, -w / 2.0, -h / 2.0)}>
                    <image href={image.href()} x="0" y="0" width={w} height={h} preserveAspectRatio="xMidYMid slice" />
                </g>
                {shine}
            </g>
            {self.mark.map_or_else(Svgr::empty, |file| mark(ctx, file, (px, py + h), h * 0.4, false))}
            <rect x={px} y={py + h + 13.0} width="4" height="24" fill={self.accent} />
            <text x={px + 16.0} y={py + h + 32.0} font-family={DISPLAY} font-weight={LABEL} font-size="17" letter-spacing="2.5" fill={NAVY}>{self.caption}</text>
            {if self.logo { logo(ctx, (px + w - 24.0 * LOGO_RATIO, py + h + 25.0), 24.0) } else { Svgr::empty() }}
        </g>)
    }
}
