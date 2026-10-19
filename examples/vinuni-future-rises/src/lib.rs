//! "Where the Future Rises": a 2:36 1080p60 introduction to `VinUniversity` in six chapters
//! (who it is, what you study, how you learn, the world, life on campus, your future), cut to
//! a 120 BPM grid in nineteen scenes after a beat-montage intro. A wipe crosses every chapter
//! cut, and the footage dissolves across the other cuts. Real footage
//! from `vinuni.edu.vn` and the university's official `YouTube` channel, the colleges'
//! lockups from their films and the employers' and universities' logos (all fetched by
//! `tools/fetch-media.py`) fill the screen, graded by `SkSL` shaders, under titles and 3D. The soundtrack is synthesized.

use std::sync::LazyLock;

use fframes::{
    AudioMap, AudioTimestamp, AudioTrack, Color, Duration, FFramesContext, FFramesSyncedVideoFrame,
    Frame, Scenes, Shader, ShaderUniforms, Svgr, SyncVideoFrameInput, Video, include_media_dir,
};

mod geo_dots;
mod scenes;
pub mod three_d;

include_media_dir!(pub struct VinUniFutureRisesMedia, "examples/vinuni-future-rises/media");

/// Output size: 1080p60. Everything is laid out on a 1920x1080 view box.
pub const WIDTH: usize = 1920;
pub const HEIGHT: usize = 1080;
pub const FPS: usize = 60;
/// 120 BPM.
pub const BEAT: f32 = 0.5;

/// The `VinUniversity` brand: blue sampled from the logo, navy, light blue and amber from
/// the vinuni.edu.vn stylesheet (`--home-text-dark`, `--home-blue-light`,
/// `--home-orange`). Amber is the accent; each college scene uses its college's colour
/// (`scenes::college`).
pub(crate) const NAVY: &str = "#002147";
pub(crate) const BLUE: &str = "#1C4078";
pub(crate) const AMBER: &str = "#F9A730";
pub(crate) const SKY: &str = "#E0ECF9";
pub(crate) const MUTED: &str = "#9DB2D6";
/// The site's typefaces: Montserrat for headings (700) and spaced labels (600, `LABEL`),
/// Mulish for body text. Montserrat Bold comes with fframes-components' media.
pub(crate) const DISPLAY: &str = "Montserrat";
pub(crate) const LABEL: u16 = 600;
pub(crate) const SANS: &str = "Mulish";

pub(crate) fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// 0 → 1 between `start` and `start + duration` seconds.
pub(crate) fn progress(t: f32, start: f32, duration: f32) -> f32 {
    clamp01((t - start) / duration)
}

pub(crate) fn ease_out(p: f32) -> f32 {
    1.0 - (1.0 - p).powi(3)
}

pub(crate) fn ease_in_out(p: f32) -> f32 {
    if p < 0.5 {
        4.0 * p * p * p
    } else {
        1.0 - (-2.0 * p + 2.0).powi(3) / 2.0
    }
}

/// A damped spring released at `start` seconds settling from 0 to 1 (with overshoot),
/// the closed form of `Easing::Spring { mass, stiffness, damping }`.
pub(crate) fn spring(t: f32, start: f32, mass: f32, stiffness: f32, damping: f32) -> f32 {
    let t = t - start;
    if t <= 0.0 {
        return 0.0;
    }
    let omega = (stiffness / mass).sqrt();
    let zeta = damping / (2.0 * (stiffness * mass).sqrt());
    if zeta >= 1.0 {
        return 1.0 - (1.0 + omega * t) * (-omega * t).exp();
    }
    let wd = omega * (1.0 - zeta * zeta).sqrt();
    1.0 - (-zeta * omega * t).exp() * ((wd * t).cos() + zeta * omega / wd * (wd * t).sin())
}

/// The punch-in spring: mass 1, stiffness 220, damping 18.
pub(crate) fn punch(t: f32, start: f32) -> f32 {
    spring(t, start, 1.0, 220.0, 18.0)
}

/// Scene content fades in over 0.25 s and out over the last 0.25 s of `length`.
pub(crate) fn scene_fade(t: f32, length: f32) -> f32 {
    progress(t, 0.0, 0.25).min(1.0 - progress(t, length - 0.25, 0.25))
}

pub(crate) static PARTICLES: LazyLock<Shader> =
    LazyLock::new(|| Shader::sksl(include_str!("shaders/particles.sksl")));
pub(crate) static GRADE: LazyLock<Shader> =
    LazyLock::new(|| Shader::sksl(include_str!("shaders/grade.sksl")));
pub(crate) static AURORA: LazyLock<Shader> =
    LazyLock::new(|| Shader::sksl(include_str!("shaders/aurora.sksl")));

/// How a clip is graded by `shaders/grade.sksl`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grade {
    /// 0 keeps the footage's colour, 1 is the full navy-to-light-blue duotone.
    pub amount: f32,
    pub zoom: f32,
    pub scan: f32,
    pub exposure: f32,
}

impl Default for Grade {
    fn default() -> Self {
        Grade {
            amount: 0.15,
            zoom: 1.0,
            scan: 0.0,
            exposure: 1.0,
        }
    }
}

/// A clip of `dynamic_media/` that starts `at` seconds into the scene, graded on the GPU
/// and drawn into the box `(x, y, w, h)` of the 1920x1080 view box. Nothing before it
/// starts, after it ends, or when the clip is missing.
pub(crate) fn footage<'a>(
    frame: &Frame,
    ctx: &FFramesContext<'a, '_>,
    clip: &str,
    at: f32,
    grade: Grade,
    (x, y, w, h): (f32, f32, f32, f32),
) -> Svgr<'a> {
    let input = SyncVideoFrameInput {
        start_from: at,
        looping: false,
        editor_fallback_image: None,
    };
    let Some(video) = frame.get_synced_video_frame(ctx, clip, &input) else {
        return Svgr::empty();
    };
    let image = video.into_image();
    let layer = GRADE.draw(
        frame,
        ShaderUniforms::new()
            .image("uFootage", &image)
            .float2("uSize", video.width() as f32, video.height() as f32)
            .float("uAmount", grade.amount)
            .float("uZoom", grade.zoom)
            .float("uScan", grade.scan)
            .float("uExposure", grade.exposure),
    );
    fframes::svgr!(<image href={layer.href()} x={x} y={y} width={w} height={h} preserveAspectRatio="none" />)
}

/// A site photo slowly pushed in (and drifted by `drift` px) from `at` seconds, clipped
/// to the box `(x, y, w, h)`; `id` names the clip path and must be unique in the frame.
pub(crate) fn photo<'a>(
    ctx: &FFramesContext<'a, '_>,
    name: &str,
    id: &str,
    t: f32,
    at: f32,
    drift: f32,
    (x, y, w, h): (f32, f32, f32, f32),
) -> Svgr<'a> {
    let Some(image) = ctx.get_image(name) else {
        return Svgr::empty();
    };
    let k = (t - at).max(0.0);
    let scale = 1.06 + 0.02 * k;
    let (cx, cy) = (x + w / 2.0 + drift * k / 4.0, y + h / 2.0);
    fframes::svgr!(<g>
        <defs><clipPath id={id.to_owned()}><rect x={x} y={y} width={w} height={h} /></clipPath></defs>
        <g clip-path={format!("url(#{id})")} opacity={progress(t, at, 0.3)}>
            <g transform={format!("translate({cx} {cy}) scale({scale}) translate({} {})", -w / 2.0, -h / 2.0)}>
                <image href={image.href()} x="0" y="0" width={w} height={h} preserveAspectRatio="xMidYMid slice" />
            </g>
        </g>
    </g>)
}

/// One shot of a scene's reel, cut in at a scene second: a clip of `dynamic_media/` or a
/// site photo.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Shot {
    Clip(&'static str, f32),
    Photo(&'static str, f32),
}

impl Shot {
    fn at(self) -> f32 {
        match self {
            Shot::Clip(_, at) | Shot::Photo(_, at) => at,
        }
    }
}

/// Scene `index`'s full-screen shots cut in sequence, each slowly pushing in. Only the
/// current shot is drawn, over the previous one while it cuts in (0.08 s); the first one
/// dissolves in over the previous scene's last shot ([`scenes::carry_in`]).
pub(crate) fn reel<'a>(
    frame: &Frame,
    ctx: &FFramesContext<'a, '_>,
    index: usize,
    shots: &[Shot],
    grade: Grade,
) -> Svgr<'a> {
    let t = frame.seconds();
    let full = (0.0, 0.0, 1920.0, 1080.0);
    let draw = |shot: Shot, id: &str, fade: f32| match shot {
        Shot::Clip(clip, at) => {
            let grade = Grade {
                zoom: grade.zoom + 0.012 * (t - at).max(0.0),
                ..grade
            };
            let layer = footage(frame, ctx, clip, at, grade, full);
            fframes::svgr!(<g opacity={progress(t, at, fade)}>{layer}</g>)
        }
        Shot::Photo(name, at) => {
            // started 0.3 s early so its own fade-in is over when it cuts in
            let layer = photo(ctx, name, id, t, at - 0.3, 30.0, full);
            fframes::svgr!(<g opacity={progress(t, at, fade)}>{layer}</g>)
        }
    };
    let carry = scenes::carry_in(index).filter(|_| t < scenes::DISSOLVE);
    let current = shots.iter().rposition(|s| t >= s.at()).unwrap_or(0);
    let first = if t - shots[current].at() < 0.08 {
        current.saturating_sub(1)
    } else {
        current
    };
    let mut layers: Vec<Svgr<'a>> = carry
        .map(|shot| draw(shot, "reel-carry", 0.0))
        .into_iter()
        .collect();
    layers.extend(shots[first..=current].iter().enumerate().map(|(i, &shot)| {
        let fade = if first + i == 0 && carry.is_some() {
            scenes::DISSOLVE
        } else {
            0.08
        };
        draw(shot, "reel-photo", fade)
    }));
    fframes::svgr!(<g>{layers}</g>)
}

/// The chapters on the video timeline (seconds): who we are, what you study, how you learn,
/// the world, life on campus, your future.
pub(crate) const CHAPTERS: [(f32, f32); 6] = [
    (10.0, 34.0),
    (34.0, 74.0),
    (74.0, 84.0),
    (84.0, 102.0),
    (102.0, 124.0),
    (124.0, 146.0),
];

/// How long the chapter wipe takes to cross the screen; the cut sits at its middle.
const WIPE: f32 = 0.8;

/// At every chapter start a slanted navy panel with an amber edge sweeps across the screen
/// from the left, covering the whole frame at the cut, and leaves on the right.
fn chapter_wipe(t: f32) -> Svgr<'static> {
    const SKEW: f32 = 320.0;
    const BAND: f32 = 2700.0;
    let Some(&(start, _)) = CHAPTERS
        .iter()
        .find(|&&(start, _)| (t - start).abs() < WIPE / 2.0)
    else {
        return Svgr::empty();
    };
    let p = ease_in_out(progress(t, start - WIPE / 2.0, WIPE));
    // the leading edge's bottom corner, from off the left edge to off the right one
    let lead = -SKEW + p * (1920.0 + BAND + 2.0 * SKEW);
    let tail = lead - BAND;
    let panel = format!(
        "M{tail} 1080L{lead} 1080L{} 0L{} 0Z",
        lead + SKEW,
        tail + SKEW
    );
    let edge = format!(
        "M{lead} 1080L{} 1080L{} 0L{} 0Z",
        lead + 26.0,
        lead + SKEW + 26.0,
        lead + SKEW
    );
    let back = format!(
        "M{} 1080L{tail} 1080L{} 0L{} 0Z",
        tail - 12.0,
        tail + SKEW,
        tail + SKEW - 12.0
    );
    fframes::svgr!(<g>
        <path d={panel} fill={NAVY} />
        <path d={edge} fill={AMBER} />
        <path d={back} fill={BLUE} />
    </g>)
}

/// The film.
#[derive(Debug, Default)]
pub struct VinUniFutureRises;

impl Video for VinUniFutureRises {
    const FPS: usize = FPS;
    const WIDTH: usize = WIDTH;
    const HEIGHT: usize = HEIGHT;
    const BACKGROUND_COLOR: Color = Color::BLACK;

    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }

    fn audio(&self) -> AudioMap<'_> {
        use AudioTimestamp::{Eof, Second};
        let at = |file: &'static str, second: f32| AudioTrack::new(file, Second(second)..Eof);
        let mut tracks = vec![at("music.wav", 0.0).fade_out(1.5)];
        tracks.extend(
            scenes::sound_effects()
                .into_iter()
                .map(|(file, second, gain, pan)| at(file, second).gain_db(gain).pan(pan)),
        );
        AudioMap::from(tracks)
    }

    fn define_scenes(&self) -> Scenes<'_> {
        Scenes::from(scenes::all())
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.global_index as f32 / FPS as f32;
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" width={WIDTH} height={HEIGHT} viewBox="0 0 1920 1080">
                <rect width="1920" height="1080" fill={NAVY} />
                {ctx.render_scenes(&frame)}
                {chapter_wipe(t)}
            </svg>
        )
    }
}

#[cfg(test)]
mod tests {
    use fframes_skia_renderer::render::compile_shader;

    use super::{AURORA, CHAPTERS, GRADE, PARTICLES};
    use crate::scenes::START;

    #[test]
    fn shaders_compile() {
        for shader in [&*PARTICLES, &*GRADE, &*AURORA] {
            compile_shader(shader).unwrap();
        }
    }

    #[test]
    fn chapters_start_and_end_on_cuts() {
        for (start, end) in CHAPTERS {
            assert!(
                START.contains(&start),
                "chapter at {start} s is not on a cut"
            );
            assert!(START.contains(&end), "chapter end {end} s is not on a cut");
        }
    }
}
