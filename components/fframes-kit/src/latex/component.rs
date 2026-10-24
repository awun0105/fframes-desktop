//! `MathTex` and `Tex` (what to typeset), `Formula` (the typeset glyphs) and `FormulaView`
//! (the component that draws one).

use fframes::{Color, Svgr};
use kurbo::{Affine, BezPath, Rect, Shape as _};

use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::{Geometry, Shape},
    layout::Anchor,
    theme::Theme,
    ui::{Component, Ui},
};

use super::{
    PartSource, TexEngine, TexError, TexGlyph, TexTemplate, split_parts, svg_to_glyphs::parse_svg,
};

/// Pixels per TeX point at the default size: 10 pt text is 48 px tall.
pub const PX_PER_PT: f32 = 4.8;

/// What a `{{ }}` group becomes in the document: a braced group (as in Manim, so spacing is
/// the same as `{{..}}`) between two dvisvgm markers.
fn mark_part(index: usize, tex: &str) -> String {
    format!("\\fframesPart{{{index}}}{{{tex}}}")
}

fn typeset(
    engine: &TexEngine,
    template: &TexTemplate,
    source: &str,
    math: bool,
) -> Result<Formula, TexError> {
    let (marked, parts) = split_parts(source, mark_part);
    let body = if math {
        format!("\\begin{{align*}}\n{marked}\n\\end{{align*}}")
    } else {
        marked
    };
    let svg = engine.svg(template, &body, source)?;
    Formula::from_svg(source, &svg, parts)
}

/// A formula in math mode (Manim's `MathTex`): the source goes into an `align*`
/// environment, so `&` aligns and `\\` breaks lines. Each `{{ }}` group is one part that
/// `color_map`, `part_bbox` and `TransformMatchingTex` can address.
#[derive(Debug, Clone, PartialEq)]
pub struct MathTex<'s> {
    pub tex: &'s str,
    pub template: TexTemplate,
}

impl<'s> MathTex<'s> {
    pub fn new(tex: &'s str) -> Self {
        Self {
            tex,
            template: TexTemplate::default(),
        }
    }

    pub fn template(self, template: TexTemplate) -> Self {
        Self { template, ..self }
    }

    /// Runs TeX (or reads the cache). Call it in the video's constructor.
    pub fn typeset(&self, engine: &TexEngine) -> Result<Formula, TexError> {
        typeset(engine, &self.template, self.tex, true)
    }
}

/// Text mode (Manim's `Tex`): anything a document body can hold, `$..$` math, TikZ
/// pictures. `{{ }}` groups mark parts like in [`MathTex`].
#[derive(Debug, Clone, PartialEq)]
pub struct Tex<'s> {
    pub text: &'s str,
    pub template: TexTemplate,
}

impl<'s> Tex<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            text,
            template: TexTemplate::default(),
        }
    }

    pub fn template(self, template: TexTemplate) -> Self {
        Self { template, ..self }
    }

    /// Runs TeX (or reads the cache). Call it in the video's constructor.
    pub fn typeset(&self, engine: &TexEngine) -> Result<Formula, TexError> {
        typeset(engine, &self.template, self.text, false)
    }
}

/// A part of a formula: the `n`th `{{ }}` group, or the one labelled `\part{name}{..}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part<'s> {
    Index(usize),
    Label(&'s str),
}

impl Part<'_> {
    fn matches(&self, glyph: &TexGlyph) -> bool {
        match self {
            Part::Index(n) => glyph.part == Some(*n),
            Part::Label(l) => glyph.label.as_deref() == Some(*l),
        }
    }
}

/// A typeset formula: its glyphs in TeX points, `y` down, with the top left of the ink at
/// the origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    /// The source as written, `{{ }}` included.
    pub source: String,
    glyphs: Vec<TexGlyph>,
    parts: Vec<PartSource>,
    width: f64,
    height: f64,
}

impl Formula {
    /// The formula from the SVG that `dvisvgm` made of `source`.
    pub fn from_svg(source: &str, svg: &str, parts: Vec<PartSource>) -> Result<Self, TexError> {
        let mut glyphs = parse_svg(svg)?;
        let ink = glyphs
            .iter()
            .map(|g| g.path.bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::ZERO);
        let to_origin = Affine::translate((-ink.x0, -ink.y0));
        for g in &mut glyphs {
            g.path.apply_affine(to_origin);
        }
        Ok(Self {
            source: source.to_owned(),
            glyphs,
            parts,
            width: ink.width(),
            height: ink.height(),
        })
    }

    /// A formula with no glyphs: [`FormulaView`] draws its source as text. For when TeX is
    /// missing and the cache has no entry (the editor).
    pub fn unset(source: &str) -> Self {
        Self {
            source: source.to_owned(),
            glyphs: Vec::new(),
            parts: Vec::new(),
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn glyphs(&self) -> &[TexGlyph] {
        &self.glyphs
    }

    /// The `{{ }}` groups of the source.
    pub fn parts(&self) -> &[PartSource] {
        &self.parts
    }

    /// Width and height of the ink in TeX points.
    pub fn size_pt(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    pub fn is_typeset(&self) -> bool {
        !self.glyphs.is_empty()
    }

    /// The box around a part's glyphs in TeX points; `None` when no glyph belongs to it.
    pub fn part_bbox(&self, part: Part<'_>) -> Option<Rect> {
        self.glyphs
            .iter()
            .filter(|g| part.matches(g))
            .map(|g| g.path.bounding_box())
            .reduce(|a, b| a.union(b))
    }
}

impl Geometry for Formula {
    /// Every glyph in TeX points.
    fn path(&self) -> BezPath {
        let mut path = BezPath::new();
        for g in &self.glyphs {
            path.extend(g.path.iter());
        }
        path
    }

    fn bbox(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }
}

/// The family of the text drawn for a formula that was not typeset. No media has it, so
/// `inspect` lists every such formula as a missing font.
pub const UNSET_FORMULA_FONT: &str = "fframes-kit formula not typeset";

/// Draws a [`Formula`] (Manim's `MathTex` on screen).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FormulaView<'f> {
    pub of: &'f Formula,
    /// Height in pixels of 10 pt text (the TeX default size).
    pub size: f32,
    pub x: f32,
    pub y: f32,
    /// The point of the formula's box at `(x, y)`.
    pub anchor: Anchor,
    /// Replaces black, the color of everything the source does not color itself.
    pub color: Color,
    /// Colors of single parts, over `color` and the source's colors.
    pub color_map: &'f [(Part<'f>, Color)],
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
    /// The part `emphasis` applies to; the whole formula when `None`.
    pub emphasize: Option<Part<'f>>,
    pub key: Option<&'f str>,
}

impl<'f> FormulaView<'f> {
    pub fn new(of: &'f Formula) -> Self {
        Self::themed_of(of, &Theme::default())
    }

    fn themed_of(of: &'f Formula, t: &Theme) -> Self {
        Self {
            of,
            size: 10.0 * PX_PER_PT,
            x: 0.0,
            y: 0.0,
            anchor: Anchor::TopLeft,
            color: t.colors.title,
            color_map: &[],
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
            emphasize: None,
            key: None,
        }
    }

    /// The same view with the theme's colors.
    pub fn themed(self, t: &Theme) -> Self {
        Self {
            color: t.colors.title,
            ..self
        }
    }

    /// Pixels per TeX point.
    pub fn scale(&self) -> f64 {
        f64::from(self.size) / 10.0
    }

    /// From TeX points to the canvas.
    pub fn transform(&self) -> Affine {
        let s = self.scale();
        let (w, h) = self.of.size_pt();
        let (ax, ay) = self.anchor.point(w * s, h * s);
        Affine::translate((f64::from(self.x) - ax, f64::from(self.y) - ay)) * Affine::scale(s)
    }

    /// The box of a part on the canvas.
    pub fn part_bbox(&self, part: Part<'_>) -> Option<Rect> {
        self.of
            .part_bbox(part)
            .map(|b| self.transform().transform_rect_bbox(b))
    }

    fn color_of(&self, glyph: &TexGlyph, paint: [u8; 3]) -> Color {
        if let Some((_, c)) = self.color_map.iter().find(|(p, _)| p.matches(glyph)) {
            return *c;
        }
        match paint {
            [0, 0, 0] => self.color,
            [r, g, b] => Color::rgb(r, g, b),
        }
    }

    /// One shape per glyph on the canvas, in drawing order.
    pub fn shapes(&self) -> Vec<Shape> {
        let at = self.transform();
        let s = self.scale() as f32;
        self.of
            .glyphs()
            .iter()
            .map(|g| Shape {
                path: at * &g.path,
                fill: g.fill.map(|p| self.color_of(g, p)),
                stroke: g.stroke.map(|(p, _)| self.color_of(g, p)),
                stroke_width: g.stroke.map_or(0.0, |(_, w)| w * s),
                dash: None,
                opacity: self.opacity,
            })
            .collect()
    }

    fn unset<'a>(&self) -> Svgr<'a> {
        let source = self.of.source.clone();
        let size = self.size;
        fframes::svgr!(
            <text x={self.x} y={self.y + size} font-family={UNSET_FORMULA_FONT}
                  font-size={size} fill={self.color} opacity={self.opacity}>{source}</text>
        )
    }
}

impl<'a> Component<'a> for FormulaView<'_> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        if !self.of.is_typeset() {
            return self.unset();
        }
        let shapes = self.shapes();
        let (enter, exit) = (self.enter.as_ref(), self.exit.as_ref());
        let id = ui.id(self.key, 0, "formula");
        let drawn = match (self.emphasize, self.emphasis.as_ref()) {
            (Some(part), Some(emphasis)) => {
                let (marked, rest): (Vec<_>, Vec<_>) = shapes
                    .into_iter()
                    .zip(self.of.glyphs())
                    .partition(|(_, g)| part.matches(g));
                let marked: Vec<Shape> = marked.into_iter().map(|(s, _)| s).collect();
                let rest: Vec<Shape> = rest.into_iter().map(|(s, _)| s).collect();
                let rest = draw_animated(ui, &rest, enter, exit, None);
                let marked = draw_animated(ui, &marked, enter, exit, Some(emphasis));
                fframes::svgr!(<g>{rest}{marked}</g>)
            }
            (_, emphasis) => draw_animated(ui, &shapes, enter, exit, emphasis),
        };
        fframes::svgr!(<g id={id}>{drawn}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{TexCompiler, TexTools};
    use super::*;

    fn sample(name: &str, parts: Vec<PartSource>) -> Formula {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/latex/samples/");
        let svg = std::fs::read_to_string(format!("{dir}{name}.svg")).unwrap();
        Formula::from_svg(name, &svg, parts).unwrap()
    }

    #[test]
    fn ten_point_text_is_48_px() {
        let euler = sample("euler", Vec::new());
        // the last glyph of e^{i\pi} + 1 = 0 is the digit 0
        let zero_pt = euler.glyphs().last().unwrap().path.bounding_box();
        let view = FormulaView::new(&euler);
        let zero_px = view.shapes().last().unwrap().path.bounding_box();
        assert!((zero_px.height() - zero_pt.height() * f64::from(PX_PER_PT)).abs() < 1e-5);
        // a digit of 10 pt Computer Modern is about two thirds of the size
        assert!((28.0..34.0).contains(&zero_px.height()), "{zero_px:?}");
        // the ink starts at the origin; anchors move the box
        let (w, h) = euler.size_pt();
        let all = view
            .shapes()
            .iter()
            .map(|s| s.path.bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap();
        assert!(all.x0.abs() < 1e-9 && all.y0.abs() < 1e-9);
        assert!((all.width() - w * 4.8).abs() < 1e-6 && (all.height() - h * 4.8).abs() < 1e-6);
        let centered = FormulaView {
            x: 100.0,
            y: 50.0,
            anchor: Anchor::Center,
            ..view
        };
        let c = centered.transform() * kurbo::Point::new(w / 2.0, h / 2.0);
        assert!((c.x - 100.0).abs() < 1e-9 && (c.y - 50.0).abs() < 1e-9);
    }

    #[test]
    fn parts_have_boxes_and_colors() {
        let (_, parts) = split_parts("{{a}} + {{b}} = {{c}}", mark_part);
        let f = sample("parts-bin", parts);
        assert_eq!(f.parts()[1].tex, "b");
        let a = f.part_bbox(Part::Index(0)).unwrap();
        let b = f.part_bbox(Part::Index(1)).unwrap();
        assert!(a.x1 < b.x0);
        assert!(f.part_bbox(Part::Index(3)).is_none());
        let red = Color::rgb(255, 0, 0);
        let map = [(Part::Index(1), red)];
        let view = FormulaView {
            color_map: &map,
            color: Color::WHITE,
            ..FormulaView::new(&f)
        };
        let fills: Vec<_> = view.shapes().iter().map(|s| s.fill).collect();
        assert_eq!(fills[2], Some(red));
        assert_eq!(fills[0], Some(Color::WHITE));
    }

    /// Runs the installed TeX; prints why and passes when there is none.
    #[test]
    fn typesets_with_the_installed_tex_and_caches() {
        if TexTools::discover()
            .require(TexCompiler::Latex, "test")
            .is_err()
        {
            eprintln!("no TeX installed: skipped");
            return;
        }
        let dir = std::env::temp_dir().join(format!("fframes-kit-tex-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let engine = TexEngine::new(&dir);
        let source = "{{a}}^2 + \\part{rest}{b}";
        let first = MathTex::new(source).typeset(&engine).unwrap();
        let parts: Vec<_> = first
            .glyphs()
            .iter()
            .map(|g| (g.part, g.label.clone()))
            .collect();
        assert_eq!(parts[0], (Some(0), None));
        assert_eq!(parts.last().unwrap(), &(None, Some("rest".to_owned())));
        assert!(first.part_bbox(Part::Label("rest")).is_some());
        let cached: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(cached.len(), 1);
        let svg = std::fs::read_to_string(cached[0].path()).unwrap();
        assert!(!svg.contains('\r'), "the cache uses \\n line ends");
        // the second time comes from the cache, even without TeX
        let offline = TexEngine::new(&dir).with_tools(TexTools::default());
        assert_eq!(first, MathTex::new(source).typeset(&offline).unwrap());
        let missing = MathTex::new("b^3").typeset(&offline).unwrap_err();
        assert!(
            matches!(missing, TexError::NotInstalled { .. }),
            "{missing}"
        );
        let broken = MathTex::new("\\undefinedmacro")
            .typeset(&engine)
            .unwrap_err();
        let TexError::Compile { log, source } = broken else {
            panic!("{broken}")
        };
        assert!(log.starts_with("! Undefined control sequence"), "{log}");
        assert!(source.contains("\\undefinedmacro"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
