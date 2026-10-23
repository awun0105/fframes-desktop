use std::{error::Error, fmt};

/// Why a formula could not be typeset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TexError {
    /// A tool is not installed and the formula is not in the cache. `help` says how to
    /// install TeX on this system; `formula` is the source that was asked for.
    NotInstalled {
        tool: String,
        help: String,
        formula: String,
    },
    /// TeX stopped with an error. `log` holds the `.log` from the first `!` line on.
    Compile { log: String, source: String },
    /// `dvisvgm` failed, or wrote an SVG that cannot be read.
    Convert(String),
    /// A tool ran longer than the engine's timeout. MiKTeX installs a missing package on
    /// first use; with automatic installation off it waits for a dialog instead.
    Timeout { tool: String, seconds: u64 },
    /// Reading or writing the cache or the temporary files failed.
    Io(String),
}

impl fmt::Display for TexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TexError::NotInstalled {
                tool,
                help,
                formula,
            } => write!(
                f,
                "`{tool}` was not found and the formula is not in the TeX cache: {formula}\n{help}"
            ),
            TexError::Compile { log, source } => {
                write!(f, "TeX failed:\n{log}\n--- source ---\n{source}")
            }
            TexError::Convert(message) => write!(f, "dvisvgm failed: {message}"),
            TexError::Timeout { tool, seconds } => write!(
                f,
                "`{tool}` ran longer than {seconds} s. On MiKTeX, enable automatic package \
                 installation so a missing package does not wait for a dialog: \
                 initexmf --set-config-value [MPM]AutoInstall=1"
            ),
            TexError::Io(message) => write!(f, "TeX files: {message}"),
        }
    }
}

impl Error for TexError {}

impl From<std::io::Error> for TexError {
    fn from(err: std::io::Error) -> Self {
        TexError::Io(err.to_string())
    }
}
