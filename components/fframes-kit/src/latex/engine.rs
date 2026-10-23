use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use super::{TexError, TexTemplate, TexTools, cache::cache_file};

/// Typesets documents with the TeX found on this system and keeps every result in
/// `cache_dir`. A cached document never runs TeX, so a committed cache renders on machines
/// without it. Create one in the video's constructor (not in `render_frame`): the first
/// `typeset` of a formula runs TeX, which takes a second or more.
#[derive(Debug)]
pub struct TexEngine {
    cache_dir: PathBuf,
    tools: OnceLock<TexTools>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    timeout: Duration,
}

impl TexEngine {
    /// An engine with its cache in `cache_dir`. The path should not depend on the working
    /// directory: use `concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache")`.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            tools: OnceLock::new(),
            timeout: Duration::from_secs(120),
        }
    }

    /// The same engine with another timeout per tool run (default 120 s; MiKTeX may install
    /// a package the first time a document uses it).
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    /// The same engine with these tools instead of the ones found on this system.
    pub fn with_tools(self, tools: TexTools) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(tools);
        Self {
            tools: cell,
            ..self
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// The TeX tools, searched for the first time they are needed.
    pub fn tools(&self) -> &TexTools {
        self.tools.get_or_init(TexTools::discover)
    }

    /// The SVG of `body` in `template`, from the cache or typeset now (and cached).
    /// `formula` names the formula in errors.
    pub(crate) fn svg(
        &self,
        template: &TexTemplate,
        body: &str,
        formula: &str,
    ) -> Result<String, TexError> {
        let document = template.document(body);
        let file = cache_file(&self.cache_dir, template, &document);
        if let Ok(svg) = std::fs::read_to_string(&file) {
            return Ok(svg);
        }
        self.typeset_now(template, &document, formula, &file)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn typeset_now(
        &self,
        template: &TexTemplate,
        document: &str,
        formula: &str,
        file: &Path,
    ) -> Result<String, TexError> {
        let (tex, dvisvgm) = self.tools().require(template.compiler, formula)?;
        // dvisvgm ends lines as the platform does; one convention keeps the cache identical
        // whichever system regenerates it
        let svg = super::run::compile(tex, dvisvgm, template.compiler, document, self.timeout)?
            .replace("\r\n", "\n");
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::write(file, &svg)?;
        Ok(svg)
    }

    /// The editor has no processes: only cached formulas typeset.
    #[cfg(target_arch = "wasm32")]
    fn typeset_now(
        &self,
        template: &TexTemplate,
        _document: &str,
        formula: &str,
        _file: &Path,
    ) -> Result<String, TexError> {
        Err(TexError::NotInstalled {
            tool: template.compiler.program().to_owned(),
            help: "TeX does not run in the web editor; render natively once to fill the cache."
                .to_owned(),
            formula: formula.to_owned(),
        })
    }
}
