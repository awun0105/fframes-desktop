//! `TransformMatchingTex`: one formula into another, moving what both have.

use std::fmt::Write as _;

use fframes::Svgr;
use kurbo::{Affine, Rect, Shape as _};

use crate::{
    anim::RateFunc,
    animate::{MORPH_SEGMENTS, enter_alpha, lerp_paint, morph},
    geometry::Shape,
    motion::lerp,
    paths,
    ui::{Component, Ui},
};

use super::{FormulaView, TexGlyph};

/// What a group of glyphs is matched by.
#[derive(Debug, Clone, PartialEq)]
enum Key {
    /// `\part{name}{..}`
    Label(String),
    /// A `{{ }}` part, by its TeX source.
    Tex(String),
    /// A glyph outside every part, by its outline.
    Outline(String),
}

/// Glyph indices of a formula grouped the way they are matched: one group per label, then
/// per `{{ }}` part, then one per remaining glyph.
fn groups(view: &FormulaView<'_>) -> Vec<(Key, Vec<usize>)> {
    let mut out: Vec<(Key, Vec<usize>)> = Vec::new();
    let add = |key: Key, i: usize, out: &mut Vec<(Key, Vec<usize>)>| {
        if let Some((_, members)) = out.iter_mut().find(|(k, _)| *k == key) {
            members.push(i);
        } else {
            out.push((key, vec![i]));
        }
    };
    let parts = view.of.parts();
    for (i, g) in view.of.glyphs().iter().enumerate() {
        let key = match (&g.label, g.part) {
            (Some(l), _) => Key::Label(l.clone()),
            // the part's index keeps two parts with the same source apart
            (None, Some(p)) => Key::Tex(format!(
                "{}\u{0}{p}",
                parts.get(p).map_or("", |s| s.tex.as_str())
            )),
            (None, None) => {
                out.push((Key::Outline(outline_key(g)), vec![i]));
                continue;
            }
        };
        add(key, i, &mut out);
    }
    out
}

/// The outline of a glyph moved to the origin, rounded: equal for the same glyph of the same
/// font and size wherever it sits.
fn outline_key(g: &TexGlyph) -> String {
    let b = g.path.bounding_box();
    let moved = Affine::translate((-b.x0, -b.y0)) * &g.path;
    let mut key = String::new();
    for el in moved.elements() {
        for p in el_points(*el) {
            let _ = write!(key, "{:.3},{:.3};", p.x, p.y);
        }
        key.push('|');
    }
    key
}

fn el_points(el: kurbo::PathEl) -> Vec<kurbo::Point> {
    use kurbo::PathEl::*;
    match el {
        MoveTo(a) | LineTo(a) => vec![a],
        QuadTo(a, b) => vec![a, b],
        CurveTo(a, b, c) => vec![a, b, c],
        ClosePath => Vec::new(),
    }
}

/// The source key of a `{{ }}` group without the index that keeps groups apart.
fn tex_of(key: &Key) -> Option<&str> {
    match key {
        Key::Tex(t) => t.split('\u{0}').next(),
        _ => None,
    }
}

/// Pairs of matched groups (from, to), then the unmatched groups of each side.
type Matching = (Vec<(Vec<usize>, Vec<usize>)>, Vec<usize>, Vec<usize>);

fn match_groups(from: &[(Key, Vec<usize>)], to: &[(Key, Vec<usize>)]) -> Matching {
    let mut used = vec![false; to.len()];
    let mut pairs = Vec::new();
    let mut gone = Vec::new();
    // labels first, then equal sources, then equal outlines
    let passes: [fn(&Key, &Key) -> bool; 3] = [
        |a, b| matches!((a, b), (Key::Label(x), Key::Label(y)) if x == y),
        |a, b| tex_of(a).is_some() && tex_of(a) == tex_of(b),
        |a, b| matches!((a, b), (Key::Outline(x), Key::Outline(y)) if x == y),
    ];
    let mut matched = vec![None; from.len()];
    for same in passes {
        for (i, (key, _)) in from.iter().enumerate() {
            if matched[i].is_some() {
                continue;
            }
            if let Some(j) = (0..to.len()).find(|&j| !used[j] && same(key, &to[j].0)) {
                used[j] = true;
                matched[i] = Some(j);
            }
        }
    }
    for (i, m) in matched.iter().enumerate() {
        match m {
            Some(j) => pairs.push((from[i].1.clone(), to[*j].1.clone())),
            None => gone.extend(&from[i].1),
        }
    }
    let new = to
        .iter()
        .zip(&used)
        .filter(|(_, u)| !**u)
        .flat_map(|((_, g), _)| g.iter().copied())
        .collect();
    (pairs, gone, new)
}

fn union(shapes: &[Shape], members: &[usize]) -> Shape {
    let mut whole = shapes[members[0]].clone();
    for &i in &members[1..] {
        whole.path.extend(shapes[i].path.iter());
    }
    whole
}

fn rect_lerp(a: Rect, b: Rect, t: f64) -> Rect {
    Rect::new(
        a.x0 + (b.x0 - a.x0) * t,
        a.y0 + (b.y0 - a.y0) * t,
        a.x1 + (b.x1 - a.x1) * t,
        a.y1 + (b.y1 - a.y1) * t,
    )
}

/// The affine that maps `a` onto `b`.
fn fit(a: Rect, b: Rect) -> Affine {
    let sx = if a.width() > 0.0 {
        b.width() / a.width()
    } else {
        1.0
    };
    let sy = if a.height() > 0.0 {
        b.height() / a.height()
    } else {
        1.0
    };
    Affine::translate((b.x0, b.y0))
        * Affine::scale_non_uniform(sx, sy)
        * Affine::translate((-a.x0, -a.y0))
}

/// `from` moved and scaled `t` of the way onto `to`'s box: exact for the same glyph.
fn slide(from: &Shape, to: &Shape, t: f32) -> Shape {
    let (a, b) = (from.path.bounding_box(), to.path.bounding_box());
    let at = fit(a, rect_lerp(a, b, f64::from(t)));
    Shape {
        path: at * &from.path,
        fill: lerp_paint(from.fill, to.fill, t),
        stroke: lerp_paint(from.stroke, to.stroke, t),
        stroke_width: lerp(from.stroke_width, to.stroke_width, t),
        dash: None,
        opacity: lerp(from.opacity, to.opacity, t),
    }
}

/// One formula into another (Manim's `TransformMatchingTex`). Parts with the same
/// `\part{..}` label match first, then `{{ }}` parts with the same source, then glyphs
/// outside parts with the same outline. Matched glyphs move to their new place (a part
/// with a different number of glyphs morphs as one shape), the rest of `from` fades out and
/// the rest of `to` fades in. Before the start it draws `from`, after the end `to`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TransformMatchingTex<'f> {
    pub from: FormulaView<'f>,
    pub to: FormulaView<'f>,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl<'f> TransformMatchingTex<'f> {
    /// From `from` to `to` in Manim's 1 second, smooth.
    pub fn new(from: FormulaView<'f>, to: FormulaView<'f>) -> Self {
        Self {
            from,
            to,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }

    /// The shapes `p` of the way from `from` to `to`.
    pub fn shapes_at(&self, p: f32) -> Vec<Shape> {
        let (a, b) = (self.from.shapes(), self.to.shapes());
        let (pairs, gone, new) = match_groups(&groups(&self.from), &groups(&self.to));
        let mut out = Vec::new();
        for (f, t) in &pairs {
            if f.len() == t.len() {
                out.extend(f.iter().zip(t).map(|(&i, &j)| slide(&a[i], &b[j], p)));
            } else {
                let (from, to) = (union(&a, f), union(&b, t));
                let (pa, pb) = paths::align_points(&from.path, &to.path, MORPH_SEGMENTS);
                out.push(morph(&from, &to, &pa, &pb, p));
            }
        }
        let faded = |s: &Shape, o: f32| Shape {
            opacity: s.opacity * o,
            ..s.clone()
        };
        out.extend(gone.iter().map(|&i| faded(&a[i], 1.0 - p)));
        out.extend(new.iter().map(|&j| faded(&b[j], p)));
        out
    }

    /// Glyph counts: (matched pairs of groups, glyphs fading out, glyphs fading in).
    pub fn matching(&self) -> (usize, usize, usize) {
        let (pairs, gone, new) = match_groups(&groups(&self.from), &groups(&self.to));
        (pairs.len(), gone.len(), new.len())
    }
}

impl<'a> Component<'a> for TransformMatchingTex<'_> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let end = self.delay + self.run_time;
        let (from, to) = (self.from.clone(), self.to.clone());
        if p <= 0.0 && ui.seconds() < self.delay {
            return from.render(ui);
        }
        if p >= 1.0 && ui.seconds() >= end {
            return to.render(ui);
        }
        let shapes: Vec<Svgr<'a>> = self
            .shapes_at(p)
            .iter()
            .filter(|s| s.opacity > 0.0)
            .map(Shape::draw)
            .collect();
        fframes::svgr!(<g>{shapes}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Formula, PartSource};
    use super::*;

    fn sample(name: &str, parts: &[&str]) -> Formula {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/latex/samples/");
        let svg = std::fs::read_to_string(format!("{dir}{name}.svg")).unwrap();
        let parts = parts
            .iter()
            .enumerate()
            .map(|(index, tex)| PartSource {
                index,
                tex: (*tex).to_owned(),
            })
            .collect();
        Formula::from_svg(name, &svg, parts).unwrap()
    }

    #[test]
    fn equal_parts_and_glyphs_match_and_the_rest_fades() {
        // {{a}} + {{b}} = {{c}} into {{a}}^2 (parts-sup: a, 2)
        let from = sample("parts-bin", &["a", "b", "c"]);
        let to = sample("parts-sup", &["a"]);
        let t = TransformMatchingTex::new(FormulaView::new(&from), FormulaView::new(&to));
        // a matches a; +, b, =, c fade out; 2 fades in
        assert_eq!(t.matching(), (1, 4, 1));
        let start = t.shapes_at(0.0);
        let end = t.shapes_at(1.0);
        // the matched a ends exactly on the target a
        let target = FormulaView::new(&to).shapes()[0].path.bounding_box();
        let moved = end[0].path.bounding_box();
        assert!((moved.x0 - target.x0).abs() < 1e-9 && (moved.y1 - target.y1).abs() < 1e-9);
        assert!(start[1..5].iter().all(|s| s.opacity == 1.0));
        assert!(end[1..5].iter().all(|s| s.opacity == 0.0));
        assert_eq!(end[5].opacity, 1.0);
    }

    #[test]
    fn glyphs_outside_parts_match_by_outline() {
        // the fraction and the marked fraction have the same unmarked glyphs, a + b, c
        let from = sample("fraction", &[]);
        let to = sample("fraction", &[]);
        let t = TransformMatchingTex::new(FormulaView::new(&from), FormulaView::new(&to));
        assert_eq!(t.matching(), (5, 0, 0));
        let still = t.shapes_at(0.5);
        let same = FormulaView::new(&to).shapes();
        for (a, b) in still.iter().zip(&same) {
            assert!((a.path.bounding_box().x0 - b.path.bounding_box().x0).abs() < 1e-9);
        }
    }
}
