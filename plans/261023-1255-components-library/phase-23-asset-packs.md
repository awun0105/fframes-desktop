---
phase: 23
title: "Phase 23: Cursor, animated emoji and sound effects"
status: completed
priority: P3
effort: 3d
dependencies: [11]
---

# Phase 23: Cursor, animated emoji and sound effects

## Goal

Add the asset packs the user chose (2026-10-24):
- a drawn cursor with click and drag animations;
- animated emoji played from user-supplied Noto Lottie files;
- a small CC0 sound-effect pack with helpers that place hits on transitions and beats;
- a `Gif` component that plays animated GIFs (user, 2026-10-24: "just implement Gif if
  not touch the fframes core").

## Context

- **Cursor.** Remotion has `@remotion/mac-cursors`. The kit draws its own cursors, with no
  Apple artwork (user, 2026-10-24).
- **Animated emoji.**
  - Remotion's `@remotion/animated-emoji` uses the Google Noto animated emoji. They are
    Lottie files under CC BY 4.0.
  - fframes plays Lottie files with its `lottie` feature (AGENTS.md section 7d,
    `frame.play_lottie`).
  - The kit's `lottie` feature is empty today. It must forward `fframes/lottie`.
  - The kit ships the emoji names, the URL pattern and the attribution text, not the files.
    The user downloads the files they use into their media folder.
- **Sound effects.**
  - Remotion's `@remotion/sfx` has whoosh, ding, mouse click, page turn, shutter, UI switch
    and others. Its licence differs per file.
  - The kit ships only files whose CC0 origin is recorded.
  - If no suitable CC0 file is found for a sound, that sound is left out and the phase report
    says so.
- **No core change.** `fn audio` belongs to the user's `Video`. The SFX helpers return
  `Vec<AudioTrack>` that the user adds to their own `AudioMap`.
- **Embedding.** Embedded audio is decoded at compile time by `include_media_dir!`. The
  pack is a folder the user copies or points to. The kit does not embed it into every
  build.
- **GIF.**
  - Remotion's `<Gif>` (`@remotion/gif`) is used in 4,258 files of the sweep.
  - It needs no core change. The `image` crate's `gif` feature decodes the frames, and
    `fframes::rendered_image(width, height, rgba)` turns premultiplied RGBA pixels into an
    image for `<image href>`.
  - `rendered_image` exists only off wasm32
    (`#[cfg(not(target_arch = "wasm32"))]` in `fframes-media/src/images.rs`). The editor
    gets no GIF frames.
  - `rendered_image` gives every call a new id, so a cache keyed by the id never hits
    when it is called per frame. Build each frame's image once.

## Requirements

- **Cursor:**
  - `Cursor`: an arrow, pointer (hand), I-beam, grab, grabbing, or crosshair, drawn as SVG
    paths, in light and dark styles.
  - `CursorPath`: moves through timed points with eased or spring motion, and slight
    overshoot on arrival.
  - `Click`: a press scale, then a ripple ring.
  - `Drag`: the grabbing cursor moves a child along with it.
  - `TypeAt`: a cursor next to the kit's `Typewriter`.
- **Animated emoji:**
  - `AnimatedEmoji { name, x, y, size, start, looping }`. It plays `"<name>.json"` from
    media through `play_lottie`.
  - A missing file renders nothing, and `inspect` warns with the download URL.
  - `emoji::NAMES`: the Noto names and their code points.
  - `emoji::ATTRIBUTION`: the text to put in the video credits.
  - The skin-tone variants of Remotion's API are a prop.
- **Sound effects:**
  - The pack goes in `components/fframes-kit/sfx/`. It has a few short files (whoosh,
    click, ding, pop, swoosh-down, shutter), each with a `LICENSE.md` entry giving its
    source URL and the words of the CC0 dedication.
  - `sfx::on_transitions(series, Sfx::Whoosh)` returns one `AudioTrack` per transition of a
    `TransitionSeries`, placed at the overlap start, with gain and a random variation from
    `rate`.
  - `sfx::on_beats(beat, range, Sfx::Click)` and `sfx::at(times, Sfx::Pop)` work the same
    way.
  - `sfx::on_clicks(cursor_path)` returns one track for each cursor click.
  - Each `Sfx` names its file, so the user can replace a file with their own.
- **GIF:**
  - `Gif::decode(bytes)` decodes every frame once, in the user's constructor: the
    composited RGBA frames and their delays (`GifDecoder::into_frames()`), converted to
    premultiplied alpha and turned into images once.
  - `Gif { gif, x, y, width, height, fit, start, speed, looping }` picks the frame for the
    local time from the delays and draws it as `<image>`. Without `looping` the last frame
    stays.
  - In the editor (wasm32) it draws a placeholder rectangle of the same size, and the
    docs say so.
  - The `image` dependency sits behind a new kit feature `gif`.
- Each of the packs gets a section in `docs/components/components.md`, and each
  Remotion package has a row in the mapping table in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/assets/{mod.rs,cursor.rs,emoji.rs,sfx.rs,gif.rs}`;
  - a small GIF made for the gallery (drawn frames, no third-party file) in
    `components/gallery/media/`;
  - `components/fframes-kit/sfx/` (audio files and `LICENSE.md`);
  - `components/gallery/src/scenes/assets.rs`.
- Modify:
  - `components/fframes-kit/Cargo.toml` (`lottie` forwards `fframes/lottie`; a `gif`
    feature with the `image` crate's `gif` feature; the `include` list for the SFX
    folder);
  - `components/fframes-kit/src/{lib.rs,prelude.rs}`;
  - `components/gallery/{Cargo.toml,src/scenes/mod.rs}`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md`.

## Tasks & Steps

1. **Cursor shapes and `CursorPath`, then `Click`, `Drag` and `TypeAt`.**
2. **The emoji wrapper and name table.**
   - Check that `play_lottie` draws one real Noto file.
   - The test file is downloaded for local verification only and is not committed.
   - Record the result in the phase report.
3. **Fetch the CC0 sound files.** Every sound has a source, all checked as CC0 on
   2026-10-24:
   - Kenney "Interface Sounds" (<https://kenney.nl/assets/interface-sounds>, 100 files):
     click, switch and pop.
   - qubodup's bamboo swooshes on OpenGameArt (<https://opengameart.org/node/2242>):
     whoosh.
   - Ding: "Pleasing Bell Sound Effect" by Spring Spring on OpenGameArt
     (<https://opengameart.org/content/pleasing-bell-sound-effect>, `pleasing-bell.wav`).
   - Shutter: "Camera" by themightyglider on OpenGameArt
     (<https://opengameart.org/node/73696>, `photo.ogg`, made from a CC0 file at
     archive.org/details/CameraShutterSound).
     - Alternative: Freesound 539136, "DSLR Shutter fast 006" by ristooooo1, the file
       Remotion uses. Freesound downloads need a logged-in account, so the loop cannot
       fetch it unattended.
   - Check each page's licence again when downloading.

   Then for each file:
   - Record each file's origin and licence in `sfx/LICENSE.md` and the phase report.
   - Trim each file to under 2 s. Normalise it with the kit's audio chain, or with ffmpeg
     before committing.
4. SFX helpers.
5. `Gif`: decoding, frame timing, the editor placeholder.
6. Gallery scenes (cursor clicks with SFX, emoji with a missing-file warning, transitions
   with whooshes, a looping GIF) and e2e frames (`Cursor@50%`, `Click@end`, `Gif@50%`).
7. A mapping row for `@remotion/gif` in `lib.rs`.

## Verification

- `cargo test -p fframes-kit`:
  - `on_transitions` gives one track per transition at the overlap start;
  - `on_beats` matches the beat grid;
  - `CursorPath` reaches each point at its time;
  - `Gif` picks the right frame from the delays, loops, and holds the last frame without
    `looping`.
- `audio analyze` on the gallery's SFX scene: no clipping, true peak below -1 dBTP.
- `inspect` warns about the missing emoji file with its URL.
- `just -f components/justfile e2e cpu`, `check-wasm` (with and without `lottie`, and with
  `gif`), `clippy` and `boundary`.

## Risks

- **Licensing.** No file is committed without a recorded CC0 source. Emoji files are never
  committed. If the CC0 search fails, ship the helpers with an empty pack and say so.
- **Crate size.** Keep the SFX folder under 1 MB. Exclude it from the crates.io package if
  it grows, and document the download instead.

## Todo

- [x] Cursor shapes, path, click, drag, type
- [x] Emoji wrapper and name table, one real file checked locally
- [x] CC0 sound files with recorded origins
- [x] SFX helpers
- [x] `Gif` component
- [x] Gallery scenes, e2e frames, docs and mapping rows
