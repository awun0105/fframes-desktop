//! The introduction of the desktop studio of fframes, made with fframes.
//!
//! The 16:9 cut tells the storyboard of `plan.md` in seven acts (`scenes/keynote`):
//! the problem, the app, the workflow, use cases, how it works, why it is fast
//! and where to get it, with real renders of other examples and real output of
//! the CLI tools. The 9:16 and short cuts keep their own story. Everything sits
//! on the beat grid of the soundtrack of `examples/fframes-intro` (see `beat.rs`).
//! The 16:9 cut is drawn in the Editorial Engineering system of
//! `docs/design/README.md` (`scenes/keynote/look.rs`).
//! `cargo run --release -p fframes-studio-intro -- timeline` lists the scenes.

pub mod beat;
pub mod facts;
#[cfg(not(target_arch = "wasm32"))]
pub mod render_cli;
pub mod scenes;
pub mod shaders;
pub mod short;
pub mod ui;
pub mod wall;

use fframes::{
    AudioMap, AudioTimestamp::*, AudioTrack, Color, Duration, FFramesContext, Frame, Scene, Scenes,
    ShaderUniforms, Svgr, Video, include_media_dir,
};

use beat::*;
use scenes::*;
use shaders::SHADERS;
use ui::*;

include_media_dir!(pub struct StudioIntroMedia, "examples/fframes-studio-intro/media");

pub const WIDTH: usize = 1920;
pub const HEIGHT: usize = 1080;

#[derive(Debug)]
pub struct StudioIntroVideo;

/// Section titles for the HUD, by the beat they start on.
const SECTIONS: &[(f32, &str)] = &[
    (-99.0, "WELCOME"),
    (0.0, "INSTALL"),
    (16.0, "WORKSPACE"),
    (22.0, "POINT"),
    (30.0, "ASK"),
    (48.0, "FIND"),
    (60.0, "EDIT"),
    (68.0, "WATCH"),
    (76.0, "EXPORT"),
    (80.0, "YOUR PROJECT"),
    (96.0, "FFRAMES STUDIO"),
    (112.0, "HOW IT WORKS"),
    (144.0, "SPEED TEST"),
    (176.0, "WHY IT'S FAST"),
    (208.0, "GET IT"),
];

/// The acts of the 16:9 cut, by the beat they start on.
const ACTS: &[(f32, &str)] = &[
    (-99.0, "01 THE PROBLEM"),
    (48.0, "02 THE SOLUTION"),
    (80.0, "03 THE WORKFLOW"),
    (128.0, "04 USE CASES"),
    (176.0, "05 HOW IT WORKS"),
    (208.0, "06 WHY IT'S FAST"),
    (240.0, "07 GET IT"),
];

/// Sound effects, placed on the beat minus the file's attack.
fn sfx(name: &'static str, at: f32, gain: f32) -> AudioTrack<'static> {
    AudioTrack::new(name, Second(at.max(0.0))..Eof).gain_db(gain)
}

impl Video for StudioIntroVideo {
    const FPS: usize = FPS;
    const WIDTH: usize = WIDTH;
    const HEIGHT: usize = HEIGHT;
    const BACKGROUND_COLOR: Color = Color::WHITE;

    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }

    fn audio(&self) -> AudioMap<'_> {
        audio_map_long()
    }

    fn define_scenes(&self) -> Scenes<'_> {
        use scenes::keynote::*;
        Scenes::from(vec![
            &ProblemScene as &dyn Scene,
            &SolutionScene,
            &WorkflowScene,
            &UseCasesScene,
            &LayersScene,
            &SpeedScene,
            &FinaleScene,
        ])
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        // a quiet paper canvas: no grain, vignette, flashes or glitches, the
        // motion of the figures carries the beat
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080">
                <rect width="1920" height="1080" fill={keynote::look::BG} />
                {ctx.render_scenes(&frame)}
                {hud_long(&frame, gbeat(&frame))}
            </svg>
        )
    }
}

/// The soundtrack and sound effects of the 16:9 cut. The music plays one
/// 32-beat phrase twice: from video beat 144 it goes back to beat 112 of the
/// file, and from video beat 176 it carries on from beat 144 of the file, so
/// both seams sit on downbeats of the full beat.
fn audio_map_long() -> AudioMap<'static> {
    let repeat = LONG_REPEAT_AT;
    let back = repeat - LONG_EXTRA_BEATS;
    let resume = repeat + LONG_EXTRA_BEATS;
    let mut tracks = vec![
        AudioTrack::new("music.wav", Second(0.0)..Second(beat_time(repeat))),
        AudioTrack::new(
            "music.wav",
            Second(beat_time(repeat))..Second(beat_time(resume)),
        )
        .offset(beat_time(back)),
        AudioTrack::new(
            "music.wav",
            Second(beat_time(resume))..Second(LONG_TOTAL_SECONDS),
        )
        .offset(beat_time(repeat))
        .fade_out(1.8),
    ];
    tracks.extend(
        keynote::sound_effects()
            .into_iter()
            .map(|(name, at, gain)| sfx(name, at, gain)),
    );
    AudioMap::from(tracks)
}

/// The soundtrack and sound effects of the 9:16 cut.
fn audio_map() -> AudioMap<'static> {
    {
        let mut tracks = vec![
            AudioTrack::new("music.wav", Second(0.0)..Second(TOTAL_SECONDS)).fade_out(1.8),
            sfx("sfx_typing.mp3", 0.02, -15.0),
            sfx("sfx_enter.mp3", beat_time(-1.0) - 0.067, -9.0),
            sfx("sfx_glitch.mp3", beat_time(16.0) - 0.2, -8.0),
            // the prompt is typed over the second half of the select scene, then sent
            sfx("sfx_typing.mp3", beat_time(studio::TYPE_START), -17.0),
            sfx("sfx_typing.mp3", beat_time(studio::TYPE_START + 3.5), -17.0),
            sfx("sfx_enter.mp3", beat_time(studio::SEND_AT) - 0.067, -9.0),
            sfx("sfx_impact.mp3", beat_time(48.0), -10.0),
            sfx("sfx_shutter.mp3", beat_time(studio::RESULT_AT) - 0.02, -4.0),
            sfx("sfx_impact.mp3", beat_time(96.0), -10.0),
            sfx("sfx_braam.mp3", beat_time(96.0), -9.0),
            sfx("sfx_impact.mp3", beat_time(144.0), -12.0),
            sfx("sfx_impact.mp3", beat_time(208.0), -11.0),
            sfx("sfx_braam.mp3", beat_time(208.0), -9.0),
        ];
        for b in [16.0, 80.0, 96.0, 112.0, 144.0, 176.0, 208.0] {
            tracks.push(sfx("sfx_whoosh.mp3", beat_time(b) - 0.18, -15.0));
        }
        for b in install::BLIP_BEATS
            .iter()
            .chain(studio::BLIP_BEATS)
            .chain(project::BLIP_BEATS)
            .chain(story::BLIP_BEATS)
        {
            tracks.push(sfx("sfx_blip.mp3", beat_time(*b), -9.0));
        }
        AudioMap::from(tracks)
    }
}

/// Sections where the full beat plays: the picture pumps with the kick.
const PUMPING: &[(f32, f32)] = &[(48.0, 80.0), (96.0, 160.0), (208.0, 232.0)];
/// Cuts that glitch for a few frames.
const GLITCH_CUTS: &[f32] = &[16.0, 80.0, 96.0, 112.0, 144.0, 176.0, 208.0];
/// Drops that flash.
const DROPS: &[f32] = &[48.0, 96.0, 144.0, 208.0];

/// The HUD of the 16:9 cut: crop marks, the product, the act and a hairline of
/// progress.
fn hud_long(frame: &Frame, b: f32) -> Svgr<'static> {
    use keynote::look::{ACTION, CONTROL, RULE, label};
    let appear = prog(gsec(frame), 0.1, 0.5) * (1.0 - prog(b, LONG_END - 1.0, LONG_END - 0.5));
    if appear <= 0.0 {
        return Svgr::empty();
    }
    let act = ACTS
        .iter()
        .rev()
        .find(|(start, _)| b >= *start)
        .map_or("", |(_, name)| *name);
    let progress = (gsec(frame) / LONG_TOTAL_SECONDS * 1788.0).max(0.5);
    fframes::svgr!(
        <g opacity={appear}>
            {corners(36.0, 36.0, 1848.0, 1008.0, 26.0, CONTROL, 1.5)}
            {label(66.0, 80.0, "FFRAMES STUDIO", "start")}
            {label(1854.0, 1016.0, act, "end")}
            <rect x="66" y="1034" width="1788" height="1.5" fill={RULE} />
            <rect x="66" y="1033" width={progress} height="3" fill={ACTION} />
        </g>
    )
}

// ---------------------------------------------------------------------------
// the 9:16 cut

/// The same video for phones: 1080x1920, same beats, sound and length.
#[derive(Debug)]
pub struct StudioIntroPortraitVideo;

impl Video for StudioIntroPortraitVideo {
    const FPS: usize = FPS;
    const WIDTH: usize = scenes::portrait::WIDTH;
    const HEIGHT: usize = scenes::portrait::HEIGHT;
    const BACKGROUND_COLOR: Color = Color::BLACK;

    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }

    fn audio(&self) -> AudioMap<'_> {
        audio_map()
    }

    fn define_scenes(&self) -> Scenes<'_> {
        use scenes::portrait::*;
        Scenes::from(vec![
            &HookScene as &dyn Scene,
            &InstallScene,
            &StudioScene,
            &ProjectScene,
            &OutroScene,
            &HowScene,
            &FastScene,
            &WhyScene,
            &EndScene,
        ])
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let b = gbeat(&frame);
        let grain = SHADERS.grain.draw(
            &frame,
            ShaderUniforms::new()
                .float("uGrain", 0.07)
                .float("uVignette", 0.55),
        );
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1920" viewBox="0 0 1080 1920">
                <rect width="1080" height="1920" fill={BG} />
                {energy_portrait(&frame, ctx, b)}
                <image href={grain.href()} x="0" y="0" width="1080" height="1920" />
                {hud_portrait(&frame, b)}
            </svg>
        )
    }
}

fn hud_portrait(frame: &Frame, b: f32) -> Svgr<'static> {
    let appear = prog(gsec(frame), 0.1, 0.5);
    if appear <= 0.0 {
        return Svgr::empty();
    }
    let section = SECTIONS
        .iter()
        .rev()
        .find(|(start, _)| b >= *start)
        .map_or("", |(_, name)| *name);
    // only what a viewer can use: the chapter and how far along the video is
    let progress = (gsec(frame) / TOTAL_SECONDS * 948.0).max(0.5);
    fframes::svgr!(
        <g opacity={appear * 0.9} style="mix-blend-mode:difference">
            {corners(36.0, 36.0, 1008.0, 1848.0, 26.0, "#d8d4cc", 2.0)}
            {label(66.0, 82.0, "FFRAMES STUDIO".to_owned(), "#d8d4cc", 20.0, "start")}
            {label(1014.0, 1866.0, section.to_owned(), "#d8d4cc", 20.0, "end")}
            <rect x="66" y="1882" width="948" height="1" fill="#d8d4cc" opacity="0.25" />
            <rect x="66" y="1881" width={progress} height="3" fill="#d8d4cc" />
        </g>
    )
}

fn energy_portrait<'a>(frame: &Frame, ctx: &FFramesContext<'a, '_>, b: f32) -> Svgr<'a> {
    let pumping = PUMPING.iter().any(|(s, e)| (*s..*e).contains(&b));
    let pump = if pumping { pulse(b, 9.0) } else { 0.0 };
    let s = 1.0 + pump * 0.006;
    let flash = DROPS
        .iter()
        .map(|d| if b >= *d { (-(b - d) * 7.0).exp() } else { 0.0 })
        .fold(0.0_f32, f32::max);
    let since_cut = GLITCH_CUTS
        .iter()
        .map(|c| (b - c) * BEAT * FPS as f32)
        .find(|f| *f >= 0.0 && *f < 3.0);
    let scene = if let Some(f) = since_cut {
        let bands: Vec<Svgr> = (0..6)
            .map(|i| {
                let y = i as f32 * 320.0;
                let dx = (hash(i as f32 * 13.0 + f.floor() * 7.0) - 0.5) * 100.0 * (1.0 - f / 3.0);
                let id = format!("glitch-p{i}");
                let url = format!("url(#{id})");
                fframes::svgr!(
                    <g>
                        <clipPath id={id.clone()}><rect x="0" y={y} width="1080" height="320" /></clipPath>
                        <g clip-path={url} transform={format!("translate({dx} 0)")}>{ctx.render_scenes(frame)}</g>
                    </g>
                )
            })
            .collect();
        fframes::svgr!(<g>{bands}</g>)
    } else {
        ctx.render_scenes(frame)
    };
    fframes::svgr!(
        <g>
            <g transform={format!("translate(540 960) scale({s}) translate(-540 -960)")}>{scene}</g>
            <rect width="1080" height="1920" fill={BONE} opacity={flash * 0.55} />
        </g>
    )
}
