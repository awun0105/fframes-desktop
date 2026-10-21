# Feature Comparison: desktop video editor UI

## Source: CapSoftware/Cap

| App | Path | Commit read |
| --- | --- | --- |
| Tauri editor (shipping) | `apps/desktop/src/routes/editor`, `apps/desktop/src-tauri` | [`4cacde7`](https://github.com/CapSoftware/Cap/commit/4cacde7a70bfbb11844faf23486518c6532b5a6b) |
| GPUI editor (native port) | `apps/desktop-gpui/src` | [`88bce03`](https://github.com/CapSoftware/Cap/commit/88bce03d6d3ffdf1a23364792599fdf74a92bb77) |

Licence: Cap is AGPL-3.0. `docs/desktop/design-system.md` records that the Studio kit was
ported "with the permission of Cap's owners". Further copying needs the same clearance.

## Local Project: fframes Studio (`desktop/`, GPUI)

Studio already follows Cap: `studio-ui/src/kit` (Button, EditorButton, Slider, Segmented,
TabRail, SettingsFrame, caption glyphs), `studio-ui/src/timeline` ("follows Cap's editor
timeline", including `minimap.rs` and Cap's drag snapping in `edit.rs`) and the Cap colour
tokens in `studio-theme`. This report is about what Cap does that Studio does not.

## How Cap builds the editor UI

### Tauri app (SolidJS 1.9 + Tailwind v4 + Kobalte)

```text
Editor.tsx: EditorInstanceContextProvider > EditorContextProvider > Inner
├─ Header.tsx          name, undo/redo, presets, Export button, custom caption buttons
├─ row
│  ├─ Player.tsx       toolbar (aspect, crop, quality) · PreviewCanvas · DOM overlays · transport
│  └─ ConfigSidebar    tabs: background, camera, audio, cursor, keyboard, captions, ...
│                      (swaps to text/image/style segment inspectors when a segment is selected)
└─ Timeline/index.tsx  resizable; ClipTrack, ZoomTrack, SceneTrack, MaskTrack, TextTrack,
                       AudioTrack, CaptionsTrack, ... built on Track.tsx primitives
                       (TrackRoot, SegmentRoot, SegmentHandle), TrackManager, Minimap
ExportPage.tsx         replaces the whole editor surface while exporting
```

- **State.** `context.ts` holds two Solid stores: `project` (persisted `ProjectConfiguration`,
  normalised on load, serialised before IPC) and `editorState` (playhead, selection, timeline
  `{zoom, position}`, hover). Undo (`@solid-primitives/history`) watches `project` only.
- **Saving.** `project-config-save.ts`: debounced by 250 ms, one save in flight, loops until
  the saved snapshot equals the latest one, `flush()` before close and export.
  `updateProjectConfigInMemory` updates the preview without waiting for the save.
- **Frames.** wgpu renderer (`crates/rendering`) → editor callback → Tokio `watch` → Axum
  WebSocket per editor → `utils/socket.ts` `createImageDataWS` → WebGPU (NV12/RGBA) or Canvas2D,
  with an optional OffscreenCanvas worker and a SharedArrayBuffer ring.
- **Commands.** tauri-specta generates typed `commands.seekTo/startPlayback/stopPlayback`
  and events. While paused, scrubbing sends `renderFrameEvent` at most 60 times a second.
- **Overlays.** Rust renders the frame. Selection boxes, handles and text or caption editing
  are DOM overlays that follow layout geometry emitted by Rust.
- **Timeline.** Segments are positioned by `(start - position) / secsPerPixel`. The lanes
  render only the visible range plus 2 s on each side. `PlaybackFollow` scrolls the view
  unless the user is interacting. Handles are 20 px. Split snapping is in
  `split-snapping.ts`. Figma-style canvas snapping is in `snapping.ts`.
- **Tokens.** `styles/theme.css` `--ed-*` and `--track-*`, mapped to Tailwind utilities.

### GPUI app (closest to Studio)

- `editor_window.rs` coordinates. `EditorSectionView` splits redraws into Header, Toolbar,
  Transport, Sidebar and Timeline entities that observe `EditorWindow`. `PreviewFrameView` is
  its own entity, so a new frame repaints only the canvas.
- **Frame hand-off.** `PlayheadSignal` keeps the latest frame in an atomic and wakes the UI
  through a one-slot `flume` channel, so the newest frame wins. `run_transport` runs engine
  work on Tokio. `unpad_rgba_to_bgra` produces an `Arc<RenderImage>`. macOS paints a
  `CVPixelBuffer` with `paint_surface_fitted`.
- **Edits.** `editor_edits.rs` has no GPUI code: `hit_test` → `Hit::{Handle, Body, Empty}`
  (10 px edge zone, tested in reverse paint order), `min_segment_duration` (seconds and pixel
  floor), `trim_start_bounds` / `trim_end_bounds` / `move_bounds`, `TrackSegmentOps`.
- **History.** `ProjectHistory` keeps up to 100 whole-config snapshots. `pause()` / `resume()`
  makes a drag or slider gesture one undo step.
- **Drag.** `DragKind` (move, trim, clip trim, create zoom segment) and `Drag` (threshold,
  modifiers, history bracket). A clip trim edits a `ClipDraft` and leaves the project alone;
  on release it commits once and animates into the repacked layout.
- **Theme.** `theme.rs` contains resolved Radix/Tailwind tokens, system/light/dark and
  `MaterialTokens` for macOS vibrancy. `ui/` components have per-surface constructors
  (`body`, `settings`, `glass`, `plain`).

## Head-to-Head

| Aspect | Cap | Studio | Recommendation |
| --- | --- | --- | --- |
| Source of truth | Structured `ProjectConfiguration` edited directly | Rust source; only `timeline.json` is edited directly, other changes go through an agent | Keep. Cap's inspector model fits only `timeline.json` and runtime parameters |
| Layout | Header · player + 416 px inspector · timeline | Header · rail + drawer · player/timeline · agent column · status strip | Keep. The agent column replaces Cap's inspector on purpose (design-system.md) |
| Redraw scope | Section entities plus a separate `PreviewFrameView` | `shell/sections.rs` `SectionView` (header, rail, player, agent); a frame notifies only the player | Already done |
| Frame transport | wgpu → callback → latest-wins signal → `RenderImage` | Worker process → loopback binary → coalesced latest target with stale-epoch rejection → `frame_image.rs` BGRA | Same idea; Studio's version is stricter. Keep |
| Edit maths outside the UI | `editor_edits.rs` | `studio-engine/src/timeline.rs` plus `timeline/edit.rs` | Already done |
| Drag during a trim | `ClipDraft` stays local, one commit on release | Ghost trim: `EditDrag::draft` in `timeline/view.rs`. `timeline_edits.rs` also sends a trim per pointer move (a queued trim of the same edge is replaced) so the preview retimes during the drag | Studio's choice gives a live preview at the cost of engine round trips. Switch to commit-on-release only if drags lag |
| Undo grouping | `pause`/`resume` on snapshots | One undo step per drag, done by the engine | Equivalent |
| Canvas overlays | DOM/GPUI overlays from renderer layout geometry, Figma snapping | Canvas selection through worker hit testing (`canvas_view.rs`) | Cap's snapping matters only if direct move/resize on the canvas is added |
| Track kinds | 9+ track types on shared track primitives | Scene rows, audio, mix waveform | Add shared primitives only when a second editable track kind exists |
| Lane virtualisation | Visible range plus 2 s | `viewport.segment_visible` (`timeline/view.rs`, "Cap's `isSegmentVisible`") | Already done |
| Follow playhead | `PlaybackFollow` scrolls the view during playback unless the user is interacting | Not found. Zoom keeps the playhead in place, but nothing scrolls during playback | **Gap.** Small and self-contained |
| Export | Full-surface page, persisted settings, flush before export | Full-surface panel (`export/`) | Already similar |
| Theme | `MaterialTokens` (vibrancy) | `studio-theme` with Cap colours | Optional, macOS only |

## Recommendation

Studio already uses Cap's layout, kit, timeline geometry and frame contract, so there is
nothing large left to port. Cap's model of editing a structured config does not fit
Studio's "Rust source is the authority" design. Section redraws, lane virtualisation, ghost
trim, snapping, the minimap and the frame contract are already ported. What remains:

1. **Playback follow** (Cap's `PlaybackFollow` in `Timeline/index.tsx`). The one clear gap.
2. **Commit-on-release trims** (`ClipDraft`). Only if live retiming during a drag is too slow.
3. **Canvas move/resize with Figma-style snapping** (`snapping.ts`, `editor_canvas.rs`). Only if
   Studio adds direct canvas edits that write to `timeline.json` or runtime parameters.

Risk: low (0 critical assumptions). The only gate is the AGPL-3.0 licence if code is copied
rather than re-implemented.
