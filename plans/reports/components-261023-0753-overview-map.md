# fframes components: overview map

Survey of every example, both scaffolding templates, the core crate and the Studio presets for
code that another video could reuse. Each row names where the code lives today. Paths are
relative to the repository root.

## Summary

- About 60 candidates across 22 examples and 2 templates. About 25 are worth a shared crate,
  the rest are story-specific art.
- Most duplicated code is motion math (`progress`, `clamp01`, `lerp`, easing curves, closed-form
  spring) in 6 examples, 28 inline `Easing::Spring { .. }` literals, the spectrum bar block
  (8 `visualize_audio_frame` calls in 6 examples, 3 copies in `podcast.rs` alone), stroke
  draw-on (6 uses), and blur/glow filter defs (11 files).
- `fframes-intro` and `fframes-studio-intro` share `beat.rs` and `ui.rs` almost verbatim (only
  timing constants and one palette constant differ). They are the best first migration target.
- All three Studio presets (`editorial`, `pulse`, `quiet-motion`) define the same 40 tokens.
  Components can take their default colors, type, spacing and motion from them via
  `fframes::Styles`.

## Map

```text
fframes-components
├── 0. core (already in fframes; wrap, do not rebuild)
├── 1. motion      pure math, no Svgr
├── 2. text        measured text and text effects
├── 3. overlays    lower thirds, captions, chapters, callouts, progress
├── 4. data        counters, bars, races
├── 5. audio       spectrum bars, waveform
├── 6. decoration  brackets, draw-on, frames, glow
├── 7. backgrounds blobs, bokeh, pixel wall, shader backgrounds (skia)
├── 8. transitions cross-fade, push, wipes
├── 9. media       Ken Burns photo, video panels, graded footage, lottie box
├── 10. mockups    terminal, modal, cursor, phone
├── 11. theme      Styles tokens to props
└── 12. 3d         camera shake, beam, sparks, grid (feature "3d")
```

### 0. Core building blocks that already exist

| Building block | API | Components that use it |
| --- | --- | --- |
| Keyframes and easing | `timeline!`, `frame.animate`, `Easing::*` | everything animated |
| Text measuring | `frame.text_width`, `text_fit`, `text_break_lines(_structure)`, `estimate_text_width` | FitText, Counter, LowerThird, Captions |
| Audio spectrum | `frame.visualize_audio_frame` | SpectrumBars, Waveform |
| Subtitles | `get_subtitle_phrase`, `get_subtitle_cue`, `get_cue_stack` | Captions |
| Video frames | `get_synced_video_frame` | VideoPanel, Footage |
| Lottie | `frame.play_lottie` | LottieBox |
| Shaders | `fframes::Shader` (Skia only) | shader backgrounds, Footage grade |
| Scene overlap | `Overlap::{Previous, Next, PreviousAndNext}` | transitions |
| Design tokens | `fframes::Styles` (feature `styles`) | theme defaults for all |

### 1. Motion (`motion`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| `progress`, `clamp01`, `lerp` | `examples/vinuni-future-rises/src/lib.rs:44-49`, `examples/arm/src/effects.rs:20`, `examples/shader-mode/src/transitions.rs:5`, `examples/lottie-launch/src/lib.rs:61`, `examples/fframes-intro/src/beat.rs:45-54` | high | 5 signatures for one idea |
| Easing curves (`expo_out`, `expo_in`, `cubic_in_out`, `quart_out`, `ease_out`) | `examples/fframes-intro/src/beat.rs:58-85`, `examples/vinuni-future-rises/src/lib.rs:53-57`, `examples/shader-mode/src/outro.rs:93` | high | hand-computed motion outside `timeline!` |
| Closed-form spring | `examples/fframes-intro/src/beat.rs:90`, `examples/vinuni-future-rises/src/lib.rs:67`, `examples/shader-mode/src/outro.rs:82` | high | same math 4 times |
| Named spring presets | `examples/motion-graphics/src/lib.rs:11`, `examples/marketing/src/marketing.rs:4`, `examples/teej-podcast/src/chapter.rs:26`, plus 28 inline literals | high | map to `motion.easing.*` tokens |
| `stagger(i, step)` | inline in the template `DataScene`, `examples/fframes-studio-intro` captions | high | design.md asks for 40-60 ms staggers |
| Beat grid (`beat_time`, `beat_frame`, `snap`, `soft`, `pulse`) | `examples/fframes-intro/src/beat.rs:15-125` (duplicate in studio-intro) | medium | music-synced videos; BPM and downbeat become props |
| `hash`, `noise` | `examples/arm/src/effects.rs:9,27`, `examples/shader-mode/src/catalog.rs:33`, `examples/fframes-intro/src/beat.rs:131` | high | deterministic randomness, no `rand` |
| `scale_about(cx, cy, s)` (punch-in) | `examples/motion-graphics/src/quote_card.rs:157`, `install_scene.rs:200`, `motion_graphics.rs:89` | high | a `Transform` constructor |
| `ramp`, `rise` | `cargo-fframes/templates/lib_multi_scene.rs.tmpl:106,111`, `examples/signal-lab/src/lib.rs:79,87` | high | the scaffold's own helpers |
| Format: `timecode`, `thousands`, `parse_timecode` | `examples/fframes-intro/src/ui.rs:137,145` (fps hard-coded to 60), `examples/svg-in-3d/src/lib.rs:150`, `examples/teej-podcast/src/chapter.rs:61` (panics) | medium | take fps; return `Result` |

### 2. Text (`text`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| FitText (shrink until it fits) | `examples/motion-graphics/src/quote_card.rs:45`, `install_scene.rs:39`, template `fit_size` (`lib_multi_scene.rs.tmpl:125`), `examples/teej-podcast/src/teej_podcast.rs:31` | high | core `text_fit` only truncates; returns `None` until fonts resolve |
| Measure with fallback and cache | `examples/fframes-intro/src/ui.rs:30`, `examples/lottie-launch/src/lib.rs:127` | high | used by every layout component |
| Letters (per-glyph offset, opacity, scale) | `examples/fframes-intro/src/ui.rs:160` | high | basis for word and letter reveals |
| TextReveal (word or letter stagger) | design.md recipe; built on Letters | high | most requested title effect |
| Typewriter with blinking cursor | `examples/motion-graphics/src/install_scene.rs:121-139`, `examples/fframes-intro/src/scenes/agents.rs:200`, `examples/fframes-studio-intro/src/short/create.rs` | high | 3 implementations |
| Slam word with trails | `examples/fframes-intro/src/ui.rs:204` | medium | title entrance |
| HudLabel, SceneLabel ("01 / 02") | `examples/fframes-intro/src/ui.rs:84`, template `label` (`lib_multi_scene.rs.tmpl:116`), `examples/signal-lab/src/lib.rs:95` | medium | positions and palette become props |

### 3. Overlays (`overlay`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| Appear (enter and exit wrapper) | `examples/lottie-launch/src/lib.rs:73` (`enter_exit`) | high | wraps any `Svgr`; base for the others |
| LowerThird / name pill | `examples/conference-splash-screen/src/speaker_screen.rs:53-65,197-216`, design.md recipe | high | bar, name, role, staggered exit |
| Captions (current cue, glow optional) | `examples/audio-announce/src/audio_announce.rs:17` | high | on top of `get_subtitle_phrase` |
| CaptionStack (timed tag + line cards) | `examples/fframes-studio-intro/src/short/studio.rs:77` | high | for captions without a VTT file |
| ChapterList (sliding highlight) | `examples/teej-podcast/src/chapter.rs:32`, `teej_podcast.rs:206-250` | high | agendas, chapters, steps |
| Callout (dot, elbow line, label) | `examples/fframes-intro/src/ui.rs:100` | high | annotations, product tours |
| ProgressBar, Check | `examples/fframes-studio-intro/src/scenes/install.rs:47,58`, template footer bar (`lib_multi_scene.rs.tmpl:90-99`) | high | video progress and task UI |
| StatCard, Wordmark echo | `examples/fframes-intro/src/scenes/benchmark.rs:119`, `examples/shader-mode/src/outro.rs:13` | low | branded; keep as examples |

### 4. Data (`data`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| Counter ("0 -> 12,480") | template `DataScene` count-up, `examples/fframes-intro/src/scenes/benchmark.rs:119`, design.md recipe | high | tabular figures, right aligned |
| BarChart (staggered growing bars) | `cargo-fframes/templates/lib_multi_scene.rs.tmpl:197-233` | high | hairline start, never 0 width |
| RaceBars (lanes at relative speed) | `examples/fframes-intro/src/scenes/benchmark.rs:56`, `examples/fframes-studio-intro/src/short/fast.rs:24` | medium | benchmarks, comparisons |

### 5. Audio (`audio`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| SpectrumBars | `examples/marketing/src/marketing.rs:287-355`, `examples/tiktok/src/tiktok.rs:91`, `examples/podcast/src/podcast.rs:157,180,200`, `examples/beta/src/iphone.rs:82`, `examples/low-poly-art/src/low_poly_art/owl.rs:38` | high | most copied block in the repo |
| Waveform (smooth spline path) | `examples/audio-announce/src/svg_waves.rs:56` (spline at `:7`) | high | clamp and baseline become props |

### 6. Decoration (`decor`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| DrawOn (stroke dash reveal) | `examples/conference-splash-screen/src/speaker_screen.rs:119`, `examples/marketing/src/marketing.rs:429`, `examples/fframes-studio-intro/src/scenes/install.rs:47` | high | any path, underline, tick |
| CornerMarks / brackets | `examples/fframes-intro/src/ui.rs:68`, `examples/conference-splash-screen/src/speaker_screen.rs:105-143` | high | static or drawn on |
| Glow and drop-shadow filter defs | `examples/audio-announce/src/audio_announce.rs:28`, `examples/marketing/src/marketing.rs:300`, 9 more files | high | read `shadow.glow`, `shadow.card` tokens |
| PhotoFrame | `examples/pixel-memory/src/photo_phrame.rs:35` | high | rounded stroked image |
| TiltedCard | `examples/conference-splash-screen/src/speaker_screen.rs:180-218` | medium | skewed card with shadow |

### 7. Backgrounds (`background`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| BlobBackground (blurred gradient ellipses) | `examples/tiktok/src/tiktok.rs:42-66` | medium | static subtree, cached |
| Bokeh field | `examples/pixel-memory/src/bokeh_background.rs:40` | medium | needs a seed instead of `rng` |
| PixelWall (grid that resolves into an image or QR) | `examples/fframes-studio-intro/src/wall.rs:48` | medium | |
| Shader backgrounds (Skia only) | `examples/shaders/src/shaders.rs` (aurora), `examples/made-of-motion/src/lib.rs:81`, SkSL utilities in `examples/shader-mode/src/shaders/source-effects.sksl:10,42,104` | medium | feature-gated; tiny-skia draws nothing |

### 8. Transitions (`transition`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| CrossFade, Push | design.md recipe with `Overlap::Previous` | high | not implemented anywhere as code |
| SceneFade | `examples/vinuni-future-rises/src/lib.rs:87` | high | fade in and out at scene edges |
| SlantWipe | `examples/vinuni-future-rises/src/lib.rs:260` | medium | colors become props |
| CircleWipe | `examples/marketing/src/marketing.rs:403-410` | medium | |

### 9. Media (`media`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| KenBurns photo (slow push-in, clipped) | `examples/vinuni-future-rises/src/lib.rs:154` | high | |
| VideoPanel (clipped synced video with border) | `examples/teej-podcast/src/teej_podcast.rs:81` | medium | always with fallback image |
| Footage with grade, Reel | `examples/vinuni-future-rises/src/lib.rs:100,122,198` | medium | grade shader is Skia only |
| ScreenshotScroll | `examples/beta/src/github.rs:4` | medium | |
| LottieBox | `examples/lottie-launch/src/lib.rs:104` | low | thin over `play_lottie`; feature `lottie` |

### 10. UI mockups (`mockup`)

| Candidate | Source today | Reuse | Note |
| --- | --- | --- | --- |
| TerminalWindow | `examples/motion-graphics/src/install_scene.rs:221-280`, `examples/fframes-intro/src/scenes/agents.rs:200` | high | uses Typewriter |
| Cursor (waypoints, click rings) | `examples/fframes-studio-intro/src/short/studio.rs:102-136` | medium | product demos |
| Modal, Spinner, Log | `examples/fframes-studio-intro/src/short/create.rs:57-148` (duplicate in `scenes/first_run.rs:58-147`) | medium | |
| PhoneFrame with chat rows | `examples/beta/src/iphone.rs:7` | low | depends on bundled PNGs |

### 11. Theme (`theme`)

Every component takes explicit props, with a `from_styles(&Styles)` default. Token names
shared by all three presets (`desktop/crates/studio-presets/builtins/*/tokens.json`):

- color: `accent`, `background`, `caption`, `highlight`, `muted`, `surface`, `text`, `title`
- typography: `title`, `heading`, `body`, `caption`
- motion: `duration.{fast,base,slow,enter,exit}`, `easing.{standard,enter,exit,emphasis}`,
  `stagger.{tight,loose}`
- spacing `{xs,sm,md,lg,xl,gutter,margin}`, radius `{sm,md,lg,card}`,
  stroke `{thin,regular,thick,divider}`, shadow `{card,glow}`

### 12. 3D (`three_d`, feature `3d`)

| Candidate | Source today | Reuse |
| --- | --- | --- |
| Camera shake | `examples/arm/src/effects.rs:50` | high |
| Light beam | `examples/arm/src/effects.rs:74` | medium |
| Sparks | `examples/arm/src/effects.rs:110` | medium |
| Grid texture | `examples/arm/src/effects.rs:156` | medium |
| SVG on a 3D screen | `examples/svg-in-3d/src/lib.rs:117-145` | medium |

## Priority

1. **First (high reuse, already duplicated):** motion module, FitText, Letters and TextReveal,
   Typewriter, Counter, Appear and LowerThird, Captions, SpectrumBars, DrawOn and CornerMarks,
   CrossFade, Push and SceneFade, ProgressBar, theme defaults.
2. **Second:** Callout, ChapterList, CaptionStack, Waveform, BarChart, RaceBars,
   TerminalWindow, Cursor, KenBurns, PhotoFrame, glow filters, BlobBackground, SlantWipe,
   CircleWipe.
3. **Feature-gated or later:** shader backgrounds (Skia), 3D effects, LottieBox, PhoneFrame,
   PixelWall, Bokeh, Modal/Spinner/Log, VideoPanel, Footage.
4. **Leave in the examples:** Ferris, the low-poly animals, logo reveals, QR data, StatCard,
   Wordmark echo, and the pseudo-3D SVG painter in `examples/vinuni-future-rises/src/three_d.rs`
   (`fframes_three_d` covers it).

## Coverage

Read in full or by symbol: all 22 examples with Rust sources (`examples/the-why` has none),
`cargo-fframes/templates`, `desktop/crates/studio-project/templates`, the preset tokens and
the core crate's public `Frame` API. Some scene bodies were only scanned by symbol name:
the `pixel-memory` galleries, the `beta` scenes, most `fframes-intro/src/scenes/*` and
`fframes-studio-intro/src/{keynote,short}/*`. They may hold more small helpers, but no new
category.

## Unresolved questions

- Fork-only crate, or written to be offered upstream?
- Should Studio templates and presets use components from the start, or only the skill docs?
