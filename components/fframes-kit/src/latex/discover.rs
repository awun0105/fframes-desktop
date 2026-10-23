//! Finding `latex`, `xelatex`, `lualatex` and `dvisvgm`.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use super::{TexCompiler, TexError};

/// The environment variable naming the directory of the TeX tools. When it is set, only
/// that directory is searched (point it at an empty directory to hide TeX).
pub const TEX_BIN_VAR: &str = "FFRAMES_TEX_BIN";

/// The TeX tools found on this system; `None` for a missing one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TexTools {
    pub latex: Option<PathBuf>,
    pub xelatex: Option<PathBuf>,
    pub lualatex: Option<PathBuf>,
    pub dvisvgm: Option<PathBuf>,
}

impl TexTools {
    /// Searches `FFRAMES_TEX_BIN`, else `PATH` and the default MiKTeX folders.
    pub fn discover() -> Self {
        let defaults = default_dirs();
        Self::discover_in(
            std::env::var_os(TEX_BIN_VAR),
            std::env::var_os("PATH"),
            &defaults,
        )
    }

    /// The search of [`discover`](Self::discover) with explicit inputs: `bin` overrides
    /// everything, otherwise `path` (a `PATH`-style list) and then `defaults` are searched.
    pub fn discover_in(
        bin: Option<OsString>,
        path: Option<OsString>,
        defaults: &[PathBuf],
    ) -> Self {
        let dirs: Vec<PathBuf> = match bin {
            Some(bin) if !bin.is_empty() => vec![PathBuf::from(bin)],
            _ => path
                .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter()
                .chain(defaults.iter().cloned())
                .collect(),
        };
        let find = |name: &str| dirs.iter().find_map(|dir| executable(dir, name));
        Self {
            latex: find("latex"),
            xelatex: find("xelatex"),
            lualatex: find("lualatex"),
            dvisvgm: find("dvisvgm"),
        }
    }

    /// The compiler and `dvisvgm`, or which one is missing. `formula` goes into the error.
    pub fn require(
        &self,
        compiler: TexCompiler,
        formula: &str,
    ) -> Result<(&Path, &Path), TexError> {
        let missing = |tool: &str| TexError::NotInstalled {
            tool: tool.to_owned(),
            help: install_help().to_owned(),
            formula: formula.to_owned(),
        };
        let tex = match compiler {
            TexCompiler::Latex => &self.latex,
            TexCompiler::XeLatex => &self.xelatex,
            TexCompiler::LuaLatex => &self.lualatex,
        }
        .as_deref()
        .ok_or_else(|| missing(compiler.program()))?;
        let dvisvgm = self.dvisvgm.as_deref().ok_or_else(|| missing("dvisvgm"))?;
        Ok((tex, dvisvgm))
    }
}

/// `dir/name` (`name.exe` on Windows) when it is a file.
fn executable(dir: &Path, name: &str) -> Option<PathBuf> {
    let file = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    file.is_file().then_some(file)
}

/// The folders MiKTeX installs into on Windows; nothing elsewhere (TeX Live and MacTeX put
/// their tools on `PATH`).
fn default_dirs() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let mut dirs = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join(r"Programs\MiKTeX\miktex\bin\x64"));
    }
    dirs.push(PathBuf::from(r"C:\Program Files\MiKTeX\miktex\bin\x64"));
    dirs
}

/// How to install TeX on this operating system.
pub fn install_help() -> &'static str {
    if cfg!(windows) {
        "Install MiKTeX from https://miktex.org/download (or `choco install miktex`), then \
         enable automatic package installation: initexmf --set-config-value [MPM]AutoInstall=1. \
         Set FFRAMES_TEX_BIN to its bin folder if a new terminal does not find `latex`. \
         See docs/latex-setup.md."
    } else if cfg!(target_os = "macos") {
        "Install MiKTeX from https://miktex.org/download or MacTeX \
         (brew install --cask mactex-no-gui), then open a new terminal so `latex` and \
         `dvisvgm` are on PATH. See docs/latex-setup.md."
    } else {
        "Install MiKTeX from https://miktex.org/download, or TeX Live: \
         sudo apt install texlive-latex-extra texlive-science texlive-pictures dvisvgm. \
         See docs/latex-setup.md."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory with fake tools named `names`.
    fn fake_bin(case: &str, names: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fframes-kit-discover-{case}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in names {
            std::fs::write(
                dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
                b"",
            )
            .unwrap();
        }
        dir
    }

    #[test]
    fn the_override_is_the_only_directory_searched() {
        let tex = fake_bin("override", &["latex", "dvisvgm"]);
        let empty = fake_bin("empty", &[]);
        let path = std::env::join_paths([&tex]).unwrap();
        let hidden = TexTools::discover_in(Some(empty.clone().into()), Some(path.clone()), &[]);
        assert_eq!(hidden, TexTools::default());
        let found = TexTools::discover_in(Some(tex.clone().into()), None, &[]);
        assert_eq!(
            found.latex,
            Some(tex.join(format!("latex{}", std::env::consts::EXE_SUFFIX)))
        );
        assert!(found.xelatex.is_none());
    }

    #[test]
    fn path_comes_before_the_default_folders() {
        let first = fake_bin("first", &["latex"]);
        let second = fake_bin("second", &["latex", "dvisvgm", "xelatex"]);
        let path = std::env::join_paths([&first]).unwrap();
        let tools = TexTools::discover_in(None, Some(path), std::slice::from_ref(&second));
        assert!(tools.latex.unwrap().starts_with(&first));
        assert!(tools.dvisvgm.unwrap().starts_with(&second));
        assert!(tools.lualatex.is_none());
    }

    #[test]
    fn a_missing_tool_names_itself_and_the_formula() {
        let tools = TexTools {
            latex: Some(PathBuf::from("latex")),
            ..TexTools::default()
        };
        let err = tools.require(TexCompiler::Latex, "e^{i\\pi}").unwrap_err();
        let TexError::NotInstalled {
            tool,
            help,
            formula,
        } = err
        else {
            panic!("{err:?}")
        };
        assert_eq!((tool.as_str(), formula.as_str()), ("dvisvgm", "e^{i\\pi}"));
        assert!(help.contains("miktex.org"));
        assert!(matches!(
            tools.require(TexCompiler::XeLatex, ""),
            Err(TexError::NotInstalled { tool, .. }) if tool == "xelatex"
        ));
    }
}
