use fframes_studio_protocol::{AlphaMode, ChannelOrder, FrameHeader, ProtocolError};
use gpui::{RenderImage, Window};
use image::{Frame, RgbaImage};
use smallvec::SmallVec;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("payload buffer too short: expected {expected} bytes, got {actual} bytes")]
    PayloadTooShort { expected: usize, actual: usize },
    #[error("failed to construct image buffer from raw bytes")]
    BufferConstructionFailed,
}

/// Fit validated video dimensions to the physical preview extent without
/// upscaling or exceeding the negotiated preview cap.
pub fn preview_scale(width: usize, height: usize, extent: (u32, u32)) -> f64 {
    use fframes_studio_protocol::{MAX_PREVIEW_HEIGHT, MAX_PREVIEW_WIDTH};
    (f64::from(extent.0.min(MAX_PREVIEW_WIDTH)) / width as f64)
        .min(f64::from(extent.1.min(MAX_PREVIEW_HEIGHT)) / height as f64)
        .min(1.)
}

/// Converts a protocol frame payload into raw BGRA8 pixels with row padding removed,
/// ready for GPUI `RenderImage`.
pub fn convert_rgba_to_gpui_bgra(
    header: &FrameHeader,
    payload: &[u8],
) -> Result<Vec<u8>, ConversionError> {
    header.validate()?;

    if payload.len() < header.payload_len {
        return Err(ConversionError::PayloadTooShort {
            expected: header.payload_len,
            actual: payload.len(),
        });
    }

    let width = header.width as usize;
    let height = header.height as usize;
    let in_stride = header.stride_bytes as usize;
    let out_stride = width * 4;
    let mut bgra_bytes = vec![0u8; width * height * 4];

    let swap_channels = header.channel_order == ChannelOrder::Rgba8;
    let un_premultiply = header.alpha_mode == AlphaMode::Premultiplied;

    // Row by row with chunked pixels: no index arithmetic or bounds checks per pixel.
    for (in_row, out_row) in payload
        .chunks(in_stride)
        .zip(bgra_bytes.chunks_exact_mut(out_stride))
        .take(height)
    {
        let (src_pixels, _) = in_row[..out_stride].as_chunks::<4>();
        let (dst_pixels, _) = out_row.as_chunks_mut::<4>();
        for (src, dst) in src_pixels.iter().zip(dst_pixels) {
            let (mut c0, mut c1, mut c2, a) = (src[0], src[1], src[2], src[3]);
            if swap_channels {
                // RGBA -> BGRA: swap c0 (R) and c2 (B)
                std::mem::swap(&mut c0, &mut c2);
            }
            if un_premultiply && a > 0 && a < 255 {
                let alpha_scale = a as f32 / 255.0;
                c0 = ((c0 as f32 / alpha_scale).min(255.0)) as u8;
                c1 = ((c1 as f32 / alpha_scale).min(255.0)) as u8;
                c2 = ((c2 as f32 / alpha_scale).min(255.0)) as u8;
            }
            *dst = [c0, c1, c2, a];
        }
    }

    Ok(bgra_bytes)
}

/// Creates a GPUI `RenderImage` from frame header and raw byte payload.
pub fn create_render_image(
    header: &FrameHeader,
    payload: &[u8],
) -> Result<Arc<RenderImage>, ConversionError> {
    let bgra_bytes = convert_rgba_to_gpui_bgra(header, payload)?;
    let buffer = RgbaImage::from_raw(header.width, header.height, bgra_bytes)
        .ok_or(ConversionError::BufferConstructionFailed)?;
    let frame = Frame::new(buffer);
    let render_image = RenderImage::new(SmallVec::from_const([frame]));
    Ok(Arc::new(render_image))
}

/// Tracks the presentation state and ensures the previous image is dropped from GPUI cache.
pub struct ImagePresentationManager {
    current_image: Option<Arc<RenderImage>>,
    resident_count: usize,
    queue_depth: usize,
    dropped_image_count: usize,
    release_failures: usize,
    last_serial: Option<u64>,
    out_of_order: usize,
}

impl Default for ImagePresentationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ImagePresentationManager {
    pub fn new() -> Self {
        Self {
            current_image: None,
            resident_count: 0,
            queue_depth: 0,
            dropped_image_count: 0,
            release_failures: 0,
            last_serial: None,
            out_of_order: 0,
        }
    }

    pub fn current_image(&self) -> Option<Arc<RenderImage>> {
        self.current_image.clone()
    }

    pub fn resident_count(&self) -> usize {
        self.resident_count
    }

    pub fn queue_depth(&self) -> usize {
        self.queue_depth
    }

    pub fn dropped_count(&self) -> usize {
        self.dropped_image_count
    }

    pub fn release_failures(&self) -> usize {
        self.release_failures
    }

    /// Frames shown after a frame of a newer seek. The pump drops them before they get here,
    /// so this stays zero; it is the proof that it does.
    pub fn out_of_order(&self) -> usize {
        self.out_of_order
    }

    /// Shows the frame answering seek `serial`. A serial older than the one on screen is
    /// counted. The preview state never restarts its serials, so installs and retimes are
    /// not counted; [`Self::replace_image`] and [`Self::clear`] forget the last one.
    pub fn present(&mut self, new_image: Arc<RenderImage>, serial: u64, window: &mut Window) {
        if self.last_serial.is_some_and(|last| serial < last) {
            self.out_of_order += 1;
        }
        self.replace_image(new_image, window);
        self.last_serial = Some(serial);
    }

    /// Replaces the current image with the new image, invoking `window.drop_image()`
    /// on the prior image to evict it from GPUI's sprite atlas and prevent memory leaks.
    pub fn replace_image(&mut self, new_image: Arc<RenderImage>, window: &mut Window) {
        self.clear(window);
        self.last_serial = None;
        self.current_image = Some(new_image);
        self.resident_count = 1;
        self.queue_depth = 1;
    }

    /// Evict the actual GPUI texture on close/detach, not only its Rust reference.
    pub fn clear(&mut self, window: &mut Window) {
        if let Some(prior) = self.current_image.take() {
            if window.drop_image(prior).is_err() {
                self.release_failures += 1;
            }
            self.dropped_image_count += 1;
            if self.resident_count > 0 {
                self.resident_count -= 1;
            }
        }
        self.queue_depth = 0;
        self.last_serial = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_engine::services::worker_client::generate_reference_frame;

    #[test]
    fn preview_fits_physical_extent_aspect_and_protocol_cap_without_upscaling() {
        assert_eq!(preview_scale(1280, 720, (718, 134)), 134. / 720.);
        assert_eq!(preview_scale(1080, 1920, (718, 134)), 134. / 1920.);
        assert_eq!(preview_scale(1280, 720, (1436, 268)), 268. / 720.);
        assert_eq!(preview_scale(1920, 1080, (2560, 1440)), 2. / 3.);
        assert_eq!(preview_scale(320, 180, (718, 134)), 134. / 180.);
        assert_eq!(preview_scale(320, 180, (718, 400)), 1.);
    }

    #[test]
    fn test_convert_rgba_to_bgra_without_stride_padding() {
        // 2x2 image:
        // Top-left: [255, 0, 0, 255] (Red) -> BGRA [0, 0, 255, 255]
        // Top-right: [0, 255, 0, 255] (Green) -> BGRA [0, 255, 0, 255]
        // Bottom-left: [0, 0, 255, 128] (Blue half-alpha) -> BGRA [255, 0, 0, 128]
        // Bottom-right: [0, 0, 0, 0] (Transparent) -> BGRA [0, 0, 0, 0]
        let (header, payload) = generate_reference_frame("rev1", 1, 1, 0, 2, 2, false);
        let bgra = convert_rgba_to_gpui_bgra(&header, &payload).expect("conversion succeeds");

        assert_eq!(bgra.len(), 2 * 2 * 4);
        // Note: x=0, y=0 had the indicator pixel overridden because 0 % 2 == 0
        // Let's check without indicator or check x=1, y=0 (Green)
        let green_px = 4;
        assert_eq!(&bgra[green_px..green_px + 4], &[0, 255, 0, 255]); // Green in BGRA is B=0, G=255, R=0, A=255

        // Bottom-left x=0, y=1: Blue half-alpha
        let blue_px = 2 * 4;
        assert_eq!(&bgra[blue_px..blue_px + 4], &[255, 0, 0, 128]); // Blue in BGRA is B=255, G=0, R=0, A=128

        // Bottom-right x=1, y=1: Transparent
        let trans_px = (2 + 1) * 4;
        assert_eq!(&bgra[trans_px..trans_px + 4], &[0, 0, 0, 0]);
    }

    struct Blank;
    impl gpui::Render for Blank {
        fn render(
            &mut self,
            _: &mut Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn an_older_serial_after_a_newer_one_is_counted_and_the_count_survives_a_clear(
        cx: &mut gpui::TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(|_, _| Blank);
        let image = || {
            let (header, payload) = generate_reference_frame("rev1", 1, 1, 0, 2, 2, false);
            create_render_image(&header, &payload).unwrap()
        };
        let mut images = ImagePresentationManager::new();
        cx.update(|window, _| {
            images.present(image(), 3, window);
            images.present(image(), 5, window);
            assert_eq!(images.out_of_order(), 0);
            images.present(image(), 4, window);
            assert_eq!(images.out_of_order(), 1);
            // A cleared presentation starts its serials again; the count stays.
            images.clear(window);
            images.present(image(), 1, window);
            assert_eq!(images.out_of_order(), 1);
        });
    }

    #[test]
    fn test_convert_rgba_with_stride_padding_stripping() {
        let (header, payload) = generate_reference_frame("rev1", 1, 1, 0, 4, 4, true);
        assert!(header.stride_bytes > header.width * 4);

        let bgra = convert_rgba_to_gpui_bgra(&header, &payload).expect("strips stride padding");
        assert_eq!(bgra.len(), (header.width * header.height * 4) as usize);
    }

    #[test]
    fn test_payload_too_short_rejected() {
        let (header, mut payload) = generate_reference_frame("rev1", 1, 1, 0, 4, 4, false);
        payload.pop(); // Remove 1 byte

        let err = convert_rgba_to_gpui_bgra(&header, &payload).unwrap_err();
        match err {
            ConversionError::PayloadTooShort { expected, actual } => {
                assert_eq!(expected, header.payload_len);
                assert_eq!(actual, header.payload_len - 1);
            }
            other => panic!("expected PayloadTooShort, got {:?}", other),
        }
    }
}
