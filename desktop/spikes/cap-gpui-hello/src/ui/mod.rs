pub mod button;
#[cfg(target_os = "windows")]
pub mod windows_caption;

pub use menu::Menu;

/// Stand-in for Cap's `ui/menu.rs`: the `OpenHandler` alias and `Menu::trigger`
/// (copied verbatim) that `button.rs` uses; the popup menu itself is not ported.
pub mod menu {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::{
        App, Bounds, InteractiveElement, ParentElement, Pixels, StatefulInteractiveElement, Styled,
        Window,
    };

    pub(crate) type OpenHandler = Box<dyn Fn(&Bounds<Pixels>, &mut Window, &mut App) + 'static>;

    pub struct Menu;

    impl Menu {
        pub fn trigger(
            element: gpui::Stateful<gpui::Div>,
            handler: impl Fn(&Bounds<Pixels>, &mut Window, &mut App) + 'static,
        ) -> gpui::Stateful<gpui::Div> {
            let bounds = Rc::new(Cell::new(None));
            let measured_bounds = bounds.clone();
            element
                .tab_index(0)
                .relative()
                .on_click(move |_, window, cx| {
                    if let Some(bounds) = bounds.get() {
                        handler(&bounds, window, cx);
                    }
                })
                .child(
                    gpui::canvas(
                        move |bounds, _, _| measured_bounds.set(Some(bounds)),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .inset_0(),
                )
        }
    }
}
