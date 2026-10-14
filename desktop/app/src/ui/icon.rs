//! Lucide icons; the enum lives in the kit.

pub use studio_ui::kit::{Icon, icon};

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
