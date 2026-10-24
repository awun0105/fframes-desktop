//! Glyph outlines of a string, for animations that draw text as paths (`Write`), and the
//! [`TextPath`] component.
//!
//! Outlines come from the font files of the media provider (`include_media_dir!` or
//! `MediaDirectory` fonts) and are laid out like `Ui::measure`: advances without kerning.
//! They are cached once per font, size and text. When no font file matches (system fonts,
//! the editor without the font), [`TextPath`] draws a `<text>` instead and `Write` fades
//! it in word by word.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use fframes::{
    Color, FFramesContext, Svgr,
    ttf_parser::{Face, OutlineBuilder, name_id},
};
use kurbo::{Affine, BezPath, Point, Rect};

use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, ShapeAnimation, draw_animated},
    geometry::{Geometry, Look, Shape, matrix},
    motion::progress,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// The outline of each glyph of a string, at the origin on the baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct TextOutline {
    /// One path per character with an outline; spaces have none.
    pub glyphs: Vec<BezPath>,
    /// Width of the whole string in pixels.
    pub advance: f64,
}

impl TextOutline {
    /// The glyphs moved so the baseline starts at `(x, y)`.
    pub fn placed(&self, x: f64, y: f64) -> Vec<BezPath> {
        let at = Affine::translate((x, y));
        self.glyphs.iter().map(|g| at * g).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    font: (usize, usize),
    size: usize,
    text: String,
}

/// Entries kept before the cache starts over; a video lays out far fewer strings.
const CACHE_LIMIT: usize = 512;

fn cache() -> &'static Mutex<HashMap<Key, Arc<TextOutline>>> {
    static CACHE: OnceLock<Mutex<HashMap<Key, Arc<TextOutline>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn family_matches(face: &Face<'_>, family: &str) -> bool {
    face.names().into_iter().any(|name| {
        matches!(name.name_id, name_id::FAMILY | name_id::TYPOGRAPHIC_FAMILY)
            && name
                .to_string()
                .is_some_and(|n| n.eq_ignore_ascii_case(family))
    })
}

/// The font file of `family` whose weight is closest to `weight`.
fn find_font<'m>(fonts: &[(&'m [u8], &str)], family: &str, weight: u16) -> Option<&'m [u8]> {
    fonts
        .iter()
        .filter_map(|&(data, _)| {
            let face = Face::parse(data, 0).ok()?;
            family_matches(&face, family)
                .then(|| (face.weight().to_number().abs_diff(weight), data))
        })
        .min_by_key(|&(distance, _)| distance)
        .map(|(_, data)| data)
}

struct Builder {
    path: BezPath,
    scale: f64,
    x: f64,
}

impl Builder {
    fn at(&self, x: f32, y: f32) -> Point {
        Point::new(
            self.x + f64::from(x) * self.scale,
            -f64::from(y) * self.scale,
        )
    }
}

impl OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.path.move_to(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.path.line_to(p);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p1, p) = (self.at(x1, y1), self.at(x, y));
        self.path.quad_to(p1, p);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (p1, p2, p) = (self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        self.path.curve_to(p1, p2, p);
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

/// Lays out `text` in the font file `data` at `size` pixels.
pub fn outline_with(data: &[u8], size: f64, text: &str) -> Option<TextOutline> {
    let face = Face::parse(data, 0).ok()?;
    let scale = size / f64::from(face.units_per_em());
    let mut x = 0.0;
    let mut glyphs = Vec::new();
    for c in text.chars() {
        let Some(id) = face.glyph_index(c) else {
            continue;
        };
        let mut builder = Builder {
            path: BezPath::new(),
            scale,
            x,
        };
        if face.outline_glyph(id, &mut builder).is_some() && !builder.path.is_empty() {
            glyphs.push(builder.path);
        }
        x += f64::from(face.glyph_hor_advance(id).unwrap_or_default()) * scale;
    }
    Some(TextOutline { glyphs, advance: x })
}

/// The outline of `text` in `style`, from the fonts of the context's media; `None` when no
/// font file has that family.
pub fn outline(
    ctx: &FFramesContext<'_, '_>,
    style: TextStyle,
    text: &str,
) -> Option<Arc<TextOutline>> {
    let fonts = ctx.media_source?.get_all_font_data();
    let data = find_font(&fonts, style.family, style.weight)?;
    let key = Key {
        font: (data.as_ptr() as usize, data.len()),
        size: style.size,
        text: text.to_owned(),
    };
    if let Some(hit) = cache().lock().ok()?.get(&key) {
        return Some(hit.clone());
    }
    let outline = Arc::new(outline_with(data, style.size as f64, text)?);
    let mut cache = cache().lock().ok()?;
    if cache.len() >= CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(key, outline.clone());
    Some(outline)
}

/// Text drawn as glyph outlines, so `Create`, `Write` and `DrawBorderThenFill` can draw it
/// (Manim's `Text` as a `VMobject`). `(x, y)` is the start of the baseline.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TextPath<'s> {
    pub text: &'s str,
    pub x: f32,
    pub y: f32,
    /// The font: a family from the video's media, its size in pixels and weight.
    pub style: TextStyle,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
}

impl Themed for TextPath<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: "",
            x: 0.0,
            y: 0.0,
            style: t.typography.title,
            fill: Some(t.colors.title),
            stroke: None,
            stroke_width: t.stroke.thin,
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
        }
    }
}

impl Default for TextPath<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl TextPath<'_> {
    fn shape(&self, path: BezPath) -> Shape {
        Shape {
            path,
            fill: self.fill,
            stroke: self.stroke,
            stroke_width: self.stroke_width,
            dash: None,
            opacity: self.opacity,
        }
    }

    /// One shape per glyph, or `None` when the font has no file in the media.
    pub fn parts(&self, ctx: &FFramesContext<'_, '_>) -> Option<Vec<Shape>> {
        let outline = outline(ctx, self.style, self.text)?;
        Some(
            outline
                .placed(f64::from(self.x), f64::from(self.y))
                .into_iter()
                .map(|p| self.shape(p))
                .collect(),
        )
    }

    /// The box of the inked glyphs, descenders included (what Manim measures around
    /// text), or the estimated box when the font has no file in the media.
    pub fn bounds(&self, ctx: &FFramesContext<'_, '_>) -> Rect {
        self.parts(ctx)
            .and_then(|parts| {
                parts
                    .iter()
                    .map(|p| kurbo::Shape::bounding_box(&p.path))
                    .reduce(|a, b| a.union(b))
            })
            .unwrap_or_else(|| kurbo::Shape::bounding_box(&self.estimated_box()))
    }

    /// A box of about the text's size for the `<text>` fallback: 0.55 em per character,
    /// from the cap height to the baseline.
    fn estimated_box(&self) -> BezPath {
        let size = self.style.size as f64;
        let width = 0.55 * size * self.text.chars().count() as f64;
        let (x, y) = (f64::from(self.x), f64::from(self.y));
        kurbo::Shape::to_path(&Rect::new(x, y - 0.7 * size, x + width, y), 0.1)
    }

    fn fallback<'a>(self, ui: &Ui<'a, '_, '_>) -> Svgr<'a> {
        let whole = self.shape(self.estimated_box());
        let mut look = Look::default();
        if let Some(e) = &self.emphasis {
            e.look(ui, &whole, &mut look);
        }
        if let Some(x) = &self.exit {
            x.look(ui, &whole, &mut look);
        }
        let words: Vec<&str> = self.text.split_inclusive(' ').collect();
        let word_opacity: Vec<f32> = match &self.enter {
            Some(EnterAnim::Write(w)) => (0..words.len())
                .map(|i| {
                    let (start, each) = w.window(i, words.len());
                    w.rate_func
                        .apply(progress(ui.seconds(), start, start + each), each)
                })
                .collect(),
            Some(e) => {
                e.look(ui, &whole, &mut look);
                // the part of the outline an outline animation has drawn
                vec![look.drawn.1.clamp(0.0, 1.0); words.len()]
            }
            None => vec![1.0; words.len()],
        };
        let tspans: Vec<Svgr<'a>> = words
            .iter()
            .zip(word_opacity)
            .map(|(word, o)| {
                let word = (*word).to_owned();
                fframes::svgr!(<tspan opacity={o}>{word}</tspan>)
            })
            .collect();
        let fill = self.fill.or(self.stroke).unwrap_or(Color::WHITE);
        let opacity = (self.opacity * look.opacity).clamp(0.0, 1.0);
        let transform = matrix(look.transform);
        fframes::svgr!(
            <text
                x={self.x}
                y={self.y}
                transform={transform}
                font-family={self.style.family}
                font-size={self.style.size}
                font-weight={self.style.weight}
                fill={fill}
                opacity={opacity}
            >
                {tspans}
            </text>
        )
    }
}

impl Geometry for TextPath<'_> {
    /// Without a context the outline is not known: the estimated box.
    fn path(&self) -> BezPath {
        self.estimated_box()
    }
}

impl<'a> Component<'a> for TextPath<'_> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        match self.parts(ui.ctx) {
            Some(parts) => draw_animated(
                ui,
                &parts,
                self.enter.as_ref(),
                self.exit.as_ref(),
                self.emphasis.as_ref(),
            ),
            None => self.fallback(ui),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Shape as _;

    fn font() -> &'static [u8] {
        include_bytes!("../../gallery/media/DMSans-Medium.ttf")
    }

    #[test]
    fn finds_the_family_and_closest_weight() {
        let fonts = [(font(), "DMSans-Medium.ttf")];
        assert!(find_font(&fonts, "DM Sans", 700).is_some());
        assert!(find_font(&fonts, "dm sans", 400).is_some());
        assert!(find_font(&fonts, "Inter", 400).is_none());
    }

    #[test]
    fn glyphs_sit_on_the_baseline_and_skip_spaces() {
        let o = outline_with(font(), 100.0, "Hi there").unwrap();
        // "Hi there" has 7 letters with outlines
        assert_eq!(o.glyphs.len(), 7);
        let h = o.glyphs[0].bounding_box();
        // capitals rise above the baseline (y up is negative), nothing hangs below it
        assert!(h.y0 < -60.0 && h.y1 <= 0.5, "{h:?}");
        assert!(o.advance > 300.0 && o.advance < 450.0, "{}", o.advance);
        let placed = o.placed(10.0, 200.0);
        assert!((placed[0].bounding_box().y1 - h.y1 - 200.0).abs() < 1e-9);
    }
}
