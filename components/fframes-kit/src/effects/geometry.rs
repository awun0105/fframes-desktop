use fframes::Svgr;

use super::{patterns::draw_over, textures::MAX_TILES};
use crate::{
    anim::Anim,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// `content` cut to the `w` by `h` area from the origin, like Remotion's canvas.
fn in_area<'a>(content: Svgr<'a>, id: &str, area: (f32, f32)) -> Svgr<'a> {
    draw_over(Svgr::empty(), content, None, id, area)
}

/// How far a copy reaches under its neighbour, so that the two edges do not leave a seam
/// where their anti-aliasing meets: a two-hundredth of the area's longer side, about a pixel
/// in a thumbnail of it.
fn bleed((w, h): (f32, f32)) -> f32 {
    w.max(h) / 200.0
}

/// Which way [`Mirror`] reflects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MirrorDirection {
    /// About a vertical line: the left and right sides match.
    #[default]
    Horizontal,
    /// About a horizontal line: the top and bottom match.
    Vertical,
}

/// Its children reflected about a line (Remotion's `mirror()`): the part past `position`
/// (0 to 1 across the area; the right, or the bottom with [`MirrorDirection::Vertical`])
/// shows the other side mirrored, or the other way round with `invert`. Where the mirrored
/// side reaches past the area, Remotion stretches the edge pixels; here it stays
/// transparent. The children render twice.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Mirror<'a> {
    pub direction: MirrorDirection,
    pub position: Anim<'a, f32>,
    pub invert: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Mirror<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            direction: MirrorDirection::Horizontal,
            position: Anim::Fixed(0.5),
            invert: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Mirror<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let position = ui.sample(&self.position);
        let id = ui.id(self.key, 0, "mirror");
        let (w, h) = self.area;
        let horizontal = self.direction == MirrorDirection::Horizontal;
        let line = position * if horizontal { w } else { h };
        // the kept side and the mirrored side as (x, y, w, h)
        let (before, after) = if horizontal {
            ((0.0, 0.0, line, h), (line, 0.0, w - line, h))
        } else {
            ((0.0, 0.0, w, line), (0.0, line, w, h - line))
        };
        // the mirrored side reaches under the kept one, which covers it there
        let b = bleed(self.area);
        let reach = |(x, y, w, h): (f32, f32, f32, f32), toward_start: bool| match (
            horizontal,
            toward_start,
        ) {
            (true, true) => (x - b, y, w + b, h),
            (true, false) => (x, y, w + b, h),
            (false, true) => (x, y - b, w, h + b),
            (false, false) => (x, y, w, h + b),
        };
        let (kept, shown) = if self.invert {
            (after, reach(before, false))
        } else {
            (before, reach(after, true))
        };
        let flip = if horizontal {
            format!("translate({} 0) scale(-1 1)", 2.0 * line)
        } else {
            format!("translate(0 {}) scale(1 -1)", 2.0 * line)
        };
        let kept_id = format!("{id}-kept");
        let shown_id = format!("{id}-shown");
        let kept_url = format!("url(#{kept_id})");
        let shown_url = format!("url(#{shown_id})");
        let original = (self.children)(ui);
        let mirrored = (self.children)(ui);
        fframes::svgr!(
            <g>
                <defs>
                    <mask id={kept_id} maskUnits="userSpaceOnUse" x={-b} y={-b} width={w + 2.0 * b} height={h + 2.0 * b}>
                        <rect x={kept.0} y={kept.1} width={kept.2.max(0.0)} height={kept.3.max(0.0)} fill="#ffffff" />
                    </mask>
                    <clipPath id={shown_id} clipPathUnits="userSpaceOnUse">
                        <rect x={shown.0} y={shown.1} width={shown.2.max(0.0)} height={shown.3.max(0.0)} />
                    </clipPath>
                </defs>
                <g clip-path={shown_url}>
                    <g transform={flip}>{mirrored}</g>
                </g>
                <g mask={kept_url}>{original}</g>
            </g>
        )
    }
}

/// Its children repeated across the area like tiles, every other copy mirrored so the
/// edges meet (Remotion's `tile()`): the copies of `rect` (x, y, width, height) are laid
/// side by side with `horizontal` and above and below with `vertical`. Remotion finds the
/// rectangle from the opaque pixels; here it is given, as fframes cannot measure the
/// children. At most [`MAX_TILES`] copies are drawn, each twice: a slightly larger one under
/// the exact ones hides the seams between them. The children render twice.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Tile<'a> {
    pub rect: (f32, f32, f32, f32),
    pub horizontal: bool,
    pub vertical: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Tile<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            rect: (640.0, 360.0, 640.0, 360.0),
            horizontal: true,
            vertical: true,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Tile<'_> {
    /// The copies as (column, row) counted from the original, at most [`MAX_TILES`].
    pub fn copies(&self) -> Vec<(i64, i64)> {
        let (x, y, tw, th) = self.rect;
        let (w, h) = self.area;
        if tw < 1.0 || th < 1.0 {
            return Vec::new();
        }
        let range = |on: bool, start: f32, size: f32, end: f32| {
            if on {
                -(start / size).ceil() as i64..((end - start) / size).ceil() as i64
            } else {
                0..1
            }
        };
        let rows = range(self.vertical, y, th, h);
        let columns = range(self.horizontal, x, tw, w);
        rows.flat_map(|j| columns.clone().map(move |i| (i, j)))
            .take(MAX_TILES)
            .collect()
    }
}

impl<'a> Component<'a> for Tile<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "tile");
        let copies = self.copies();
        let content = (self.children)(ui);
        if copies.is_empty() {
            return content;
        }
        let (x, y, tw, th) = self.rect;
        let b = bleed(self.area);
        let (bx, by) = (
            if self.horizontal { b } else { 0.0 },
            if self.vertical { b } else { 0.0 },
        );
        let crop = format!("{id}-crop");
        let crop_url = format!("url(#{crop})");
        let wide = format!("{id}-wide");
        let wide_url = format!("url(#{wide})");
        let tile = format!("{id}-tile");
        let under = format!("{id}-under");
        let place = |href: &str| -> Vec<Svgr<'a>> {
            copies
                .iter()
                .map(|&(i, j)| {
                    // an odd copy is the original flipped about its own middle
                    let (sx, sy) = (
                        if i % 2 == 0 { 1 } else { -1 },
                        if j % 2 == 0 { 1 } else { -1 },
                    );
                    let (cx, cy) = (x + tw / 2.0, y + th / 2.0);
                    let place = format!(
                        "translate({} {}) scale({sx} {sy}) translate({} {})",
                        cx + i as f32 * tw,
                        cy + j as f32 * th,
                        -cx,
                        -cy
                    );
                    let href = format!("#{href}");
                    fframes::svgr!(<use href={href} transform={place} />)
                })
                .collect()
        };
        let (below, above) = (place(&under), place(&tile));
        let tiles = fframes::svgr!(
            <g>
                <defs>
                    <mask id={crop} maskUnits="userSpaceOnUse" x={x - 1.0} y={y - 1.0} width={tw + 2.0} height={th + 2.0}>
                        <rect x={x} y={y} width={tw} height={th} fill="#ffffff" />
                    </mask>
                    <clipPath id={wide} clipPathUnits="userSpaceOnUse">
                        <rect x={x - bx} y={y - by} width={tw + 2.0 * bx} height={th + 2.0 * by} />
                    </clipPath>
                    <g id={tile} mask={crop_url}>{content}</g>
                    <g id={under} clip-path={wide_url}>{(self.children)(ui)}</g>
                </defs>
                {Svgr::from(below)}
                {Svgr::from(above)}
            </g>
        );
        in_area(tiles, &id, self.area)
    }
}

/// Its children slanted (Remotion's `skew()`): `x` degrees along the horizontal, then `y`
/// along the vertical, like CSS `skewX` and `skewY`, about `origin` (0 to 1 across the
/// area). Cut to the area.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Skew<'a> {
    pub x: Anim<'a, f32>,
    pub y: Anim<'a, f32>,
    pub origin: (f32, f32),
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Skew<'_> {
    /// Remotion's defaults: 20 degrees along the horizontal about the middle.
    fn themed(_: &Theme) -> Self {
        Self {
            x: Anim::Fixed(20.0),
            y: Anim::Fixed(0.0),
            origin: (0.5, 0.5),
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Skew<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (x, y) = (ui.sample(&self.x), ui.sample(&self.y));
        let id = ui.id(self.key, 0, "skew");
        let (ox, oy) = (self.origin.0 * self.area.0, self.origin.1 * self.area.1);
        let slant = format!(
            "translate({ox} {oy}) skewY({y}) skewX({x}) translate({} {})",
            -ox, -oy
        );
        let content = (self.children)(ui);
        in_area(
            fframes::svgr!(<g transform={slant}>{content}</g>),
            &id,
            self.area,
        )
    }
}

/// Its children scaled by `scale` about the middle of the area (Remotion's `scale()`),
/// only across with `vertical` off and only up and down with `horizontal` off. Cut to the
/// area.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Scale<'a> {
    pub scale: Anim<'a, f32>,
    pub horizontal: bool,
    pub vertical: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Scale<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            scale: Anim::Fixed(1.0),
            horizontal: true,
            vertical: true,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Scale<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let scale = ui.sample(&self.scale).max(0.0);
        let id = ui.id(self.key, 0, "scale");
        let content = (self.children)(ui);
        let sx = if self.horizontal { scale } else { 1.0 };
        let sy = if self.vertical { scale } else { 1.0 };
        let (cx, cy) = (self.area.0 / 2.0, self.area.1 / 2.0);
        let resize = format!(
            "translate({cx} {cy}) scale({sx} {sy}) translate({} {})",
            -cx, -cy
        );
        in_area(
            fframes::svgr!(<g transform={resize}>{content}</g>),
            &id,
            self.area,
        )
    }
}

/// Its children moved by `x` and `y` (Remotion's `xyTranslate()`), or with `uv` by those
/// shares of the area, 1 the full width or height (Remotion's `uvTranslate()`). Cut to the
/// area.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Translate<'a> {
    pub x: Anim<'a, f32>,
    pub y: Anim<'a, f32>,
    pub uv: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Translate<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            x: Anim::Fixed(0.0),
            y: Anim::Fixed(0.0),
            uv: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Translate<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (mut x, mut y) = (ui.sample(&self.x), ui.sample(&self.y));
        if self.uv {
            (x, y) = (x * self.area.0, y * self.area.1);
        }
        let id = ui.id(self.key, 0, "translate");
        let content = (self.children)(ui);
        let shift = format!("translate({x} {y})");
        in_area(
            fframes::svgr!(<g transform={shift}>{content}</g>),
            &id,
            self.area,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_the_area_from_the_original() {
        let tile = Tile {
            rect: (640.0, 360.0, 640.0, 360.0),
            ..Tile::themed(&Theme::default())
        };
        let copies = tile.copies();
        assert_eq!(copies.len(), 9);
        assert!(copies.contains(&(-1, -1)) && copies.contains(&(1, 1)));
        let row = Tile {
            vertical: false,
            ..tile
        };
        assert_eq!(row.copies(), [(-1, 0), (0, 0), (1, 0)]);
    }

    #[test]
    fn tiles_stay_below_the_cap() {
        let tiny = Tile {
            rect: (0.0, 0.0, 1.0, 1.0),
            ..Tile::themed(&Theme::default())
        };
        assert_eq!(tiny.copies().len(), MAX_TILES);
    }
}
