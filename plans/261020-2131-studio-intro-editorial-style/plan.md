# Studio intro 16:9 cut in the Editorial Engineering style

Status: done

## Outcome

The 16:9 keynote cut of `examples/fframes-studio-intro` follows
`docs/design/README.md` and `design-system/tokens.css`: a light paper canvas,
Geist for claims and explanations, Geist Mono only for commands, identifiers,
stage numbers and metadata, semantic state colours, 4 px radii, thin rules and
no elevation.

## Constraints and non-goals

- Portrait (9:16) and short cuts keep their look: `ui.rs` and the existing fonts
  stay as they are; the keynote gets its own tokens in `scenes/keynote/look.rs`.
- Timing, story, audio and copy stay unchanged, except the act 6 measurement:
  this cut renders in 26.0 s on 32 Xeon Icelake cores (4 sockets of 8), not the
  72 s measured for the dark cut.
- Fonts: static instances (Geist 400/450/500, Geist Mono 400/500) of the repository's Geist woff2 files
  (`landing/studio/fonts`), licenses in `assets/`.

## Decisions

| Old role                 | New role                                                     |
| ------------------------ | ------------------------------------------------------------ |
| black canvas             | `--ff-surface-canvas` paper                                  |
| bone text / slate / grey | primary / secondary text, rule                               |
| blue accent (buttons)    | action role, in blue `#2350b5` at the user's request (not the green `--ff-action-primary`) |
| sky (GPU, speed, draft)  | candidate slate blue `#4a6f9c`                               |
| green check              | accepted green, always naming what passed                    |
| red "old way"            | video-local failure exception (the spec has no error role)   |
| code block, agent frame, shader layer | inverse surface in navy `#14243f` instead of forest (tool panels stay paper) |
| grain, vignette, drop flash, glitch, pump | removed from the 16:9 cut: they communicate no change |

`→` and `✓` are not in the Geist Latin subset; they are drawn as vector paths.

## Acceptance

- `inspect` reports no missing fonts or glyphs for the 16:9 cut.
- Contact sheets of every act reviewed, plus full-size crops of the arrows,
  the agent chat, the export check and the text-dense acts 5 to 7.
- Portrait and short frames are pixel-identical to the baseline.
- `cargo clippy -p fframes-studio-intro -- -D warnings` and `cargo fmt` clean.
