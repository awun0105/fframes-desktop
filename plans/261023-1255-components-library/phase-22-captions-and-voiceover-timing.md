---
phase: 22
title: "Phase 22: Word captions, text layout and voiceover timing"
status: in-progress
priority: P1
effort: 3d
dependencies: [4]
---

# Phase 22: Word captions, text layout and voiceover timing

## Goal

Make it easy to animate to spoken words:
- TikTok-style word captions (`@remotion/captions`);
- the `@remotion/layout-utils` text fitters;
- manim-voiceover's timing model: voiceover blocks, bookmarks and the remaining duration.

None of these generate speech or recognise it.

## Context

- **`@remotion/captions`** has 3.45M downloads a month, the most of any Remotion component
  package. Its API is `Caption`, `createTikTokStyleCaptions`, `parseSrt` and
  `serializeSrt`. `manim-voiceover` has 5,706, the most of any Manim plugin.
- **The kit today.** It has sentence captions (`components/fframes-kit/src/overlay/captions.rs`,
  `Captions`, `CaptionStack`) from VTT through `ctx.get_subtitles`. It has no word timings.
- **Inputs.** Word timings are user data:
  - the JSON that Whisper and whisper.cpp write (`segments[].words[]` with `start`, `end`,
    `word`);
  - `.srt` and `.vtt` files;
  - the kit's own `Vec<CaptionWord>`.

  The kit never runs speech-to-text or text-to-speech (user decision, 2026-10-24: "timing
  only, no TTS").
- **Voiceover blocks.** The plan earlier dropped the recorded `Script` of the Manim API
  report. This phase supersedes that for timing only: a `Voiceover` reads timings that
  already exist. It never records audio and never calls a service.
- **Audio.** `fn audio` belongs to the `Video`. The voiceover helpers return the
  `AudioTrack`s for the voice file. The user adds them to their own `fn audio`, the same way
  as the phase 23 SFX helpers.

## Requirements

- **Captions:**
  - A `CaptionWord { text, start_ms, end_ms, timestamp_ms, confidence }` type, with the
    fields of Remotion's `Caption`.
    - The name differs because the kit already exports `Caption` (`overlay/captions.rs`,
      `Caption::new(time, title, body)` for `CaptionStack`). That type stays unchanged.
    - The mapping row reads: Remotion `Caption` → `CaptionWord`.
  - `parse_srt`, `serialize_srt` and `parse_vtt`, plus `from_whisper_json` (the openai
    whisper and whisper.cpp shapes).
  - `tiktok_pages(captions, combine_within_ms)`, Remotion's `createTikTokStyleCaptions`:
    pages of words with each word's own timing.
  - `WordCaptions`: draws the current page with the current word highlighted. Props:
    highlight colour, scale pop, background pill, uppercase, position, maximum lines and an
    entrance per page.
- **Text layout:**
  - `fit_text_on_n_lines`: the largest font size at which the text fits in N lines of a
    width.
  - `fill_text_box`: adds words to a box and reports when it overflows.
  - Both use the kit's font measurement (`ctx.font_source`). Map `measureText` and `fitText`
    to `text_width` and `FitText` in the mapping table.
- **Voiceover timing:**
  - `VoiceoverScript`: built from a list of blocks. Each block has its text, its audio file
    name, its start, its duration and its bookmarks.
    - Bookmarks are written in the text as `<bookmark mark="A"/>`, as in manim-voiceover.
    - Each bookmark gets its time from the block's word timings.
  - Methods:
    - `block(i)`: start, duration, end;
    - `bookmark("A")`: a time;
    - `time_until_bookmark`;
    - `remaining(i, t)`;
    - `subcaptions()`, which returns `CaptionWord`s for `WordCaptions`;
    - `audio_tracks()`, which returns `Vec<AudioTrack>`.
  - The script works with `Sequence` and `Series`, so a scene can last as long as its
    voiceover block.
- Every manim-voiceover and `@remotion/captions` name has a row in the mapping table in
  `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/overlay/word_captions.rs`;
  - `components/fframes-kit/src/text/{caption_files.rs,fit_lines.rs}`;
  - `components/fframes-kit/src/voiceover/{mod.rs,script.rs,bookmarks.rs}`;
  - in the existing `components/gallery/media/`: a short Whisper JSON and an SRT, written
    for the gallery and timed to `tones.wav`;
  - `components/gallery/src/scenes/voiceover.rs`.
- Modify:
  - `components/fframes-kit/src/{overlay/mod.rs,text/mod.rs,lib.rs,prelude.rs}`;
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` (the captions and voiceover sections).

## Tasks & Steps

1. **Caption types, parsers and the Whisper reader.** Round-trip tests come first.
2. TikTok pages and `WordCaptions`.
3. `fit_text_on_n_lines` and `fill_text_box`.
4. `VoiceoverScript`, bookmarks and audio tracks.
5. Gallery scenes and e2e frames (`WordCaptions@1s`, `WordCaptions@2.5s`, `Voiceover@end`).

## Verification

- `cargo test -p fframes-kit`:
  - SRT and VTT round trips;
  - pages for `combine_within_ms` of 0 and 1200;
  - bookmark times from word timings;
  - `fit_text_on_n_lines` never overflows its width.
- `inspect` on the gallery: no text is cut at the canvas edge.
- `just -f components/justfile e2e cpu`, `check-wasm`, `clippy` and `boundary`.

## Risks

- **Font measurement in the editor.** The text helpers return `None` without the font. Fall
  back to the raw text, as the existing text components do.

## Todo

- [x] Caption type, SRT/VTT parsers, Whisper reader
- [x] TikTok pages and `WordCaptions`
- [x] `fit_text_on_n_lines` and `fill_text_box`
- [ ] Voiceover script, bookmarks and audio tracks
- [ ] Gallery scenes, e2e frames, docs and mapping rows
