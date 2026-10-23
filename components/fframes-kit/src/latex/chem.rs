//! `ChemEq`: a reaction in mhchem notation with one part per species.

use super::{Formula, MathTex, TexEngine, TexError, TexTemplate};

/// Whether `token` is an mhchem arrow (`->`, `<=>`, `->[\Delta]`, ...).
fn is_arrow(token: &str) -> bool {
    ["->", "<-", "<->", "<=>", "<=>>", "<<=>", "<-->"]
        .iter()
        .any(|a| token == *a || token.starts_with(&format!("{a}[")))
}

/// The `MathTex` source of a reaction: every species becomes its own `{{\ce{..}}}` part, the
/// `+` signs and arrows stay outside the parts.
pub fn chem_source(ce: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut species: Vec<&str> = Vec::new();
    let flush = |species: &mut Vec<&str>, out: &mut Vec<String>| {
        if !species.is_empty() {
            out.push(format!("{{{{\\ce{{{}}}}}}}", species.join(" ")));
            species.clear();
        }
    };
    for token in ce.split_whitespace() {
        if token == "+" {
            flush(&mut species, &mut out);
            out.push("+".to_owned());
        } else if is_arrow(token) {
            flush(&mut species, &mut out);
            out.push(format!("\\ce{{{token}}}"));
        } else {
            species.push(token);
        }
    }
    flush(&mut species, &mut out);
    out.join(" ")
}

/// A chemical equation (`\ce{..}` with [`TexTemplate::chemistry`]): parts are the species,
/// so `color_map` and `TransformMatchingTex` address `2H2O` by its index. Separate `+` and
/// arrows from the species by spaces, as mhchem asks anyway.
#[derive(Debug, Clone, PartialEq)]
pub struct ChemEq<'s> {
    pub ce: &'s str,
    pub template: TexTemplate,
}

impl<'s> ChemEq<'s> {
    pub fn new(ce: &'s str) -> Self {
        Self {
            ce,
            template: TexTemplate::chemistry(),
        }
    }

    pub fn template(self, template: TexTemplate) -> Self {
        Self { template, ..self }
    }

    /// Runs TeX (or reads the cache). Call it in the video's constructor.
    pub fn typeset(&self, engine: &TexEngine) -> Result<Formula, TexError> {
        let source = chem_source(self.ce);
        let mut formula = MathTex::new(&source)
            .template(self.template.clone())
            .typeset(engine)?;
        self.ce.clone_into(&mut formula.source);
        Ok(formula)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_become_parts() {
        assert_eq!(
            chem_source("2H2 + O2 -> 2H2O"),
            "{{\\ce{2H2}}} + {{\\ce{O2}}} \\ce{->} {{\\ce{2H2O}}}"
        );
        assert_eq!(
            chem_source("CaCO3 ->[\\Delta] CaO + CO2 ^"),
            "{{\\ce{CaCO3}}} \\ce{->[\\Delta]} {{\\ce{CaO}}} + {{\\ce{CO2 ^}}}"
        );
    }

    /// Runs the installed TeX; prints why and passes when there is none.
    #[test]
    fn typesets_each_species_as_a_part() {
        use super::super::{Part, TexCompiler, TexTools};
        if TexTools::discover()
            .require(TexCompiler::Latex, "test")
            .is_err()
        {
            eprintln!("no TeX installed: skipped");
            return;
        }
        let dir =
            std::env::temp_dir().join(format!("fframes-kit-chem-test-{}", std::process::id()));
        let engine = TexEngine::new(&dir);
        let f = ChemEq::new("2H2 + O2 -> 2H2O").typeset(&engine).unwrap();
        assert_eq!(f.source, "2H2 + O2 -> 2H2O");
        let boxes: Vec<_> = (0..3)
            .map(|i| f.part_bbox(Part::Index(i)).unwrap())
            .collect();
        assert!(
            boxes[0].x1 < boxes[1].x0 && boxes[1].x1 < boxes[2].x0,
            "{boxes:?}"
        );
        assert!(f.part_bbox(Part::Index(3)).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
