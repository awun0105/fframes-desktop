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

    for y in 0..height {
        let in_row_start = y * in_stride;
        let out_row_start = y * out_stride;

        for x in 0..width {
            let in_px = in_row_start + x * 4;
            let out_px = out_row_start + x * 4;

            let mut c0 = payload[in_px];
            let mut c1 = payload[in_px + 1];
            let mut c2 = payload[in_px + 2];
            let a = payload[in_px + 3];

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

            bgra_bytes[out_px] = c0;
            bgra_bytes[out_px + 1] = c1;
            bgra_bytes[out_px + 2] = c2;
            bgra_bytes[out_px + 3] = a;
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

pub use crate::worker_client::generate_reference_frame;

/// Tracks the presentation state and ensures the previous image is dropped from GPUI cache.
pub struct ImagePresentationManager {
    current_image: Option<Arc<RenderImage>>,
    resident_count: usize,
    queue_depth: usize,
    dropped_image_count: usize,
    release_failures: usize,
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

    /// Replaces the current image with the new image, invoking `window.drop_image()`
    /// on the prior image to evict it from GPUI's sprite atlas and prevent memory leaks.
    pub fn replace_image(&mut self, new_image: Arc<RenderImage>, window: &mut Window) {
        self.clear(window);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
