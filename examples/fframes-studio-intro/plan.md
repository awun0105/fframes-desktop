# Project Showcase Video Storyboard: `fframes-desktop`
*An Accessible, High-Impact Visual Storyboard Crafted for Everyone—From Everyday Creators to Developers*

## 1. Creative Vision & Production Specs

| Parameter | Specification |
| :--- | :--- |
| **Target Runtime** | **2 Minutes (120 seconds / 7,200 frames at 60 FPS)** — paced so non-technical viewers easily follow the story while technical viewers appreciate the depth |
| **Target Audience** | **Universal**: Creators, marketers, founders, and developers. Uses intuitive visual metaphors first, supported by real benchmarks and code/desktop UI |
| **Resolution & Format** | 1920×1080 (16:9 Landscape, 60 FPS) |
| **Visual Style & Palette** | Apple/Linear-inspired dark studio aesthetic (`#0B0F17` obsidian background, `#F8FAFC` crisp white text, `#F97316` warm Rust orange for energy, `#38BDF8` electric cyan for GPU speed, `#EF4444` coral red for "old way" pain points) |
| **Music & Rhythm** | 120 BPM upbeat electronic/percussive track (1 beat = 0.5s, 1 bar = 2.0s). Every visual transition snaps to the beat grid |
| **Signature Animation Toolkit** | **Match-cuts**, **Spring-physics pop-ins**, **Isometric 3D pipeline breakdown**, **Ghost-trail (onion-skin) motion arcs**, **Split-screen speed races**, and **Live GPU Shader backgrounds** |

## 2. Master 7-Act Storyboard Overview

| Act / Scene | Time Window | Core Story Pillar | Plain-English Message | Signature Animation & Visual Technique |
| :--- | :--- | :--- | :--- | :--- |
| **Act 1: The Problem** | `0:00–0:20`<br>(20s) | **Why Current Tools Hurt** | Manual video editors take hours of repetitive clicking; code-based browser tools take forever to render; AI agents can't "see" their mistakes. | **3-Way Split-Screen Stress Test**: Tangled timeline keyframes, a overheating snail-paced browser progress bar (`4 FPS`), and broken overlapping AI text—all shattering via a glass-break wipe. |
| **Act 2: The Solution** | `0:20–0:32`<br>(12s) | **Meet `fframes-desktop`** | Create studio-quality videos using code or AI prompts, preview live at 60 FPS on your desktop, and export in seconds. | **Morphing Portal Reveal**: A single prompt/code card expands with spring physics into the sleek `fframes-desktop` native studio window playing a buttery-smooth 60 FPS motion graphic. |
| **Act 3: Workflow Demo** | `0:32–0:55`<br>(23s) | **How You & Your AI Agent Build a Video** | Brief your AI agent (or write a scene), let automated visual/audio tools verify every frame, scrub in the desktop timeline, and hit export. | **Interactive Over-the-Shoulder Flow**: Step-by-step camera pan from prompt (`npx skills add`) → Automated visual check (`inspect`, `strip` contact sheet, `onion` motion trail, `audio analyze`) → Live desktop scrubbing. |
| **Act 4: Real Use Cases** | `0:55–1:15`<br>(20s) | **What You Can Build** | From product launch videos and TikTok/podcast visualizers to batch conference intros and 3D shader art. | **2×2 Bento Carousel**: Four live video cards flipping in 3D perspective—Product UI Explainer (`signal-lab`), Podcast & TikTok Audiograms (`podcast`/`tiktok`), 100× Batch Splash Screens, and 3D Shader Promo (`shader-mode`). |
| **Act 5: How It Works** | `1:15–1:32`<br>(17s) | **The Simple Secret Behind the Scenes** | Every frame is drawn as infinite-resolution vector shapes (SVG) combined with GPU pixel shaders, guided by a musical timeline. | **Exploded Layer Stack (3D Parallax)**: The video frame rotates 45° in 3D space, separating into 3 glowing layers: GPU Shader background + Crisp SVG Vector Shapes + Beat-Synced Audio Timeline. |
| **Act 6: Why It's So Fast** | `1:32–1:50`<br>(18s) | **The 3 Speed Engines (10× Faster)** | 1. Remembers unchanging background shapes. 2. Draws directly on your graphics card (GPU). 3. Saves straight to `.mp4` with zero middleman. | **Visual Factory Pipeline + Live Speed Race**: Animated conveyor belt showing cached static shapes bypassing work, Metal/Vulkan GPU cores lighting up, and a 128s video finishing in **36 seconds**. |
| **Act 7: Finale & CTA** | `1:50–2:00`<br>(10s) | **Start Creating Today** | Open-source, free, and ready in one command. | **Kinetic Typography Finale**: Fast beat-synced recap cuts landing on the clean terminal command and GitHub repo card (`khanhthanhdev/fframes-desktop` & [dmtrKovalenko/fframes](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2FdmtrKovalenko%2Ffframes)). |

## 3. Scene-by-Scene Script, Visual Choreography & Animation Techniques

### Act 1: Current Problems with Other Solutions (`0:00–0:20`)
* **Goal**: Make anyone—even someone who has never written code—instantly feel the frustration of existing video tools.
* **Visual Breakdown**:
  * **Problem A — Traditional Video Editors (`0:00–0:06`)**: Open on a cluttered, intimidating timeline filled with hundreds of tiny diamond keyframes. A cursor frantically drags layers around because a client changed one sentence of text.
    * *On-Screen Caption*: **"Traditional Editors: Hours of manual tweaking. Impossible to automate at scale."**
  * **Problem B — Browser-Based Code Tools (`0:06–0:13`)**: Whip-pan right to a web-based video renderer. Show a comical cartoon metaphor: a heavy web browser taking a photo of a webpage, saving it, reloading, and taking another photo—one frame at a time. A red progress bar crawls at `4.2 FPS (Estimated time left: 45 mins)`.
    * *On-Screen Caption*: **"Browser Frameworks: Screenshotting a webpage 3,000 times is painfully slow."**
  * **Problem C — "Blind" AI Video Coding (`0:13–0:20`)**: Whip-pan down to an AI chat generating video code, but the resulting video has cut-off text, missing fonts, and clipping audio because the AI cannot watch the video it made.
    * *On-Screen Caption*: **"AI Coding Agents: Great at writing code, but completely blind to broken visuals."**
* **Animation Technique**:
  * **Camera**: Fast whip-pans (`Easing::ExponentialInOut`) between the three pain points, building tension with a rising ticking clock sound and a subtle screen shake as the CPU meter hits `100%`.
* **Voiceover (Plain English)**:
  > *"Making videos today forces an awful trade-off. Traditional editors trap you in hours of manual keyframing. Code-based web tools spin up a heavy browser to screenshot every single frame at a snail's pace. And when you ask AI to code a video, it's flying blind—leaving you with cut-off text and broken timing."*

### Act 2: The Solution — Meet `fframes-desktop` (`0:20–0:32`)
* **Goal**: Deliver an immediate breath of fresh air—clean, fast, and intuitive.
* **Visual Breakdown**:
  * **The Smash Cut (`0:20`)**: On the musical bass drop, a glowing cyan shockwave sweeps away the cluttered red screens, revealing the minimalist dark interface of **`fframes-desktop`** powered by [dmtrKovalenko/fframes](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2FdmtrKovalenko%2Ffframes).
  * **The Core Promise (`0:23–0:32`)**: Show three clean floating glass cards that snap into place on the beat:
    1. **Write or Prompt** (Clean declarative scenes or natural-language AI prompts)
    2. **See Instantly** (Native 60 FPS desktop GPU preview & timeline scrubber)
    3. **Render in Seconds** (10× faster GPU rendering straight to `.mp4`)
* **Animation Technique**:
  * **Spring-Physics Card Reveal**: Each card scales from `0.85 -> 1.0` using a damped spring curve (`Spring { stiffness: 220, damping: 18 }`) with a soft glowing border trace.
* **Voiceover (Plain English)**:
  > *"Meet fframes-desktop: the programmatic video framework that is actually fast. You get the automation of code and AI agents, the instant feedback of a 60-frames-per-second desktop studio, and GPU rendering that finishes in seconds, not hours."*

### Act 3: Demo the Workflow — From Idea to Finished Video (`0:32–0:55`)
* **Goal**: Walk through the exact 4-step workflow so viewers see how humans and AI agents collaborate seamlessly.
* **Visual Breakdown**:
  * **Step 1 — Start in Seconds (`0:32–0:37`)**: A clean terminal window types `cargo fframes new launch-video` (or connects an AI agent via `npx skills add https://fframes.studio` from [fframes.studio](https://www.google.com/search?q=https%3A%2F%2Ffframes.studio%2F)). You type a simple brief: *"Make a 30-second product promo synced to a 120 BPM beat."*
  * **Step 2 — Give Your AI Agent "Eyes & Ears" (`0:37–0:46`)**: How does the AI know the video looks good without watching it? Show the four built-in visual diagnostic tools lighting up in a clean 4-panel dashboard:
    * **`inspect`**: Automatically scans every 0.25s—highlighting a piece of text that was slightly off-screen in red, then snapping it cleanly inside the safe zone in green!
    * **`strip`**: Generates a 12-frame comic-strip contact sheet (`strip.png`) so the agent sees the whole story at a glance.
    * **`onion`**: Overlays semi-transparent ghost frames (`onion.png`) showing the exact curved arc of a moving shape so motion is buttery smooth.
    * **`audio analyze`**: Draws a colorful audio waveform with LUFS volume meters, ensuring music and voiceover never clip.
  * **Step 3 — Scrub & Polish on Desktop (`0:46–0:51`)**: Zoom into the **`fframes-desktop` Native Player & Timeline Editor**. The user drags the playhead back and forth across scenes with zero lag, stepping frame-by-frame.
  * **Step 4 — Instant Export (`0:51–0:55`)**: Click **Render** (or run `render --draft` for a 1-second instant check). The progress bar zips from `0%` to `100%` in a flash.
* **Animation Technique**:
  * **Connected Node Path**: A glowing animated line draws from Step 1 → Step 2 → Step 3 → Step 4 like an interactive circuit board, using **zoom-through match cuts** into each tool's output.
* **Voiceover (Plain English)**:
  > *"Here's how the workflow feels in practice. Start a project or brief your AI coding agent. Because AI can't watch video, fframes translates your video into visual diagnostics it can actually read—automatically catching cut-off text, generating storyboard contact sheets, tracing motion arcs with onion skins, and balancing audio levels. Then, scrub through your scenes in real time in the desktop player, and export your final MP4 in seconds."*

### Act 4: Real-World Use Cases (`0:55–1:15`)
* **Goal**: Inspire viewers by showing concrete, high-value videos they can create today (grounded in real examples from [dmtrKovalenko/fframes](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2FdmtrKovalenko%2Ffframes)).
* **Visual Breakdown (2×2 Dynamic Bento Grid)**:
  * **Card 1 — Tech & Product Launch Videos (`0:55–1:00`)**: Show snippets from `fframes-intro` and `signal-lab`—sleek animated charts, UI mockups, and kinetic text locked to a music beat grid.
  * **Card 2 — Automated Podcasts & Vertical Social Shorts (`1:00–1:05`)**: Show `podcast`, `teej-podcast`, and `tiktok`—a 9:16 vertical clip and a 16:9 podcast player with reactive audio waveforms, chapter progress bars, and auto-synced subtitles (`webvtt-parser`).
  * **Card 3 — Mass Personalized & Batch Videos (`1:05–1:10`)**: Show `conference-splash-screen`—one template automatically rendering 50 different speaker intro videos (changing speaker names, photos, and talk titles from data) in a single batch run.
  * **Card 4 — Generative Art & 3D Shader Visuals (`1:10–1:15`)**: Show `shader-mode` and `low-poly-art`—mesmerizing 3D GPU raymarching visuals layered behind sharp vector titles.
* **Animation Technique**:
  * **Bento Grid Spotlight Expansion**: All 4 cards play simultaneously in a 2×2 grid; as each use case is announced, that card smoothly expands to 75% of the screen (`Easing::CubicInOut`) while the other three compress neatly to the side.
* **Voiceover (Plain English)**:
  > *"What can you build with it? Craft beat-synced product launch videos and animated data stories. Automate podcast visualizers and vertical TikTok clips complete with reactive waveforms and subtitles. Generate hundreds of personalized conference intros from a single template in batch. Or blend 3D GPU shader art with razor-sharp typography."*

### Act 5: How It Works Under the Hood (Made Simple) (`1:15–1:32`)
* **Goal**: Explain the core concept (`Rust + SVG + Shaders + Timeline`) visually so anyone grasps why it produces such clean results.
* **Visual Breakdown**:
  * Show a finished video frame in the center of the screen.
  * **The 3D Exploded View (`1:18`)**: The flat video frame tilts in 3D isometric perspective and separates into three floating transparent glass layers:
    1. **Bottom Layer — GPU Shaders (SkSL / Shadertoy GLSL)**: Rich, animated lighting, gradients, and 3D effects running on the graphics card.
    2. **Middle Layer — Infinite-Resolution Vector Graphics (SVG)**: Shapes, icons, and text defined mathematically (`svgr!`). Zoom in 1,000% on a letter to show the edge stays razor-sharp at HD or 4K UHD.
    3. **Top Layer — The Musical Timeline (`fframes::timeline!`)**: A clean horizontal timeline bar showing simple rules: *At 0s, slide in; at 2s, bounce with spring physics*.
  * **The Re-Merge (`1:28`)**: The three layers snap back together into a single glowing 60 FPS video frame.
* **Animation Technique**:
  * **Isometric 3D Layer Separation**: Use SVG skew/scale transforms to create a faux-3D parallax stack with floating callout labels connected by thin leader lines.
* **Voiceover (Plain English)**:
  > *"How does it work? Think of every video frame as three simple layers stacked together. On the bottom, optional GPU shaders create rich dynamic backgrounds. In the middle, every shape, chart, and word is drawn as mathematical vector graphics—so it stays razor-sharp whether you export for a phone or a 4K screen. And on top, a human-readable timeline tells every element exactly when to move, bounce, or fade."*

### Act 6: Why Is It So Fast? (The 3 Speed Engines) (`1:32–1:50`)
* **Goal**: Turn complex systems engineering (static caching, Skia Metal/Vulkan, in-process `libav`) into an unforgettable visual explanation.
* **Visual Breakdown**:
  * **Engine 1 — Smart Memory / Compile-Time Caching (`1:32–1:38`)**: Show a video scene where the background card and logo don't change, while only a chart line moves. The background turns cool cyan with a padlock icon (`Cached Once — 0ms Redraw`), while only the moving chart line is updated.
    * *Plain-English Label*: **"1. Smart Caching: Never redraws parts of the screen that didn't change."**
  * **Engine 2 — Direct GPU Drawing via Skia (`1:38–1:44`)**: Show a highway bypass. The "Old Way" exit goes through a congested tollbooth labeled `Web Browser / CPU`. The `fframes` express lane shoots straight into a glowing `GPU (Apple Metal / Vulkan)` chip running **10× faster**.
    * *Plain-English Label*: **"2. Pure GPU Power: Draws directly on Metal & Vulkan—10× faster than CPU."**
  * **Engine 3 — Direct-to-MP4 Encoding (`1:44–1:50`)**: Instead of dumping thousands of temporary PNG files onto a hard drive, show frames flowing directly through an internal pipe (`FFmpeg libav in-memory`) straight into an `.mp4` file.
    * *Plain-English Label*: **"3. Zero Middleman: Encodes video directly in memory."**
  * **The Ultimate Proof Callout (`1:47–1:50`)**: Big bold counter slams onto the screen:
    * **Real Benchmark: 128-second HD launch video rendered in just 36 seconds** ([fframes.studio](https://www.google.com/search?q=https%3A%2F%2Ffframes.studio%2F)).
* **Animation Technique**:
  * **Particle Stream & Counter Slam**: High-speed glowing particles flow through the 3-stage pipeline, culminating in a fast numerical odometer roll (`128s video -> 36s render`).
* **Voiceover (Plain English)**:
  > *"Why is it so fast? Three reasons. First, smart caching: if a background or logo doesn't change between frames, fframes remembers it instead of redrawing it. Second, it skips the web browser entirely and draws directly on your graphics card using Metal or Vulkan—ten times faster than CPU rendering. Third, it links the FFmpeg video encoder directly in memory instead of saving thousands of temporary image files. The result? A two-minute HD video renders in just 36 seconds."*

### Act 7: Outro & Call to Action (`1:50–2:00`)
* **Goal**: End on a high-energy crescendo with clear next steps.
* **Visual Breakdown**:
  * Rapid 2-second montage of all previous animations zooming out into the `fframes-desktop` application window.
  * Transition to a clean, bold end card featuring:
    * Quickstart command: `cargo install --locked cargo-fframes && cargo fframes new my-video`
    * AI Agent Skill: `npx skills add https://fframes.studio`
    * Repository links: [khanhthanhdev](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Fkhanhthanhdev) (`khanhthanhdev/fframes-desktop`) & [dmtrKovalenko/fframes](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2FdmtrKovalenko%2Ffframes)
* **Animation Technique**:
  * **Zoom-Out Mosaic + Neon Border Trace**: All scenes tile into a mosaic that collapses into the central logo lockup with a final resonant beat hit.
* **Voiceover (Plain English)**:
  > *"Stop waiting on slow renders. Whether you're crafting videos by hand or vibe-coding with AI, try fframes-desktop today—open source on GitHub."*

## 4. Cheat Sheet of Animation Techniques to Use in the Video

| Technique Name | Where to Use It | How to Achieve It in `fframes` |
| :--- | :--- | :--- |
| **Spring-Physics Pop-In** | Act 2 solution cards & Act 6 benchmark numbers | Animate `Transform::scale` from `0.8` to `1.0` using spring easing parameters so UI cards overshoot slightly and settle naturally. |
| **Whip-Pan & Glass Wipe** | Act 1 -> Act 2 transition (breaking away from old tools) | Combine a fast `Transform::translate` (`Easing::ExponentialInOut`) with a diagonal SVG `<clipPath>` polygon sweep across the 1920×1080 canvas. |
| **3D Isometric Layer Stack** | Act 5 ("How It Works" layer breakdown) | Apply composite SVG transforms (`rotate(-30) skewX(30) scale(1, 0.86)`) to three separate `<g>` groups and animate their vertical `y` offset apart. |
| **Real Onion-Skin Ghosting** | Act 3 workflow demo | Embed actual output from `cargo run --release -- onion "<scene>@0..1s" -n 6 onion.png` to show authentic motion-trail verification. |
| **Hybrid Shader + SVG Masking** | Act 4 & Act 5 visual showcases | Render a Shadertoy GLSL or SkSL shader layer clipped inside a rounded SVG `<rect rx="24">` card with crisp `JetBrains Mono` typography on top. |
| **Beat-Locked Cut Grid** | Throughout all 120 seconds | At 120 BPM, place major scene changes on multiples of `2.0s` (1 bar) and bullet/card reveals on multiples of `0.5s` (1 beat) using `fframes::timeline!`. |
