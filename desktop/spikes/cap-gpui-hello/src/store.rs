//! Stand-in for Cap's settings store: only the theme preference `theme.rs` reads.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Default)]
pub struct GeneralSettings {
    pub theme: AppTheme,
}

impl GeneralSettings {
    pub fn load() -> Self {
        Self::default()
    }
}
