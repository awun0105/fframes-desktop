use cap_gpui_hello::{HelloView, SpikeAssets};
use gpui::{AppContext, Bounds, WindowBounds, WindowOptions, px, size};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    gpui_platform::application()
        .with_assets(SpikeAssets)
        .run(|cx| {
            cap_gpui_hello::init(cx);
            // As desktop/app/src/main.rs: fframes' tokens follow the OS appearance.
            ff_ui::design_system::set_appearance(ff_ui::design_system::appearance_from_window(
                cx.window_appearance(),
            ));
            let bounds = Bounds::centered(None, size(px(640.), px(420.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // Cap's main-window frame on Windows (src/main.rs).
                    #[cfg(target_os = "windows")]
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("cap-gpui-hello".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    app_owns_titlebar_drag: true,
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| HelloView::new(window, cx)),
            )
            .expect("open window");
            cx.activate(true);
        });
}
