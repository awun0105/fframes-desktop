//! LaTeX formulas through the user's TeX installation (feature `latex`): Manim's pipeline of
//! `latex` to DVI and `dvisvgm` to paths, cached by a hash of the input.

mod cache;
mod component;
mod discover;
mod engine;
mod error;
mod parts;
#[cfg(not(target_arch = "wasm32"))]
mod run;
mod svg_to_glyphs;
mod template;

pub use component::{
    Anchor, Formula, FormulaView, MathTex, PX_PER_PT, Part, Tex, UNSET_FORMULA_FONT,
};
pub use discover::{TEX_BIN_VAR, TexTools, install_help};
pub use engine::TexEngine;
pub use error::TexError;
pub use parts::{PartSource, split_parts};
pub use svg_to_glyphs::TexGlyph;
pub use template::{PLACEHOLDER, TexCompiler, TexTemplate};
