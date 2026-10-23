#![cfg(target_arch = "wasm32")]
use components_gallery::{GalleryMedia, GalleryVideo};
use fframes_editor_controller::prelude::*;

impl_wasm_bridge_for!(GalleryVideo<'static>, GalleryMedia);

lazy_static! {
    static ref MEDIA: GalleryMedia =
        GalleryMedia::prepare().expect("Failed static media processing for wasm bridge");
}

#[wasm_bindgen]
pub fn create_wasm_bridge() -> WasmBridge {
    console_error_panic_hook::set_once();

    WasmBridge::new(GalleryVideo::new(&MEDIA), &MEDIA)
}
