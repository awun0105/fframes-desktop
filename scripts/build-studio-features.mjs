// Homepage chapters: project ownership, timing, SDK preparation and export. Each
// figure is a scroll scene (scripts/studio-scenes.mjs); without motion it shows the
// finished state.
import { readFileSync, writeFileSync } from "node:fs";
import { mobileDiagrams } from "./studio-mobile-diagrams.mjs";
import {
  win,
  blink,
  part,
  edge,
  token,
  chip,
  ring,
  narration,
} from "./studio-scenes.mjs";
const file = "landing/studio/index.html";
const figure = (
  name,
  title,
  desc,
  body,
  caption
) => `<figure class="product-figure" data-product-figure="${name}">
<div class="product-figure-heading"><span>${title}</span><span><span class="scroll-cue">SCROLL TO FOLLOW · </span>ILLUSTRATION</span></div>
<p class="product-figure-hint">Scroll the figure sideways to see every detail ↔</p>
<div class="product-figure-scroll" tabindex="0" role="region" aria-label="${title}; scroll horizontally on small screens">
<svg viewBox="0 0 640 360" role="img" aria-labelledby="product-${name}-t product-${name}-d"><title id="product-${name}-t">${title}</title><desc id="product-${name}-d">${desc}</desc>${body}</svg></div>
<figcaption>${caption}</figcaption></figure>`;
// A part that moves by dx inside a..b and keeps the offset afterwards.
const move = (dx, a, b, inner, cls = "") =>
  `<g class="sc sc-move ${cls}" style="--a: ${a}; --b: ${b}; --dx: ${dx}px">${inner}</g>`;
const check = (x, y) => `<path d="m${x} ${y} 4 4 8-9" class="sc-check"/>`;

// Ordinary files, a workspace around them, one build, one preview.
const fileRow = (y, name, tag, tone) =>
  `<rect x="58" y="${y - 16}" width="222" height="32" rx="3" class="pf-${tone}"/><text x="72" y="${y + 5}" class="pf-code">${name}</text><text x="268" y="${y + 4}" text-anchor="end" class="pf-micro">${tag}</text>`;
const project = figure(
  "project",
  "YOUR FILES, CONNECTED",
  "Rust source, media and timing remain ordinary project files that your editor and Git use directly. Studio adds a workspace around them, builds the project and prepares a video preview.",
  `
${part("fade", 0.3, 0.38, '<rect x="22" y="40" width="596" height="262" rx="6" class="pf-workspace"/><text x="606" y="31" text-anchor="end" class="pf-micro">STUDIO WORKSPACE</text>')}
<rect x="44" y="62" width="250" height="184" rx="4" class="pf-paper"/><text x="60" y="86" class="pf-micro">MY-VIDEO/</text><path d="M44 98h250" class="pf-edge"/>
${part("rise", 0.02, 0.08, fileRow(124, "src/lib.rs", "RUST", "paper"))}
${part("rise", 0.06, 0.12, fileRow(168, "media/", "MEDIA", "blue"))}
${part("rise", 0.1, 0.16, fileRow(212, "timeline.json", "TIMING", "green"))}
${part("fade", 0.16, 0.24, '<path d="M110 314V246M202 314V246" class="pf-guide"/><rect x="58" y="314" width="104" height="24" rx="12" class="pf-paper"/><text x="110" y="330" text-anchor="middle" class="pf-micro">YOUR EDITOR</text><rect x="174" y="314" width="56" height="24" rx="12" class="pf-paper"/><text x="202" y="330" text-anchor="middle" class="pf-micro">GIT</text>')}
${edge("project-src", "M280 124H302V159H318", 0.44, 0.5)}
${edge("project-media", "M280 168H302V159H318", 0.46, 0.52)}
${edge("project-timing", "M280 212H302V159H318", 0.48, 0.54)}
${part("lit", 0.5, 0.56, `<rect x="318" y="128" width="124" height="62" rx="4" class="pf-paper"/><text x="332" y="152" class="pf-code">cargo build</text><rect x="332" y="166" width="82" height="5" rx="2.5" class="sc-bar-track"/><rect x="332" y="166" width="82" height="5" rx="2.5" class="sc sc-bar" ${win(0.58, 0.74)}/>`)}
${part("pop", 0.74, 0.78, check(422, 166))}
${edge("project-preview", "M442 159H458", 0.76, 0.82)}
${token("project-src", 0.5, 0.62, chip("blue"))}
${token("project-media", 0.52, 0.64, chip("pink"))}
${token("project-timing", 0.54, 0.66, chip("green"))}
${token("project-preview", 0.8, 0.88, chip("green"))}
${part("rise", 0.84, 0.92, '<rect x="462" y="74" width="146" height="170" rx="4" class="pf-paper"/><text x="476" y="94" class="pf-micro">STUDIO PREVIEW</text><path d="M462 104h146" class="pf-edge"/><rect x="474" y="114" width="122" height="78" class="pf-green"/><circle cx="576" cy="136" r="12" class="pf-pink"/><text x="486" y="170" class="pf-title">Hello.</text><path d="M476 222h118" class="pf-edge"/><circle cx="520" cy="222" r="3" class="pf-fill-green"/>', "pf-output")}
${ring(462, 74, 146, 170, 0.92, 0.96, 0.97, 0.99)}
${narration(
  618,
  330,
  [
    [0, 0.04, "Your video is a Rust project of ordinary files"],
    [0.16, 0.22, "Open them in your editor; keep them in Git"],
    [0.3, 0.36, "Studio adds a workspace around them"],
    [0.44, 0.5, "It builds the project from those files"],
    [0.8, 0.86, "Studio prepares a video preview"],
  ],
  "end"
)}`,
  "A project structure, not a hidden document format. The preview requires a compatible project and a successful build."
);

// Split B, move the outro to the start, trim the intro; each edit writes timeline.json.
const ticks = Array.from({ length: 13 }, (_, i) => `M${32 + 48 * i} 78v8`).join(
  ""
);
const clip = (x, w, tone, label, inset = 14) =>
  `<rect x="${x}" y="104" width="${w}" height="62" rx="2" class="pf-${tone}"/><text x="${x + inset}" y="141" class="pf-code">${label}</text>`;
const timing = figure(
  "timing",
  "CHANGE THE CUT, KEEP THE CODE",
  "A scene is split, the outro moves to the beginning and a clip edge is trimmed. Each timing change is written to timeline.json; Rust source stays unchanged.",
  `
<text x="32" y="40" class="pf-micro">TIMELINE</text><path d="M32 82H608" class="pf-guide"/><path d="${ticks}" class="pf-tick"/>
<rect x="32" y="98" width="576" height="74" class="pf-faint"/>
${edge("timing-write", "M167 172V258", 0.04, 0.12, "sc-quiet", "sc-quiet")}
${move(180, 0.4, 0.58, `${clip(32, 168, "blue", "A / INTRO", 38)}${part("fade", 0.66, 0.74, '<rect x="32" y="104" width="24" height="62" class="pf-trimmed"/>')}${part("fade", 0.62, 0.66, move(24, 0.66, 0.78, '<path d="M32 96v78" class="pf-trim-edge"/>'))}`)}
${move(180, 0.4, 0.58, `<rect x="212" y="104" width="81" height="62" rx="2" class="pf-green"/>${move(6, 0.16, 0.26, '<rect x="293" y="104" width="81" height="62" rx="2" class="pf-green"/>')}${part("hide", 0.16, 0.2, '<text x="226" y="141" class="pf-code">B / MAIN</text>')}${part("fade", 0.18, 0.24, `<text x="226" y="141" class="pf-code">B₁</text>${move(6, 0.16, 0.26, '<text x="309" y="141" class="pf-code">B₂</text>')}`)}<path d="M293 92v86" class="sc sc-blink pf-split-mark" ${blink(0.08, 0.12, 0.26, 0.3)}/>`)}
${move(-360, 0.4, 0.58, `<g class="sc sc-undip" ${win(0.32, 0.4)}><g class="sc sc-dip" ${win(0.58, 0.66)}>${clip(392, 168, "pink", "C / OUTRO")}</g></g>`, "pf-clip-c")}
${token("timing-write", 0.24, 0.32, chip("blue"))}
${token("timing-write", 0.6, 0.68, chip("blue"))}
${token("timing-write", 0.78, 0.86, chip("blue"))}
<rect x="32" y="262" width="270" height="56" rx="3" class="pf-green"/><text x="48" y="286" class="pf-code">timeline.json</text>
<text x="288" y="286" text-anchor="end" class="pf-micro sc sc-blink" ${blink(-0.01, 0, 0.3, 0.32)}>REV 1</text>
<text x="288" y="286" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.3, 0.32, 0.66, 0.68)}>REV 2</text>
<text x="288" y="286" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.66, 0.68, 0.84, 0.86)}>REV 3</text>
<text x="288" y="286" text-anchor="end" class="pf-micro sc sc-fade" ${win(0.84, 0.86)}>REV 4</text>
${part("fade", 0.3, 0.32, '<text x="48" y="306" class="pf-micro">SPLIT</text>')}
${part("fade", 0.66, 0.68, '<text x="96" y="306" class="pf-micro">· MOVE</text>')}
${part("fade", 0.84, 0.86, '<text x="150" y="306" class="pf-micro">· TRIM</text>')}
${ring(32, 262, 270, 56, 0.3, 0.32, 0.38, 0.42)}
${ring(32, 262, 270, 56, 0.66, 0.68, 0.74, 0.78)}
${ring(32, 262, 270, 56, 0.84, 0.86, 0.96, 0.99)}
<rect x="326" y="262" width="282" height="56" rx="3" class="pf-paper"/><text x="342" y="286" class="pf-code">Rust source → unchanged</text>
${part("fade", 0.88, 0.94, `<text x="342" y="306" class="pf-micro">NO REBUILD</text>${check(430, 298)}`)}
${narration(
  608,
  40,
  [
    [0.04, 0.1, "Split the main scene"],
    [0.32, 0.38, "Move the outro to the start"],
    [0.62, 0.68, "Trim the intro’s edge"],
    [0.88, 0.94, "timeline.json changes; Rust does not"],
  ],
  "end"
)}`,
  "Illustrative split → reorder → trim. Timing-only edits do not rebuild Rust. Timing changes are unavailable while an agent task is running."
);

// Install the SDK, open an example, build it, prepare worker, frame and audio, swap.
const prep = (y, label, a) =>
  `<rect x="504" y="${y - 10}" width="12" height="12" rx="2" class="pf-paper"/>${part("pop", a, a + 0.04, `<path d="m506 ${y - 4} 3 3 6-7" class="sc-check"/>`)}${part("lit", a, a + 0.04, `<text x="524" y="${y}" class="pf-micro">${label}</text>`)}`;
const sdk = figure(
  "sdk",
  "FROM SETUP TO A WORKING PREVIEW",
  "Studio installs a managed SDK. A compatible example project builds with it and prepares a preview worker, frame and audio before replacing the preview, which stays available meanwhile.",
  `
<path d="M45 291H605" class="pf-guide"/>
<path d="m48 160 112-61 112 61-112 61Z" class="pf-green"/><path d="m48 160v30l112 61 112-61v-30l-112 61Z" class="pf-paper"/><text x="160" y="176" text-anchor="middle" class="pf-code">MANAGED SDK</text>
<g class="sc sc-drop pf-tool-one" ${win(0.04, 0.14)}><rect x="58" y="47" width="90" height="35" rx="3" class="pf-blue"/><text x="76" y="70" class="pf-code">Rust</text></g>
<g class="sc sc-drop pf-tool-two" ${win(0.1, 0.2)}><rect x="164" y="47" width="102" height="35" rx="3" class="pf-pink"/><text x="179" y="70" class="pf-code">fframes</text></g>
<rect x="70" y="262" width="180" height="5" rx="2.5" class="sc-bar-track"/><rect x="70" y="262" width="180" height="5" rx="2.5" class="sc sc-bar" ${win(0.06, 0.26)}/>
${part("fade", 0.26, 0.3, `<text x="70" y="282" class="pf-micro">SDK INSTALLED</text>${check(178, 274)}`)}
${edge("sdk-open", "M284 174H330", 0.3, 0.36)}
${part("rise", 0.34, 0.42, '<rect x="340" y="100" width="124" height="150" rx="4" class="pf-paper"/><text x="354" y="120" class="pf-micro">EXAMPLE</text><path d="M340 130h124" class="pf-edge"/><path d="M356 142h40l12 12v38h-52Z" class="pf-blue"/><text x="368" y="176" class="pf-code">&lt;/&gt;</text><text x="354" y="216" class="pf-micro">cargo build</text><rect x="354" y="226" width="96" height="5" rx="2.5" class="sc-bar-track"/>', "pf-example")}
<rect x="354" y="226" width="96" height="5" rx="2.5" class="sc sc-bar" ${win(0.44, 0.62)}/>
${edge("sdk-prepare", "M464 140H494", 0.62, 0.66)}
${prep(124, "WORKER", 0.66)}
${prep(146, "FIRST FRAME", 0.72)}
${prep(168, "AUDIO", 0.78)}
<g class="pf-ready"><rect x="504" y="184" width="104" height="86" rx="4" class="pf-paper"/><text x="514" y="201" class="pf-micro">PREVIEW</text><rect x="512" y="210" width="88" height="50" class="pf-faint"/>
${part("hide", 0.86, 0.9, '<text x="556" y="239" text-anchor="middle" class="pf-micro">CURRENT</text>')}
${part("fade", 0.86, 0.92, '<rect x="512" y="210" width="88" height="50" class="pf-green"/><path d="m548 224 16 11-16 11Z" class="pf-fill-ink"/>')}</g>
${ring(504, 184, 104, 86, 0.9, 0.94, 0.97, 0.99)}
${narration(48, 330, [
  [0.02, 0.08, "Install the managed SDK from Setup"],
  [0.3, 0.36, "Open a compatible example"],
  [0.42, 0.48, "Studio builds it with the SDK"],
  [0.64, 0.7, "and prepares its worker, first frame and audio"],
  [0.86, 0.92, "The new preview replaces the current one"],
])}`,
  "A preparation sequence, not elapsed build time. Failed or cancelled builds keep the previous preview."
);

// The worker states what it can export; render with progress, cancel, save the file.
const frame = (x, cx, n, a) =>
  part(
    "rise",
    a,
    a + 0.06,
    `<rect x="${x}" y="56" width="80" height="100" rx="3" class="pf-paper"/><rect x="${x + 8}" y="64" width="64" height="56" class="pf-blue"/><circle cx="${cx}" cy="92" r="11" class="pf-pink"/><text x="${x + 8}" y="142" class="pf-micro">${n}</text>`
  );
const field = (y, label, value, a) =>
  `<text x="46" y="${y}" class="pf-micro">${label}</text><rect x="132" y="${y - 13}" width="108" height="18" rx="2" class="pf-faint"/>${part("fade", a, a + 0.04, `<text x="137" y="${y}" class="pf-micro">${value}</text>`)}`;
const output = figure(
  "export",
  "TURN THE PROJECT INTO A VIDEO",
  "The project's preview worker exposes its supported export formats. Studio renders the chosen output with progress and cancellation, then saves the video file.",
  `<g transform="translate(0 26)">
<rect x="32" y="36" width="200" height="52" rx="4" class="pf-green"/><text x="46" y="57" class="pf-micro">PREVIEW WORKER</text><text x="46" y="75" class="pf-micro">supported formats</text>
${edge("export-offer", "M132 88V118", 0.02, 0.08)}
${token("export-offer", 0.06, 0.16, chip("green"))}
<rect x="32" y="118" width="214" height="150" rx="4" class="pf-paper"/><text x="46" y="138" class="pf-micro">EXPORT</text><path d="M32 148h214" class="pf-edge"/>
${field(170, "FORMAT", "from worker ▾", 0.16)}
${field(196, "SIZE", "1920×1080", 0.22)}
${field(222, "FRAME RATE", "30 fps", 0.28)}
<g class="sc-button"><rect x="46" y="236" width="78" height="22" rx="3"/><text x="85" y="251" text-anchor="middle">RENDER</text></g>
<rect x="134" y="236" width="84" height="22" rx="3" class="pf-paper"/><text x="176" y="251" text-anchor="middle" class="pf-micro">CANCEL</text>
${ring(46, 236, 78, 22, 0.36, 0.4, 0.46, 0.5, 4)}
<rect x="134" y="236" width="84" height="22" rx="3" class="sc sc-blink pf-active" ${blink(0.42, 0.46, 0.84, 0.88)}/>
${frame(262, 286, "001", 0.46)}
${frame(350, 386, "045", 0.58)}
${frame(438, 488, "090", 0.7)}
${edge("export-save", "M524 106H544", 0.8, 0.84)}
${part("pop", 0.84, 0.9, '<path d="M548 56h40l20 20v80h-60Z" class="pf-green"/><path d="M588 56v20h20" class="pf-edge"/><path d="m570 94 18 11-18 11Z" class="pf-fill-green"/><text x="578" y="144" text-anchor="middle" class="pf-micro">VIDEO</text>', "pf-video-file")}
${ring(548, 56, 60, 100, 0.9, 0.94, 0.97, 0.99)}
${part("fade", 0.42, 0.46, '<text x="262" y="196" class="pf-micro">RENDERING</text>')}
<text x="608" y="196" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.44, 0.46, 0.56, 0.58)}>FRAME 001 / 090</text>
<text x="608" y="196" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.58, 0.6, 0.68, 0.7)}>FRAME 045 / 090</text>
<text x="608" y="196" text-anchor="end" class="pf-micro sc sc-fade" ${win(0.7, 0.72)}>FRAME 090 / 090</text>
<rect x="262" y="206" width="346" height="8" rx="4" class="pf-faint"/><rect x="262" y="206" width="346" height="8" rx="4" class="sc pf-render-bar" ${win(0.44, 0.84)}/>
${part("fade", 0.9, 0.96, `<text x="608" y="236" text-anchor="end" class="pf-micro">SAVED</text>${check(552, 228)}`)}
${narration(262, 262, [
  [0.02, 0.08, "The worker reports what it can export"],
  [0.16, 0.22, "Choose format, size and frame rate"],
  [0.38, 0.44, "Render: follow progress, or cancel"],
  [0.84, 0.9, "The video file is saved"],
])}</g>`,
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
writeFileSync(file, mobileDiagrams(page));
