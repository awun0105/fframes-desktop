//! The SVG from `dvisvgm --no-fonts` as one path per glyph, rule and stroke.

use fframes::usvgr::{self, Node, Paint, fontdb};
use kurbo::BezPath;

use super::TexError;

/// The group id prefixes that mark a `{{ }}` part and a `\part{name}{..}` label.
const PART_ID: &str = "part-";
const LABEL_ID: &str = "label-";

/// One drawn element of a typeset page, in TeX points (`y` down).
#[derive(Debug, Clone, PartialEq)]
pub struct TexGlyph {
    pub path: BezPath,
    /// The fill color TeX chose (black unless the formula sets one).
    pub fill: Option<[u8; 3]>,
    /// The outline color and width, for TikZ lines.
    pub stroke: Option<([u8; 3], f32)>,
    /// The `{{ }}` part this glyph belongs to.
    pub part: Option<usize>,
    /// The innermost `\part{name}{..}` around it.
    pub label: Option<String>,
}

fn rgb(paint: &Paint) -> [u8; 3] {
    match paint {
        Paint::Color(c) => [c.red, c.green, c.blue],
        _ => [0, 0, 0],
    }
}

fn walk(group: &usvgr::Group, part: Option<usize>, label: Option<&str>, out: &mut Vec<TexGlyph>) {
    let part = group
        .id()
        .strip_prefix(PART_ID)
        .and_then(|n| n.parse().ok())
        .or(part);
    let label = group.id().strip_prefix(LABEL_ID).or(label);
    for node in group.children() {
        match node {
            Node::Group(g) => walk(g, part, label, out),
            Node::Path(path) => {
                let (bez, at) = crate::paths::from_usvgr(path);
                let scale = at.determinant().abs().sqrt() as f32;
                out.push(TexGlyph {
                    path: bez,
                    fill: path.fill().map(|f| rgb(f.paint())),
                    stroke: path
                        .stroke()
                        .map(|s| (rgb(s.paint()), s.width().get() * scale)),
                    part,
                    label: label.map(str::to_owned),
                });
            }
            _ => {}
        }
    }
}

/// Every glyph of `svg` with its absolute position.
pub(crate) fn parse_svg(svg: &str) -> Result<Vec<TexGlyph>, TexError> {
    let tree = usvgr::Tree::from_str(svg, &usvgr::Options::default(), &fontdb::Database::new())
        .map_err(|e| TexError::Convert(format!("the SVG from dvisvgm does not parse: {e}")))?;
    let mut out = Vec::new();
    walk(tree.root(), None, None, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use kurbo::Shape;

    use super::*;

    /// The shape of a `dvisvgm --no-fonts` page: glyphs in `<defs>` placed by `<use>`, a
    /// rule as `<rect>`, a colored group and a part group.
    const PAGE: &str = "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' viewBox='0 0 20 10' width='20pt' height='10pt'>\
        <defs><path id='g0-1' d='M0 0L2 0L2 -2Z'/></defs>\
        <g id='page1'>\
          <use x='1' y='5' xlink:href='#g0-1'/>\
          <rect x='4' y='6' height='0.5' width='6'/>\
          <g id='label-lhs'><g id='part-3'><g fill='#ff0000'><use x='10' y='5' xlink:href='#g0-1'/></g></g></g>\
        </g></svg>";

    #[test]
    fn uses_become_placed_paths_with_their_part_and_color() {
        let glyphs = parse_svg(PAGE).unwrap();
        assert_eq!(glyphs.len(), 3);
        let b = glyphs[0].path.bounding_box();
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (1.0, 3.0, 3.0, 5.0));
        assert_eq!(glyphs[0].fill, Some([0, 0, 0]));
        assert_eq!(glyphs[0].part, None);
        let rule = glyphs[1].path.bounding_box();
        assert_eq!((rule.x0, rule.width()), (4.0, 6.0));
        assert_eq!(glyphs[2].part, Some(3));
        assert_eq!(glyphs[2].label.as_deref(), Some("lhs"));
        assert_eq!(glyphs[2].fill, Some([255, 0, 0]));
        assert_eq!(glyphs[2].path.bounding_box().x0, 10.0);
    }

    fn sample(name: &str) -> Vec<TexGlyph> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/latex/samples/");
        parse_svg(&std::fs::read_to_string(format!("{dir}{name}.svg")).unwrap()).unwrap()
    }

    /// SVGs that MiKTeX 26.5 (pdfTeX 4.27, dvisvgm 3.6) made of the six reference formulas.
    #[test]
    fn dvisvgm_samples_parse_into_glyphs() {
        // e^{i\pi} + 1 = 0: e, i, pi, +, 1, =, 0
        assert_eq!(sample("euler").len(), 7);
        // \frac{a+b}{c}: three glyphs above the rule, the rule, one below
        let frac = sample("fraction");
        assert_eq!(frac.len(), 5);
        let rule = frac[3].path.bounding_box();
        assert!(rule.width() > 20.0 && rule.height() < 0.5, "{rule:?}");
        assert!(frac[0].path.bounding_box().y1 < rule.y0);
        assert!(frac[4].path.bounding_box().y0 > rule.y1);
        // a 2x2 pmatrix: four letters and the two parentheses
        assert!(sample("matrix").len() >= 6);
        assert!(sample("sum").len() > 10);
        assert!(sample("chem").len() >= 8);
        // TikZ: the arrow's line and head are stroked, the circle blue, the square red
        let tikz = sample("tikz");
        assert!(
            tikz.iter()
                .any(|g| g.stroke == Some(([0, 0, 255], 0.4 * 0.996_264)))
        );
        assert!(
            tikz.iter()
                .any(|g| g.fill == Some([255, 0, 0]) && g.stroke.is_none())
        );
        assert!(tikz.iter().all(|g| g.fill.is_some() || g.stroke.is_some()));
    }

    /// The markers of `{{a}}^2`, `\frac{{{a}}}{b}`, `\left({{x}}\right)` and
    /// `{{a}} + {{b}} = {{c}}` come out as part groups around exactly the glyphs of each part.
    #[test]
    fn part_markers_survive_as_groups() {
        let parts = |name| sample(name).iter().map(|g| g.part).collect::<Vec<_>>();
        assert_eq!(parts("parts-sup"), [Some(0), None]);
        assert_eq!(parts("parts-frac"), [Some(0), None, None]);
        assert_eq!(parts("parts-left"), [None, Some(0), None]);
        assert_eq!(parts("parts-bin"), [Some(0), None, Some(1), None, Some(2)]);
    }

    /// A marker pair moves the glyphs after it by at most one scaled point (TeX's unit,
    /// 1/65536 pt), which dvisvgm writes as a difference in the fifth decimal. The samples
    /// show up to 2.5e-5 pt; allow 2 sp.
    #[test]
    fn part_markers_keep_glyph_positions() {
        const TWO_SP: f64 = 2.0 / 65536.0;
        for case in ["sup", "frac", "left", "bin"] {
            let plain = sample(&format!("parts-{case}-plain"));
            let marked = sample(&format!("parts-{case}"));
            assert_eq!(plain.len(), marked.len(), "{case}");
            for (a, b) in plain.iter().zip(&marked) {
                let (a, b) = (a.path.bounding_box(), b.path.bounding_box());
                let d = (a.x0 - b.x0).abs().max((a.y0 - b.y0).abs());
                assert!(d <= TWO_SP, "{case}: {a:?} vs {b:?}");
            }
        }
    }
}
