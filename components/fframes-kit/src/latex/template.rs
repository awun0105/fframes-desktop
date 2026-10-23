use std::borrow::Cow;

/// The TeX program that typesets the document. All three produce DVI (XDV for XeLaTeX)
/// that `dvisvgm` turns into paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TexCompiler {
    #[default]
    Latex,
    XeLatex,
    LuaLatex,
}

impl TexCompiler {
    /// The executable's name.
    pub fn program(self) -> &'static str {
        match self {
            TexCompiler::Latex => "latex",
            TexCompiler::XeLatex => "xelatex",
            TexCompiler::LuaLatex => "lualatex",
        }
    }

    /// Arguments before the `.tex` file, besides the interaction flags.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn output_args(self) -> &'static [&'static str] {
        match self {
            TexCompiler::Latex => &[],
            TexCompiler::XeLatex => &["-no-pdf"],
            TexCompiler::LuaLatex => &["--output-format=dvi"],
        }
    }

    /// Extension of the file the compiler writes.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn output_extension(self) -> &'static str {
        match self {
            TexCompiler::XeLatex => "xdv",
            _ => "dvi",
        }
    }
}

/// The placeholder in [`TexTemplate::body_wrapper`] that the formula replaces (Manim's).
pub const PLACEHOLDER: &str = "YourTextHere";

/// The macros that mark parts: dvisvgm turns the raw specials into `<g id="part-N">` and
/// `<g id="label-name">` around the glyphs between them. Both keep the body a braced group,
/// so a marked part is spaced like Manim's `{{..}}`. `\part{name}{..}` replaces LaTeX's
/// sectioning command, which a formula has no use for. Every document defines them right
/// after `\begin{document}`, so they work with any preamble.
const PART_MACROS: &str = "\\def\\fframesPart#1#2{{\\special{dvisvgm:raw <g id='part-#1'>}#2\\special{dvisvgm:raw </g>}}}\n\
\\def\\part#1#2{{\\special{dvisvgm:raw <g id='label-#1'>}#2\\special{dvisvgm:raw </g>}}}\n";

/// The preamble lines every template starts with: the `standalone` class, which crops the
/// page to its content, and `xcolor` for colored parts.
const BASE: &str = "\\documentclass[preview]{standalone}\n\\usepackage{xcolor}\n";

/// A document around a formula (Manim's `TexTemplate`): the compiler, the preamble and how
/// the body is wrapped (`body_wrapper` contains [`PLACEHOLDER`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TexTemplate {
    pub compiler: TexCompiler,
    pub preamble: Cow<'static, str>,
    pub body_wrapper: Cow<'static, str>,
}

impl Default for TexTemplate {
    /// `standalone` with amsmath and amssymb, compiled with `latex`.
    fn default() -> Self {
        Self {
            compiler: TexCompiler::Latex,
            preamble: Cow::Owned(format!(
                "{BASE}\\usepackage{{amsmath}}\n\\usepackage{{amssymb}}\n"
            )),
            body_wrapper: Cow::Borrowed(PLACEHOLDER),
        }
    }
}

impl TexTemplate {
    /// The default with mhchem (`\ce{..}`) and chemfig (`\chemfig{..}`).
    pub fn chemistry() -> Self {
        Self::default().add_to_preamble("\\usepackage[version=4]{mhchem}\n\\usepackage{chemfig}\n")
    }

    /// The default with physics (`\dv`, `\bra`, `\ket`, ...) and siunitx (`\SI`, `\qty`).
    pub fn physics() -> Self {
        Self::default().add_to_preamble("\\usepackage{physics}\n\\usepackage{siunitx}\n")
    }

    /// The default with TikZ, using the dvisvgm driver so pictures come out as paths.
    pub fn tikz() -> Self {
        Self::default()
            .add_to_preamble("\\def\\pgfsysdriver{pgfsys-dvisvgm.def}\n\\usepackage{tikz}\n")
    }

    /// The same template with `lines` appended to the preamble (packages, macros).
    pub fn add_to_preamble(mut self, lines: &str) -> Self {
        self.preamble.to_mut().push_str(lines);
        self
    }

    /// The same template with another compiler (`XeLatex` for system fonts with fontspec).
    pub fn with_compiler(self, compiler: TexCompiler) -> Self {
        Self { compiler, ..self }
    }

    /// The whole `.tex` file for `body`.
    pub fn document(&self, body: &str) -> String {
        let body = self.body_wrapper.replace(PLACEHOLDER, body);
        format!(
            "{}\\begin{{document}}\n{PART_MACROS}{body}\n\\end{{document}}\n",
            self.preamble
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_extend_the_default_preamble() {
        let base = TexTemplate::default();
        assert!(base.preamble.contains("amsmath"));
        let chem = TexTemplate::chemistry();
        assert!(chem.preamble.starts_with(base.preamble.as_ref()));
        assert!(chem.preamble.contains("mhchem"));
        let tikz = TexTemplate::tikz();
        // the driver must be chosen before TikZ loads
        assert!(tikz.preamble.find("pgfsysdriver") < tikz.preamble.find("{tikz}"));
    }

    #[test]
    fn the_body_replaces_the_placeholder() {
        let t = TexTemplate {
            body_wrapper: "$ YourTextHere $".into(),
            ..TexTemplate::default()
        };
        let doc = t.document("x^2");
        assert!(doc.contains("$ x^2 $\n\\end{document}"));
        // the part macros come with every document, whatever the preamble
        let bare = TexTemplate {
            preamble: "\\documentclass{standalone}\n".into(),
            ..t
        };
        assert!(
            bare.document("x")
                .contains("\\begin{document}\n\\def\\fframesPart")
        );
    }
}
