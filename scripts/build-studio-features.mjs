// Homepage chapters: project ownership, timing, SDK preparation and export.
import { readFileSync, writeFileSync } from "node:fs";
const file = "landing/studio/index.html";
const figure = (
  name,
  title,
  desc,
  body,
  caption
) => `<figure class="product-figure" data-product-figure="${name}">
<div class="product-figure-heading"><span>${title}</span><span>ILLUSTRATION · SCROLL TO EXPLORE</span></div>
<p class="product-figure-hint">Scroll the figure sideways to see every detail ↔</p>
<div class="product-figure-scroll" tabindex="0" role="region" aria-label="${title}; scroll horizontally on small screens">
<svg viewBox="0 0 640 360" role="img" aria-labelledby="product-${name}-t product-${name}-d"><title id="product-${name}-t">${title}</title><desc id="product-${name}-d">${desc}</desc>${body}</svg></div>
<figcaption>${caption}</figcaption></figure>`;
const project = figure(
  "project",
  "YOUR FILES, CONNECTED",
  "Rust source, media and timing remain ordinary project files. Studio uses them to build and prepare a video preview.",
  `
<path d="M35 300H606" class="pf-guide"/>
<g class="pf-file pf-file-one"><path d="M40 66h80l13 14h155v52H40Z" class="pf-paper"/><text x="57" y="104" class="pf-code">src/lib.rs</text><text x="223" y="104" class="pf-micro">RUST</text></g>
<g class="pf-file pf-file-two"><path d="M40 148h80l13 14h155v52H40Z" class="pf-blue"/><text x="57" y="186" class="pf-code">media/</text><path d="M236 175h28v24h-28Zm4 19 8-10 6 6 5-4" class="pf-edge"/></g>
<g class="pf-file pf-file-three"><path d="M40 230h80l13 14h155v52H40Z" class="pf-green"/><text x="57" y="268" class="pf-code">timeline.json</text></g>
<path d="M307 100h30v80h37m-67 3h67m-67 80h30v-83" class="pf-edge"/>
<g class="pf-flow"><path d="M340 174h12v12h-12Z" class="pf-signal"/></g>
<g class="pf-output"><rect x="385" y="88" width="225" height="184" rx="4" class="pf-paper"/><path d="M385 119h225" class="pf-edge"/><text x="401" y="108" class="pf-micro">STUDIO PREVIEW</text><rect x="401" y="134" width="193" height="99" class="pf-green"/><circle cx="498" cy="183" r="29" class="pf-pink"/><path d="m490 167 23 16-23 16Z" class="pf-fill-ink"/><path d="M403 252h170m-170 0h61" class="pf-edge"/><circle cx="467" cy="252" r="3" class="pf-fill-green"/></g>
<text x="40" y="332" class="pf-micro">ORDINARY FILES. YOUR OWN TOOLS.</text><text x="610" y="332" text-anchor="end" class="pf-micro">SOURCE → FRAME</text>`,
  "A project structure, not a hidden document format. The preview requires a compatible project and a successful build."
);
const timing = figure(
  "timing",
  "CHANGE THE CUT, KEEP THE CODE",
  "A scene is split, the outro moves to the beginning and a clip edge is trimmed. Timing changes are written to timeline.json; Rust source stays unchanged.",
  `
<text x="32" y="40" class="pf-micro">ORIGINAL SCENE ORDER</text><path d="M32 54H608" class="pf-guide"/>
<rect x="32" y="69" width="168" height="35" class="pf-faint"/><text x="45" y="92" class="pf-micro">A / INTRO</text><rect x="212" y="69" width="168" height="35" class="pf-faint"/><text x="225" y="92" class="pf-micro">B / MAIN</text><rect x="392" y="69" width="168" height="35" class="pf-faint"/><text x="405" y="92" class="pf-micro">C / OUTRO</text>
<text x="32" y="145" class="pf-micro">EDITED TIMELINE</text><path d="M32 205H607" class="pf-edge"/>
<g class="pf-clip-a"><rect x="32" y="163" width="168" height="62" rx="2" class="pf-blue"/><rect class="pf-trim-mask" x="32" y="163" width="24" height="62" fill="#fafbf8"/><path class="pf-trim-edge" d="M32 160v68"/><text x="69" y="200" class="pf-code">A / INTRO</text></g>
<g class="pf-clip-b"><rect x="212" y="163" width="81" height="62" rx="2" class="pf-green"/><text x="225" y="200" class="pf-code pf-split-label">B₁</text><g class="pf-split-half"><rect x="293" y="163" width="81" height="62" rx="2" class="pf-green"/><text x="309" y="200" class="pf-code pf-split-label">B₂</text></g><text x="225" y="200" class="pf-code pf-unsplit-label">B / MAIN</text><path class="pf-split-mark" d="M293 150v87"/></g>
<g class="pf-clip-c"><rect x="392" y="163" width="168" height="62" rx="2" class="pf-pink"/><text x="405" y="200" class="pf-code">C / OUTRO</text></g>
<path d="M300 243v20" class="pf-edge"/><rect x="32" y="276" width="270" height="53" rx="3" class="pf-green"/><text x="48" y="307" class="pf-code">timeline.json → written</text><rect x="326" y="276" width="282" height="53" rx="3" class="pf-paper"/><text x="342" y="307" class="pf-code">Rust source → unchanged</text>`,
  "Illustrative split → reorder → trim. Timing-only edits do not rebuild Rust. Timing changes are unavailable while an agent task is running."
);
const sdk = figure(
  "sdk",
  "FROM SETUP TO A WORKING PREVIEW",
  "Studio installs a managed SDK. A compatible example project builds with it and prepares a preview worker, frame and audio before replacing the preview.",
  `
<path d="M45 291H605" class="pf-guide"/>
<g class="pf-sdk-case"><path d="m48 160 112-61 112 61-112 61Z" class="pf-green"/><path d="m48 160v30l112 61 112-61v-30l-112 61Z" class="pf-paper"/><text x="160" y="176" text-anchor="middle" class="pf-code">MANAGED SDK</text></g>
<g class="pf-tool pf-tool-one"><rect x="58" y="47" width="90" height="35" rx="3" class="pf-blue"/><text x="76" y="70" class="pf-code">Rust</text></g><g class="pf-tool pf-tool-two"><rect x="164" y="47" width="102" height="35" rx="3" class="pf-pink"/><text x="179" y="70" class="pf-code">fframes</text></g>
<path d="M284 174h51m-7-6 7 6-7 6" class="pf-edge"/>
<g class="pf-example"><rect x="344" y="103" width="112" height="147" rx="4" class="pf-paper"/><path d="M363 126h48l13 13v81h-61Z" class="pf-blue"/><text x="375" y="171" class="pf-code">&lt;/&gt;</text><text x="400" y="236" text-anchor="middle" class="pf-micro">EXAMPLE</text></g>
<path d="M469 174h32m-7-6 7 6-7 6" class="pf-edge"/>
<g class="pf-ready"><rect x="512" y="123" width="95" height="104" rx="4" class="pf-green"/><path d="m535 162 14 14 25-31" class="pf-check"/><text x="560" y="209" text-anchor="middle" class="pf-micro">PREVIEW</text></g>
<text x="48" y="326" class="pf-micro">01 / INSTALL SDK</text><text x="344" y="326" class="pf-micro">02 / BUILD EXAMPLE</text><text x="607" y="350" text-anchor="end" class="pf-micro">03 / PREPARE WORKER, FRAME &amp; AUDIO</text>`,
  "A preparation sequence, not elapsed build time. Failed or cancelled builds keep the previous preview."
);
const output = figure(
  "export",
  "TURN THE PROJECT INTO A VIDEO",
  "The project's preview worker exposes its supported export formats. Studio renders the chosen output with progress and cancellation.",
  `
<text x="32" y="46" class="pf-micro">YOUR VIDEO, FRAME BY FRAME</text>
<g class="pf-export-frame pf-export-one"><rect x="32" y="80" width="110" height="130" rx="3" class="pf-paper"/><rect x="42" y="94" width="90" height="83" class="pf-blue"/><circle cx="72" cy="136" r="16" class="pf-pink"/><text x="43" y="197" class="pf-micro">FRAME 001</text></g>
<g class="pf-export-frame pf-export-two"><rect x="156" y="80" width="110" height="130" rx="3" class="pf-paper"/><rect x="166" y="94" width="90" height="83" class="pf-blue"/><circle cx="211" cy="136" r="16" class="pf-pink"/><text x="167" y="197" class="pf-micro">FRAME 002</text></g>
<g class="pf-export-frame pf-export-three"><rect x="280" y="80" width="110" height="130" rx="3" class="pf-paper"/><rect x="290" y="94" width="90" height="83" class="pf-blue"/><circle cx="353" cy="136" r="16" class="pf-pink"/><text x="291" y="197" class="pf-micro">FRAME 003</text></g>
<path d="M405 144h52m-8-6 8 6-8 6" class="pf-edge"/>
<g class="pf-video-file"><path d="M478 75h82l45 44v118H478Z" class="pf-green"/><path d="M560 75v44h45" class="pf-edge"/><path d="m523 146 30 18-30 18Z" class="pf-fill-green"/><text x="542" y="216" text-anchor="middle" class="pf-code">VIDEO FILE</text></g>
<rect x="32" y="265" width="573" height="8" rx="4" class="pf-faint"/><rect class="pf-render-bar" x="32" y="265" width="573" height="8" rx="4"/>
<text x="32" y="310" class="pf-micro">WORKER-SUPPORTED FORMAT</text><text x="605" y="310" text-anchor="end" class="pf-micro">RENDER → SAVE</text>`,
  "Illustrative frames and render progress. Available formats, size and frame rate come from the project’s worker."
);
const features = [
  [
    "project",
    "01 / YOUR PROJECT",
    "Open files.<br>Keep control.",
    "Your video is a Rust project: source, media and a timing document. Open it in your editor, keep it in Git, and bring the tools you already use.",
    "Studio adds a workspace around those files. The project remains yours.",
    project,
  ],
  [
    "timing",
    "02 / THE TIMELINE",
    "Change the rhythm.<br>Keep the source.",
    "Split a scene, move the outro, or trim a clip’s edge. Timing edits write timeline.json and update the preview without rebuilding Rust.",
    "Timing edits support undo and redo. Finish or cancel an agent task before changing the timeline.",
    timing,
  ],
  [
    "sdk",
    "03 / THE BUILD ENVIRONMENT",
    "An example in.<br>A preview out.",
    "Install the managed SDK from Setup, then open an example. Studio builds the project and prepares its worker, first frame and audio.",
    "The current preview stays available while the next revision is prepared.",
    sdk,
  ],
  [
    "export",
    "04 / THE FINISHED FILE",
    "Make the video.<br>Take it with you.",
    "Choose an export format supported by your project’s worker, set the size and frame rate, and render a video file. Follow progress or cancel the export.",
    "From editable source to a file you can share. Export capabilities depend on the project.",
    output,
  ],
];
const html = features
  .map(
    ([id, number, title, copy, detail, drawing]) =>
      `<article class="product-chapter" id="studio-${id}"><div class="feature-copy"><span class="principle-number">${number}</span><h3>${title}</h3><p>${copy}</p><p class="feature-detail">${detail}</p></div>${drawing}</article>`
  )
  .join("\n");
let page = readFileSync(file, "utf8");
if (!page.includes("<!-- studio-features -->")) {
  const start = page.indexOf('<div class="principles">');
  const end = page.indexOf("\n      </section>", start);
  page =
    page.slice(0, start) +
    '<div class="principles"><!-- studio-features --><!-- /studio-features --></div>' +
    page.slice(end);
}
page = page.replace(
  /<!-- studio-features -->[\s\S]*?<!-- \/studio-features -->/,
  `<!-- studio-features -->\n${html}\n<!-- /studio-features -->`
);
writeFileSync(file, page);
