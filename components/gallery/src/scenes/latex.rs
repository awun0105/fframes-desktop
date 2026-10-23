//! LaTeX formulas, one scene each. TeX runs once, when the scenes are built; the SVGs are
//! cached in `gallery/tex-cache`, which is committed, so machines without TeX render them.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

/// The scene with `title`, as in the other gallery files.
macro_rules! scenes {
    ($($name:literal, $seconds:expr => |$ui:ident| $body:expr,)*) => {
        vec![$(GalleryScene::new($name, $seconds, |$ui| {
            let head = title($ui, $name);
            let body: Svgr<'_> = $body;
            svgr!(<g>{head}{body}</g>)
        }),)*]
    };
}

#[derive(Debug)]
struct Formulas {
    euler: Formula,
    sum: Formula,
    pythagoras: Formula,
    solved: Formula,
    water: Formula,
    tikz: Formula,
}

/// Typesets `tex`, or keeps its source to draw as text when TeX is missing and the cache
/// has no entry (the editor).
fn typeset(tex: &str, result: Result<Formula, TexError>) -> Formula {
    result.unwrap_or_else(|e| {
        eprintln!("gallery: {e}");
        Formula::unset(tex)
    })
}

impl Formulas {
    fn typeset() -> Self {
        let engine = TexEngine::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache"));
        let math = |tex: &str| typeset(tex, MathTex::new(tex).typeset(&engine));
        let water = "2H2 + O2 -> 2H2O";
        let tikz = "\\begin{tikzpicture}[thick]\
            \\draw[->] (0,0) -- (4,0) node[right] {$x$};\
            \\draw[->] (0,0) -- (0,2.6) node[above] {$y$};\
            \\draw[blue, domain=0:3.6, samples=40] plot (\\x, {1.2 + sin(\\x r)});\
            \\fill[red] (1.57,2.2) circle (0.08);\
            \\end{tikzpicture}";
        Self {
            euler: math("e^{i\\pi} + 1 = 0"),
            sum: math("\\sum_{k=1}^{n} k = \\frac{n(n+1)}{2}"),
            pythagoras: math("{{a}}^2 + {{b}}^2 = {{c}}^2"),
            solved: math("{{a}}^2 = {{c}}^2 - {{b}}^2"),
            water: typeset(water, ChemEq::new(water).typeset(&engine)),
            tikz: typeset(
                tikz,
                Tex::new(tikz)
                    .template(TexTemplate::tikz())
                    .typeset(&engine),
            ),
        }
    }
}

static FORMULAS: OnceLock<Formulas> = OnceLock::new();

fn formulas() -> &'static Formulas {
    FORMULAS.get_or_init(Formulas::typeset)
}

/// `of` centred on the canvas below the title, 10 pt as `size` pixels.
fn centered<'f>(ui: &Ui<'_, '_, '_>, of: &'f Formula, size: f32) -> FormulaView<'f> {
    FormulaView {
        size,
        x: 960.0,
        y: 600.0,
        anchor: Anchor::Center,
        ..FormulaView::new(of).themed(ui.theme)
    }
}

/// Scenes that measure the cost of formulas per frame against plain text.
pub fn bench_scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("BenchFormulas", 5.0, bench_formulas),
        GalleryScene::new("BenchFormulaText", 5.0, bench_formula_text),
    ]
}

const BENCH_FORMULAS: usize = 20;

/// Center of grid cell `i` (4 columns, 5 rows).
fn bench_cell(i: usize) -> (f32, f32) {
    (
        270.0 + 460.0 * (i % 4) as f32,
        200.0 + 180.0 * (i / 4) as f32,
    )
}

fn bench_formula(i: usize) -> &'static Formula {
    let f = formulas();
    [&f.euler, &f.sum, &f.pythagoras, &f.water][i % 4]
}

fn bench_formulas<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    (0..BENCH_FORMULAS)
        .map(|i| {
            let (x, y) = bench_cell(i);
            let view = FormulaView {
                x,
                y,
                ..centered(ui, bench_formula(i), 40.0)
            };
            ui.show(view)
        })
        .collect::<Vec<_>>()
        .into()
}

fn bench_formula_text<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let style = ui.theme.typography.body;
    let fill = ui.theme.colors.title;
    (0..BENCH_FORMULAS)
        .map(|i| {
            let (x, y) = bench_cell(i);
            let text = bench_formula(i).source.clone();
            svgr!(<text x={x} y={y} text-anchor="middle" font-family={style.family}
                        font-size={style.size} fill={fill}>{text}</text>)
        })
        .collect::<Vec<_>>()
        .into()
}

pub fn scenes() -> Vec<GalleryScene> {
    // typeset now, in the video's constructor, not in the first frame
    formulas();
    scenes![
        "MathTex", 2.0 => |ui| {
            let f = formulas();
            let euler = FormulaView { y: 470.0, ..centered(ui, &f.euler, 110.0) };
            let sum = FormulaView {
                y: 730.0,
                color: ui.theme.colors.accent,
                ..centered(ui, &f.sum, 90.0)
            };
            let (euler, sum) = (ui.show(euler), ui.show(sum));
            svgr!(<g>{euler}{sum}</g>)
        },
        "WriteFormula", 2.5 => |ui| {
            let view = FormulaView {
                enter: Some(Write { run_time: 1.8, ..ui.defaults() }.into()),
                ..centered(ui, &formulas().sum, 110.0)
            };
            ui.show(view)
        },
        "FormulaParts", 2.5 => |ui| {
            let f = &formulas().pythagoras;
            let map = [
                (Part::Index(0), ui.theme.colors.accent),
                (Part::Index(1), ui.theme.colors.highlight),
            ];
            let view = FormulaView {
                color_map: &map,
                emphasis: Some(Indicate { delay: 0.6, ..ui.defaults() }.into()),
                emphasize: Some(Part::Index(2)),
                ..centered(ui, f, 130.0)
            };
            // a box around the third part, placed from the formula's own geometry
            let frame = view.part_bbox(Part::Index(2)).map(|b| {
                let b = b.inflate(16.0, 16.0);
                let (x, y, w, h) = (b.x0, b.y0, b.width(), b.height());
                svgr!(<rect x={x} y={y} width={w} height={h} rx="10" fill="none"
                            stroke={ui.theme.colors.muted} stroke-width="3" />)
            });
            let formula = ui.show(view);
            svgr!(<g>{formula}{frame.unwrap_or_else(Svgr::empty)}</g>)
        },
        "TransformMatchingTex", 2.5 => |ui| {
            let f = formulas();
            let morph = TransformMatchingTex {
                delay: 0.5,
                run_time: 1.5,
                ..TransformMatchingTex::new(centered(ui, &f.pythagoras, 130.0), centered(ui, &f.solved, 130.0))
            };
            ui.show(morph)
        },
        "ChemEq", 2.5 => |ui| {
            let map = [
                (Part::Index(0), ui.theme.colors.accent),
                (Part::Index(1), ui.theme.colors.highlight),
                (Part::Index(2), Color::hex("#7cd992")),
            ];
            let view = FormulaView {
                color_map: &map,
                enter: Some(Write { run_time: 1.5, ..ui.defaults() }.into()),
                ..centered(ui, &formulas().water, 130.0)
            };
            ui.show(view)
        },
        "TikzDiagram", 2.5 => |ui| {
            let view = FormulaView {
                enter: Some(Create { run_time: 1.8, ..ui.defaults() }.into()),
                ..centered(ui, &formulas().tikz, 60.0)
            };
            ui.show(view)
        },
    ]
}
