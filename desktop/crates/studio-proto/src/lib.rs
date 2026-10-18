//! Identities shared by the studio engine and its UI.
//!
//! Every event the engine sends about an open project carries the [`SessionId`] of the
//! project session it came from, so a window drops what a closed project still says. A
//! [`CommandId`] pairs a command with the event that reports its completion.

use std::fmt;

use serde::{Deserialize, Serialize};

/// One opened project, from open until it is closed or replaced. Reopening the folder that
/// is already open keeps its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub u64);

/// One command sent to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session {}", self.0)
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "command {}", self.0)
    }
}

/// The appearance the user chose. `System` follows the operating system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    #[default]
    System,
    Light,
    Dark,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_theme_is_stored_by_lowercase_name() {
        for (theme, name) in [
            (AppTheme::System, "\"system\""),
            (AppTheme::Light, "\"light\""),
            (AppTheme::Dark, "\"dark\""),
        ] {
            assert_eq!(serde_json::to_string(&theme).unwrap(), name);
            assert_eq!(serde_json::from_str::<AppTheme>(name).unwrap(), theme);
        }
        assert_eq!(AppTheme::default(), AppTheme::System);
    }
}
