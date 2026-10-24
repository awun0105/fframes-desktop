//! Phylogenetic trees read from Newick and drawn as cladograms or, with branch lengths, as
//! phylograms.

use std::{error::Error, fmt};

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    kits::draw::{mix, partial, stroke},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// A node of a [`Newick`] tree.
#[derive(Debug, Clone, PartialEq)]
pub struct NewickNode {
    /// The name, empty when it has none; `_` in unquoted names reads as a space.
    pub name: String,
    /// The length of the branch into it.
    pub length: Option<f64>,
    pub children: Vec<usize>,
}

/// A tree in Newick format: `(A:1,(B:2,C:2)BC:1)root;`.
#[derive(Debug, Clone, PartialEq)]
pub struct Newick {
    pub nodes: Vec<NewickNode>,
    pub root: usize,
}

/// Why a Newick string could not be read; `at` is a character index.
#[derive(Debug, Clone, PartialEq)]
pub enum NewickError {
    Unexpected { at: usize, found: char },
    UnexpectedEnd,
    BadLength { at: usize },
}

impl fmt::Display for NewickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NewickError::Unexpected { at, found } => {
                write!(f, "unexpected `{found}` at character {at}")
            }
            NewickError::UnexpectedEnd => write!(f, "the tree ends before it is closed"),
            NewickError::BadLength { at } => write!(f, "not a branch length at character {at}"),
        }
    }
}

impl Error for NewickError {}

struct Parser {
    chars: Vec<char>,
    at: usize,
    nodes: Vec<NewickNode>,
}

impl Parser {
    fn skip_space(&mut self) {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_space();
        self.chars.get(self.at).copied()
    }

    fn name(&mut self) -> Result<String, NewickError> {
        if self.peek() == Some('\'') {
            self.at += 1;
            let mut name = String::new();
            loop {
                match self.chars.get(self.at) {
                    None => return Err(NewickError::UnexpectedEnd),
                    // a doubled quote is a quote
                    Some('\'') if self.chars.get(self.at + 1) == Some(&'\'') => {
                        name.push('\'');
                        self.at += 2;
                    }
                    Some('\'') => {
                        self.at += 1;
                        return Ok(name);
                    }
                    Some(&c) => {
                        name.push(c);
                        self.at += 1;
                    }
                }
            }
        }
        let start = self.at;
        while self
            .chars
            .get(self.at)
            .is_some_and(|c| !"(),:;'".contains(*c))
        {
            self.at += 1;
        }
        let raw: String = self.chars[start..self.at].iter().collect();
        Ok(raw.trim().replace('_', " "))
    }

    fn length(&mut self) -> Result<Option<f64>, NewickError> {
        if self.peek() != Some(':') {
            return Ok(None);
        }
        self.at += 1;
        self.skip_space();
        let start = self.at;
        while self
            .chars
            .get(self.at)
            .is_some_and(|c| c.is_ascii_digit() || "+-.eE".contains(*c))
        {
            self.at += 1;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        text.parse()
            .map(Some)
            .map_err(|_| NewickError::BadLength { at: start })
    }

    fn subtree(&mut self) -> Result<usize, NewickError> {
        let mut children = Vec::new();
        if self.peek() == Some('(') {
            self.at += 1;
            loop {
                children.push(self.subtree()?);
                match self.peek() {
                    Some(',') => self.at += 1,
                    Some(')') => {
                        self.at += 1;
                        break;
                    }
                    Some(found) => return Err(NewickError::Unexpected { at: self.at, found }),
                    None => return Err(NewickError::UnexpectedEnd),
                }
            }
        }
        let name = self.name()?;
        let length = self.length()?;
        self.nodes.push(NewickNode {
            name,
            length,
            children,
        });
        Ok(self.nodes.len() - 1)
    }
}

impl Newick {
    /// Reads a tree. The closing `;` may be left out.
    pub fn parse(text: &str) -> Result<Self, NewickError> {
        let mut parser = Parser {
            chars: text.chars().collect(),
            at: 0,
            nodes: Vec::new(),
        };
        if parser.peek().is_none() {
            return Err(NewickError::UnexpectedEnd);
        }
        let root = parser.subtree()?;
        if parser.peek() == Some(';') {
            parser.at += 1;
        }
        if let Some(found) = parser.peek() {
            return Err(NewickError::Unexpected {
                at: parser.at,
                found,
            });
        }
        Ok(Self {
            nodes: parser.nodes,
            root,
        })
    }

    /// The names of the leaves, top to bottom.
    pub fn leaves(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.walk(self.root, &mut |node, _| {
            if self.nodes[node].children.is_empty() {
                out.push(self.nodes[node].name.as_str());
            }
        });
        out
    }

    /// Visits `node` and everything under it, parents first, with each one's depth.
    fn walk(&self, node: usize, visit: &mut impl FnMut(usize, usize)) {
        fn go(tree: &Newick, node: usize, depth: usize, visit: &mut impl FnMut(usize, usize)) {
            visit(node, depth);
            for &child in &tree.nodes[node].children {
                go(tree, child, depth + 1, visit);
            }
        }
        go(self, node, 0, visit);
    }

    /// Where each node sits: `x` from the root (its depth, or the sum of the branch lengths
    /// when every branch has one; leaves line up at the deepest leaf when they do not) and
    /// `y` in rows (leaves one row apart in order, parents midway between their first and
    /// last child), with the depth of each node.
    fn layout(&self) -> Vec<(f64, f64, usize)> {
        fn rows(tree: &Newick, node: usize, next: &mut f64, placed: &mut [(f64, f64, usize)]) {
            let children = &tree.nodes[node].children;
            if children.is_empty() {
                placed[node].1 = *next;
                *next += 1.0;
                return;
            }
            for &child in children {
                rows(tree, child, next, placed);
            }
            let (first, last) = (children[0], children[children.len() - 1]);
            placed[node].1 = f64::midpoint(placed[first].1, placed[last].1);
        }

        let mut placed = vec![(0.0, 0.0, 0); self.nodes.len()];
        let lengths = self
            .nodes
            .iter()
            .enumerate()
            .all(|(i, n)| i == self.root || n.length.is_some());
        let mut deepest = 0usize;
        self.walk(self.root, &mut |node, depth| {
            placed[node].2 = depth;
            deepest = deepest.max(depth);
        });
        // x, parents first
        self.walk(self.root, &mut |node, _| {
            for &child in &self.nodes[node].children {
                placed[child].0 = if lengths {
                    placed[node].0 + self.nodes[child].length.unwrap_or(0.0)
                } else if self.nodes[child].children.is_empty() {
                    deepest as f64
                } else {
                    placed[node].0 + 1.0
                };
            }
        });
        // y, children first
        rows(self, self.root, &mut 0.0, &mut placed);
        placed
    }
}

/// A phylogenetic tree from `newick`, growing from the root at the left, `width` pixels from
/// the root to the leaves and `row` pixels between leaves, centred on `center` (units of
/// `ui.space`). With branch lengths for every branch it is drawn to scale; without, as a
/// cladogram with the leaves in a column. From `delay` the branches draw outward one level
/// every `step` seconds, and each leaf's name appears when its branch arrives. Named inner
/// nodes are traits shared by everything after them, written on the branch that leads to
/// them. With `highlight` naming a node, its clade turns `highlight_color` at `highlight_at`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PhyloTree {
    pub newick: String,
    pub delay: f32,
    pub step: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub row: f32,
    pub highlight: Option<String>,
    pub highlight_at: f32,
    pub branch_color: Color,
    pub highlight_color: Color,
    pub trait_color: Color,
    pub name_style: TextStyle,
    pub trait_style: TextStyle,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for PhyloTree {
    fn themed(t: &Theme) -> Self {
        let mut trait_style = t.typography.caption;
        trait_style.size = 22;
        Self {
            newick: "(Lamprey,(Shark,(Cod,(Frog,(Lizard,(Mouse,Human)hair)'amniotic egg')\
                     'four limbs')'bony skeleton')jaws)vertebrae;"
                .into(),
            delay: 0.0,
            step: 0.55,
            center: (-0.7, -0.4),
            width: 1000.0,
            row: 66.0,
            highlight: None,
            highlight_at: 0.0,
            branch_color: t.colors.muted,
            highlight_color: t.colors.accent,
            trait_color: Color::hex("#ffd43b"),
            name_style: t.typography.caption,
            trait_style,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for PhyloTree {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for PhyloTree {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let tree = match Newick::parse(&self.newick) {
            Ok(tree) => tree,
            Err(err) => {
                let at = ui.space.to_px(self.center.0, self.center.1);
                let text = format!("Newick: {err}");
                return line(
                    at.x as f32,
                    at.y as f32,
                    self.name_style,
                    self.text_color,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    text.into(),
                );
            }
        };
        let placed = tree.layout();
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let span_x = placed.iter().map(|p| p.0).fold(0.0, f64::max).max(1e-9);
        let span_y = placed.iter().map(|p| p.1).fold(0.0, f64::max);
        let (width, row) = (f64::from(self.width), f64::from(self.row));
        let left = middle.x - width / 2.0;
        let top = middle.y - span_y * row / 2.0;
        let at = |node: usize| {
            Point::new(
                left + placed[node].0 / span_x * width,
                top + placed[node].1 * row,
            )
        };
        let t = ui.seconds() - self.delay;

        // the clade to highlight
        let mut lit = vec![false; tree.nodes.len()];
        if let Some(name) = &self.highlight
            && let Some(node) = tree.nodes.iter().position(|n| &n.name == name)
        {
            tree.walk(node, &mut |n, _| lit[n] = true);
        }
        let turned = cubic_in_out(progress(
            ui.seconds(),
            self.highlight_at,
            self.highlight_at + 0.4,
        ));
        let color_of = |node: usize, base: Color| {
            if lit[node] {
                mix(base, self.highlight_color, turned)
            } else {
                base
            }
        };

        // long enough for the name of the root's trait
        let stub = 120.0;
        let stub_time = 0.4;
        let mut branches = Vec::new();
        let mut marks = Vec::new();
        let mut names = Vec::new();
        // the root's stem, and its trait on it
        let root = at(tree.root);
        let stem = progress(t, 0.0, stub_time);
        if stem > 0.0 {
            let reached = partial(&[Point::new(root.x - stub, root.y), root], f64::from(stem));
            branches.push(polyline(&reached, color_of(tree.root, self.branch_color)));
        }
        let mut traits = vec![(
            tree.root,
            Point::new(root.x - stub / 2.0, root.y),
            stub_time * 0.5,
        )];
        for (node, &(_, _, depth)) in placed.iter().enumerate() {
            let Some(parent) = tree.nodes.iter().position(|n| n.children.contains(&node)) else {
                continue;
            };
            let (from, to) = (at(parent), at(node));
            let start = stub_time + (depth - 1) as f32 * self.step;
            let drawn = cubic_in_out(progress(t, start, start + self.step));
            if drawn <= 0.0 {
                continue;
            }
            let path = [from, Point::new(from.x, to.y), to];
            let reached = partial(&path, f64::from(drawn));
            branches.push(polyline(&reached, color_of(node, self.branch_color)));
            let name = &tree.nodes[node].name;
            if tree.nodes[node].children.is_empty() {
                let shown = progress(t, start + self.step * 0.8, start + self.step * 1.3);
                names.push(line(
                    (to.x + 16.0) as f32,
                    (to.y + self.name_style.size as f64 * 0.35) as f32,
                    self.name_style,
                    color_of(node, self.text_color),
                    TextAnchor::Start,
                    0.0,
                    shown,
                    name.clone().into(),
                ));
            } else {
                let mark = Point::new(f64::midpoint(from.x, to.x), to.y);
                traits.push((node, mark, start + self.step * 0.5));
            }
        }
        for (node, mark, start) in traits {
            let name = &tree.nodes[node].name;
            let shown = progress(t, start, start + 0.3);
            if name.is_empty() || shown <= 0.0 {
                continue;
            }
            marks.push(
                Shape {
                    path: Circle::new(mark, 6.0).to_path(0.1),
                    fill: Some(self.trait_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: shown,
                }
                .draw(),
            );
            marks.push(line(
                mark.x as f32,
                (mark.y - 14.0) as f32,
                self.trait_style,
                self.trait_color,
                TextAnchor::Middle,
                0.0,
                shown,
                name.clone().into(),
            ));
        }

        let parts: Svgr<'a> = vec![branches.into(), marks.into(), names.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

fn polyline<'a>(points: &[Point], color: Color) -> Svgr<'a> {
    let mut path = BezPath::new();
    for (i, &p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    stroke(path, color, 4.0, 1.0).draw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newick_reads_names_lengths_and_quotes() {
        let tree = Newick::parse("(A:1,(B_b:2.5,'C ''c''':2)BC:1e0)root;").unwrap();
        assert_eq!(tree.leaves(), ["A", "B b", "C 'c'"]);
        let bc = tree.nodes.iter().find(|n| n.name == "BC").unwrap();
        assert_eq!(bc.length, Some(1.0));
        assert_eq!(bc.children.len(), 2);
        assert_eq!(tree.nodes[tree.root].name, "root");
        assert_eq!(tree.nodes[tree.root].length, None);
        assert_eq!(Newick::parse("(A,B)").unwrap().leaves(), ["A", "B"]);
    }

    #[test]
    fn bad_newick_says_where() {
        assert_eq!(Newick::parse("(A,B"), Err(NewickError::UnexpectedEnd));
        assert_eq!(
            Newick::parse("(A:x,B);"),
            Err(NewickError::BadLength { at: 3 })
        );
        assert_eq!(
            Newick::parse("(A,B);C"),
            Err(NewickError::Unexpected { at: 6, found: 'C' })
        );
        assert_eq!(Newick::parse("  "), Err(NewickError::UnexpectedEnd));
    }

    #[test]
    fn leaves_line_up_and_parents_sit_between_their_children() {
        let tree = Newick::parse(&PhyloTree::default().newick).unwrap();
        let placed = tree.layout();
        let leaves: Vec<usize> = (0..tree.nodes.len())
            .filter(|&n| tree.nodes[n].children.is_empty())
            .collect();
        assert_eq!(leaves.len(), 7);
        // a cladogram: every leaf in the last column, one row apart in order
        let mut rows: Vec<f64> = leaves.iter().map(|&n| placed[n].1).collect();
        rows.sort_by(f64::total_cmp);
        assert_eq!(rows, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert!(leaves.iter().all(|&n| placed[n].0 == 6.0));
        for node in &tree.nodes {
            if let [first, .., last] = node.children[..] {
                let parent = tree
                    .nodes
                    .iter()
                    .position(|n| std::ptr::eq(n, node))
                    .unwrap();
                assert_eq!(
                    placed[parent].1,
                    f64::midpoint(placed[first].1, placed[last].1)
                );
            }
        }
        // with lengths, x is the distance from the root
        let scaled = Newick::parse("(A:1,(B:2,C:3):1);").unwrap();
        let placed = scaled.layout();
        let x: Vec<f64> = scaled
            .leaves()
            .iter()
            .map(|name| {
                let n = scaled.nodes.iter().position(|n| n.name == *name).unwrap();
                placed[n].0
            })
            .collect();
        assert_eq!(x, [1.0, 3.0, 4.0]);
    }
}
