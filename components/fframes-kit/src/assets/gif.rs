//! Animated GIFs (Remotion's `<Gif>`, feature `gif`): [`Gif::decode`] once, [`Gif`] per
//! frame.

use fframes::{Color, Svgr};

use crate::{
    motion::progress,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// The error of [`Gif::decode`].
pub use image::ImageError as GifError;

/// The seconds a frame with a delay this short is shown, as browsers do.
const SHORT_DELAY: f32 = 0.1;

/// The frames of a GIF, decoded once: the composited pictures as images, and how long each
/// is shown. In the editor (wasm32) only the size and the delays are kept.
#[derive(Debug, Default)]
pub struct GifData {
    width: u32,
    height: u32,
    delays: Vec<f32>,
    #[cfg(not(target_arch = "wasm32"))]
    frames: Vec<fframes::media::ImageData<'static>>,
}

/// What a [`Gif`] draws before it is given one.
static NO_GIF: GifData = GifData {
    width: 0,
    height: 0,
    delays: Vec::new(),
    #[cfg(not(target_arch = "wasm32"))]
    frames: Vec::new(),
};

impl GifData {
    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// How long each frame is shown, in seconds.
    pub fn delays(&self) -> &[f32] {
        &self.delays
    }

    /// The length of one play, in seconds.
    pub fn seconds(&self) -> f32 {
        self.delays.iter().sum()
    }
}

/// The frame shown `t` seconds into the GIF with these delays: the first before 0, the
/// last after the end unless `looping`.
pub fn frame_index(delays: &[f32], t: f32, looping: bool) -> usize {
    let total: f32 = delays.iter().sum();
    if delays.is_empty() || t <= 0.0 || total <= 0.0 {
        return 0;
    }
    let t = if looping { t.rem_euclid(total) } else { t };
    let mut end = 0.0;
    for (i, delay) in delays.iter().enumerate() {
        end += delay;
        if t < end {
            return i;
        }
    }
    delays.len() - 1
}

/// How a [`Gif`] fills its box (Remotion's `fit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum GifFit {
    /// Stretched to the box.
    #[default]
    Fill,
    /// As large as fits, centred.
    Contain,
    /// Covers the box, centred; the rest is cut off.
    Cover,
}

impl GifFit {
    fn aspect(self) -> &'static str {
        match self {
            GifFit::Fill => "none",
            GifFit::Contain => "xMidYMid meet",
            GifFit::Cover => "xMidYMid slice",
        }
    }
}

/// An animated GIF in the box `(x, y, width, height)`, playing from `start` seconds of its
/// container at `speed`. Each frame shows for its own delay (a delay under 20 ms is 100 ms, as
/// in browsers); with `looping` it starts again, otherwise the last frame stays. It fades in
/// over `fade` seconds. Decode the file once with [`Gif::decode`] and keep the [`GifData`]
/// in the video. The editor (wasm32) draws a `placeholder` rectangle of the same size.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Gif<'a> {
    pub gif: &'a GifData,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fit: GifFit,
    pub start: f32,
    pub speed: f32,
    pub looping: bool,
    pub fade: f32,
    pub placeholder: Color,
}

impl Themed for Gif<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            gif: &NO_GIF,
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 320.0,
            fit: GifFit::Fill,
            start: 0.0,
            speed: 1.0,
            looping: true,
            fade: t.motion.duration.fast,
            placeholder: t.colors.surface,
        }
    }
}

impl Default for Gif<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Gif<'_> {
    /// Decodes every frame of a GIF file, composited onto the frames before it, and turns
    /// each into an image once. Call it in the video's constructor, not per frame.
    pub fn decode(bytes: &[u8]) -> Result<GifData, GifError> {
        use image::{AnimationDecoder, codecs::gif::GifDecoder};

        let decoder = GifDecoder::new(std::io::Cursor::new(bytes))?;
        let frames = decoder.into_frames().collect_frames()?;
        let mut gif = GifData::default();
        for frame in frames {
            let (numer, denom) = frame.delay().numer_denom_ms();
            let ms = numer as f32 / denom.max(1) as f32;
            gif.delays
                .push(if ms < 20.0 { SHORT_DELAY } else { ms / 1000.0 });
            let buffer = frame.into_buffer();
            gif.width = buffer.width();
            gif.height = buffer.height();
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut rgba = buffer.into_raw();
                premultiply(&mut rgba);
                gif.frames
                    .push(fframes::media::rendered_image(gif.width, gif.height, rgba));
            }
        }
        Ok(gif)
    }

    /// The frame shown at `t` seconds of the container.
    pub fn frame_at(&self, t: f32) -> usize {
        frame_index(
            &self.gif.delays,
            (t - self.start) * self.speed,
            self.looping,
        )
    }
}

/// Straight to premultiplied alpha, which `rendered_image` takes.
#[cfg(not(target_arch = "wasm32"))]
fn premultiply(rgba: &mut [u8]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u16::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
}

impl<'a> Component<'a> for Gif<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let opacity = progress(t, self.start, self.start + self.fade);
        if opacity <= 0.0 {
            return Svgr::empty();
        }
        let (x, y, w, h) = (self.x, self.y, self.width, self.height);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(frame) = self.gif.frames.get(self.frame_at(t)) else {
                return Svgr::empty();
            };
            let aspect = self.fit.aspect();
            fframes::svgr!(
                <image href={frame.href()} x={x} y={y} width={w} height={h}
                       preserveAspectRatio={aspect} opacity={opacity} />
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self.fit.aspect();
            fframes::svgr!(
                <rect x={x} y={y} width={w} height={h} fill={self.placeholder} opacity={opacity} />
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DELAYS: [f32; 3] = [0.1, 0.2, 0.3];

    #[test]
    fn picks_the_frame_from_the_delays() {
        assert_eq!(frame_index(&DELAYS, -1.0, false), 0);
        assert_eq!(frame_index(&DELAYS, 0.05, false), 0);
        assert_eq!(frame_index(&DELAYS, 0.15, false), 1);
        assert_eq!(frame_index(&DELAYS, 0.35, false), 2);
    }

    #[test]
    fn loops_or_holds_the_last_frame() {
        assert_eq!(frame_index(&DELAYS, 0.65, true), 0);
        assert_eq!(frame_index(&DELAYS, 0.85, true), 1);
        assert_eq!(frame_index(&DELAYS, 0.65, false), 2);
        assert_eq!(frame_index(&DELAYS, 99.0, false), 2);
        assert_eq!(frame_index(&[], 1.0, true), 0);
    }

    #[test]
    fn start_and_speed_shift_the_time() {
        let gif = GifData {
            delays: DELAYS.to_vec(),
            ..GifData::default()
        };
        let player = Gif {
            gif: &gif,
            start: 1.0,
            speed: 2.0,
            looping: false,
            ..Gif::default()
        };
        assert_eq!(player.frame_at(0.5), 0);
        // 0.1 s after the start at double speed is 0.2 s into the GIF
        assert_eq!(player.frame_at(1.1), 1);
        assert_eq!(player.frame_at(5.0), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn decodes_frames_and_delays() {
        use image::{
            Delay, Frame, RgbaImage,
            codecs::gif::{GifEncoder, Repeat},
        };

        let mut bytes = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            for (i, ms) in [(0u8, 50u32), (1, 0)] {
                let image = RgbaImage::from_pixel(4, 2, image::Rgba([255 * i, 0, 0, 255]));
                let frame = Frame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(ms, 1));
                encoder.encode_frame(frame).unwrap();
            }
        }
        let gif = Gif::decode(&bytes).unwrap();
        assert_eq!((gif.width(), gif.height()), (4, 2));
        assert_eq!(gif.delays(), &[0.05, SHORT_DELAY]);
        assert_eq!(gif.frames.len(), 2);
        assert!(Gif::decode(b"not a gif").is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn premultiplies_alpha() {
        let mut px = [255, 128, 0, 128];
        premultiply(&mut px);
        assert_eq!(px, [128, 64, 0, 128]);
    }
}
