//! Lucide icons as gpui `svg()` elements.

use gpui::{Rgba, Styled, Svg, px, svg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    StepBack,
    StepForward,
    ZoomIn,
    ZoomOut,
    Maximize,
    Volume2,
    VolumeX,
    Speaker,
    RefreshCw,
    /// Build.
    Hammer,
    /// Stop or cancel.
    Square,
    Folder,
    FolderOpen,
    File,
    Image,
    Palette,
    Bot,
    Download,
    Upload,
    Plus,
    X,
    Check,
    ChevronDown,
    ChevronRight,
    MoreHorizontal,
    Settings,
    AlertTriangle,
    Info,
    CircleCheck,
    History,
    Save,
    Eye,
    Layers,
    Package,
    ArrowUp,
    Search,
    Trash,
}

impl Icon {
    pub const ALL: [Icon; 41] = [
        Icon::Play,
        Icon::Pause,
        Icon::SkipBack,
        Icon::SkipForward,
        Icon::StepBack,
        Icon::StepForward,
        Icon::ZoomIn,
        Icon::ZoomOut,
        Icon::Maximize,
        Icon::Volume2,
        Icon::VolumeX,
        Icon::Speaker,
        Icon::RefreshCw,
        Icon::Hammer,
        Icon::Square,
        Icon::Folder,
        Icon::FolderOpen,
        Icon::File,
        Icon::Image,
        Icon::Palette,
        Icon::Bot,
        Icon::Download,
        Icon::Upload,
        Icon::Plus,
        Icon::X,
        Icon::Check,
        Icon::ChevronDown,
        Icon::ChevronRight,
        Icon::MoreHorizontal,
        Icon::Settings,
        Icon::AlertTriangle,
        Icon::Info,
        Icon::CircleCheck,
        Icon::History,
        Icon::Save,
        Icon::Eye,
        Icon::Layers,
        Icon::Package,
        Icon::ArrowUp,
        Icon::Search,
        Icon::Trash,
    ];

    /// Asset path served by `crate::assets::Assets`.
    pub const fn path(self) -> &'static str {
        match self {
            Icon::Play => "icons/play.svg",
            Icon::Pause => "icons/pause.svg",
            Icon::SkipBack => "icons/skip-back.svg",
            Icon::SkipForward => "icons/skip-forward.svg",
            Icon::StepBack => "icons/step-back.svg",
            Icon::StepForward => "icons/step-forward.svg",
            Icon::ZoomIn => "icons/zoom-in.svg",
            Icon::ZoomOut => "icons/zoom-out.svg",
            Icon::Maximize => "icons/maximize.svg",
            Icon::Volume2 => "icons/volume-2.svg",
            Icon::VolumeX => "icons/volume-x.svg",
            Icon::Speaker => "icons/speaker.svg",
            Icon::RefreshCw => "icons/refresh-cw.svg",
            Icon::Hammer => "icons/hammer.svg",
            Icon::Square => "icons/square.svg",
            Icon::Folder => "icons/folder.svg",
            Icon::FolderOpen => "icons/folder-open.svg",
            Icon::File => "icons/file.svg",
            Icon::Image => "icons/image.svg",
            Icon::Palette => "icons/palette.svg",
            Icon::Bot => "icons/bot.svg",
            Icon::Download => "icons/download.svg",
            Icon::Upload => "icons/upload.svg",
            Icon::Plus => "icons/plus.svg",
            Icon::X => "icons/x.svg",
            Icon::Check => "icons/check.svg",
            Icon::ChevronDown => "icons/chevron-down.svg",
            Icon::ChevronRight => "icons/chevron-right.svg",
            Icon::MoreHorizontal => "icons/ellipsis.svg",
            Icon::Settings => "icons/settings.svg",
            Icon::AlertTriangle => "icons/triangle-alert.svg",
            Icon::Info => "icons/info.svg",
            Icon::CircleCheck => "icons/circle-check.svg",
            Icon::History => "icons/history.svg",
            Icon::Save => "icons/save.svg",
            Icon::Eye => "icons/eye.svg",
            Icon::Layers => "icons/layers.svg",
            Icon::Package => "icons/package.svg",
            Icon::ArrowUp => "icons/arrow-up.svg",
            Icon::Search => "icons/search.svg",
            Icon::Trash => "icons/trash.svg",
        }
    }
}

/// A square icon of `size` px. gpui's `svg()` keeps only the glyph's alpha and
/// does not inherit `text_color`, so the colour is set on the element itself.
pub fn icon(icon: Icon, size: f32, color: Rgba) -> Svg {
    svg()
        .path(icon.path())
        .size(px(size))
        .flex_none()
        .text_color(color)
}

#[cfg(test)]
mod tests {
    use gpui::AssetSource;

    use super::*;
    use crate::assets::Assets;

    #[test]
    fn every_icon_has_an_asset() {
        for icon in Icon::ALL {
            let bytes = Assets.load(icon.path()).expect("load");
            let bytes = bytes.unwrap_or_else(|| panic!("missing asset {}", icon.path()));
            assert!(bytes.starts_with(b"<!--") || bytes.starts_with(b"<svg"));
        }
    }

    #[test]
    fn icon_paths_are_unique_and_listed() {
        let listed = Assets.list("icons/").expect("list");
        let mut paths: Vec<_> = Icon::ALL.iter().map(|icon| icon.path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), Icon::ALL.len());
        assert_eq!(paths.len(), listed.len());
    }
}
