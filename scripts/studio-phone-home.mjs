// Phone compositions of the home page figures: portrait SVGs, 340 units wide, that
// tell the same scroll story as the wide figures (the same --p windows and hero
// variables). Every id ends in -m so tokens follow these paths, not the wide ones.
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

const move = (dx, a, b, inner, cls = "") =>
  `<g class="sc sc-move ${cls}" style="--a: ${a}; --b: ${b}; --dx: ${dx}px">${inner}</g>`;
const check = (x, y) => `<path d="m${x} ${y} 4 4 8-9" class="sc-check"/>`;
const say = (y, steps) => narration(12, y, steps, "start", 42);

// A panel with a thin side below it, like the planes of the wide figure.
const panel = (x, y, w, h, tone) =>
  `<path class="plane-side ${tone}" d="M${x} ${y + h}v6h${w}v-6Z"/><rect class="plane ${tone}" x="${x}" y="${y}" width="${w}" height="${h}"/>`;
// A chip home.js moves from (x0, y0) to (x1, y1) with --context-k and friends.
const flowChip = (kind, label, width, [x0, y0], [x1, y1]) =>
  `<g class="phone-chip chip-${kind}" style="--x0: ${x0}px; --y0: ${y0}px; --x1: ${x1}px; --y1: ${y1}px"><rect x="${-width / 2}" y="-12" width="${width}" height="24" rx="12"/><text y="4" text-anchor="middle">${label}</text></g>`;

export const workflow = {
  height: 346,
  body: `<defs><pattern id="diagram-grid-m" width="26" height="25" patternUnits="userSpaceOnUse"><path d="M26 0H0V25" fill="none" stroke="#cbd1d2" stroke-width=".45"/></pattern></defs>
<g>${panel(2, 258, 300, 80, "check-tone")}
<text x="14" y="276" class="diagram-label">03 / CHECKS</text><text x="290" y="276" text-anchor="end" class="diagram-small">Build → validate</text>
<path class="gate gate-build" d="M12 284h136v46H12Z"/><path class="gate gate-inspect" d="M156 284h136v46H156Z"/>
<text x="22" y="300" class="diagram-code">cargo build</text><text x="166" y="300" class="diagram-code">inspect</text>
<rect x="22" y="307" width="116" height="4" rx="2" class="gate-track"/><rect x="22" y="307" width="116" height="4" rx="2" class="gate-run gate-build-run"/>
<rect x="166" y="307" width="116" height="4" rx="2" class="gate-track"/><rect x="166" y="307" width="116" height="4" rx="2" class="gate-run gate-inspect-run"/>
<text x="22" y="325" class="diagram-code gate-build-label">✓ compiled</text><text x="166" y="325" class="diagram-code gate-inspect-label">✓ checked</text></g>
<path class="result-path" d="M232 292L240 108" fill="none"/>
<g>${panel(18, 150, 300, 92, "draft-tone")}
<text x="30" y="168" class="diagram-code">src/lib.rs · working copy</text><text x="306" y="168" text-anchor="end" class="diagram-label">02 / DRAFT</text><path d="M18 176h300" class="leader"/>
<rect x="24" y="182" width="176" height="18" rx="2" class="key-mark"/><text x="30" y="195" class="diagram-code">&lt;text id={title_key}&gt;</text>
<text x="40" y="214" class="diagram-code source-before">− &quot;Hello.&quot;</text><path d="M38 210h66" class="source-strike"/>
<g class="source-after"><rect x="30" y="220" width="276" height="18" fill="#dbeee5" class="insert-bar"/><text x="40" y="233" class="diagram-code">+ &quot;Make your move.&quot;</text></g></g>
<g>${panel(34, 40, 300, 94, "preview-tone")}
<rect x="40" y="46" width="288" height="82" fill="url(#diagram-grid-m)"/>
<text x="46" y="62" class="diagram-small">LAUNCH FILM / INTRO</text><text x="322" y="62" text-anchor="end" class="diagram-label">01 / PREVIEW</text>
<path d="M292 90h30v30h-30Z" fill="#b6d8fb" stroke="#74a7d8"/><circle cx="302" cy="101" r="12" fill="#f9c2d2" stroke="#df8da7"/>
<g class="title-before"><text x="48" y="112" class="diagram-title">Hello.</text></g>
<g class="title-after"><text x="48" y="112" class="diagram-title final-title">Make your move.</text></g>
<path class="accepted-mark" d="M40 88V80h10M262 80h10v8M272 112v8h-10M50 120H40v-8"/>
<g class="object-selection"><path d="M40 88V80h10M132 80h10v8M142 112v8h-10M50 120H40v-8" fill="none" stroke="#d65e88" stroke-width="2"/><path d="m124 108 3 21 5-7 8 5 3-5-9-5 7-4Z" fill="white" stroke="#ad456a"/></g>
<circle cx="125" cy="109" r="10" class="click-ring"/></g>
<g class="prompt-note"><rect x="0" y="2" width="300" height="28" rx="3" fill="#fff" stroke="#c9cecc"/><text x="10" y="20" class="diagram-small">REQUEST</text><text x="72" y="21" class="prompt-text">Change the title to “Make your move.”</text></g>
${flowChip("context", "title_key", 92, [96, 126], [130, 191])}
${flowChip("candidate", "candidate", 92, [210, 229], [150, 297])}
${flowChip("result", "accepted", 84, [232, 292], [240, 108])}`,
};

// Ordinary files, a workspace around them, one build, one preview.
const fileRow = (y, name, tag, tone) =>
  `<rect x="32" y="${y - 15}" width="186" height="30" rx="3" class="pf-${tone}"/><text x="44" y="${y + 5}" class="pf-code">${name}</text><text x="208" y="${y + 4}" text-anchor="end" class="pf-micro">${tag}</text>`;
export const project = {
  height: 468,
  body: `
${part("fade", 0.3, 0.38, '<rect x="6" y="28" width="328" height="380" rx="6" class="pf-workspace"/><text x="328" y="18" text-anchor="end" class="pf-micro">STUDIO WORKSPACE</text>')}
<rect x="20" y="42" width="210" height="146" rx="4" class="pf-paper"/><text x="34" y="64" class="pf-micro">MY-VIDEO/</text><path d="M20 76h210" class="pf-edge"/>
${part("rise", 0.02, 0.08, fileRow(100, "src/lib.rs", "RUST", "paper"))}
${part("rise", 0.06, 0.12, fileRow(136, "media/", "MEDIA", "blue"))}
${part("rise", 0.1, 0.16, fileRow(172, "timeline.json", "TIMING", "green"))}
${part("fade", 0.16, 0.24, '<path d="M230 92H242M230 140H242" class="pf-guide"/><rect x="242" y="80" width="84" height="24" rx="12" class="pf-paper"/><text x="284" y="96" text-anchor="middle" class="pf-micro">EDITOR</text><rect x="242" y="128" width="84" height="24" rx="12" class="pf-paper"/><text x="284" y="144" text-anchor="middle" class="pf-micro">GIT</text>')}
${edge("project-src-m", "M80 188V218", 0.44, 0.5)}
${edge("project-media-m", "M125 188V218", 0.46, 0.52)}
${edge("project-timing-m", "M170 188V218", 0.48, 0.54)}
${part("lit", 0.5, 0.56, `<rect x="40" y="218" width="170" height="48" rx="4" class="pf-paper"/><text x="54" y="238" class="pf-code">cargo build</text><rect x="54" y="250" width="124" height="5" rx="2.5" class="sc-bar-track"/><rect x="54" y="250" width="124" height="5" rx="2.5" class="sc sc-bar" ${win(0.58, 0.74)}/>`)}
${part("pop", 0.74, 0.78, check(186, 250))}
${edge("project-preview-m", "M125 266V286", 0.76, 0.82)}
${token("project-src-m", 0.5, 0.62, chip("blue"))}
${token("project-media-m", 0.52, 0.64, chip("pink"))}
${token("project-timing-m", 0.54, 0.66, chip("green"))}
${token("project-preview-m", 0.8, 0.88, chip("green"))}
${part("rise", 0.84, 0.92, '<rect x="40" y="286" width="260" height="112" rx="4" class="pf-paper"/><text x="54" y="304" class="pf-micro">STUDIO PREVIEW</text><path d="M40 312h260" class="pf-edge"/><rect x="52" y="320" width="236" height="52" class="pf-green"/><circle cx="262" cy="338" r="12" class="pf-pink"/><text x="66" y="356" class="pf-title">Hello.</text><path d="M54 386h232" class="pf-edge"/><circle cx="120" cy="386" r="3" class="pf-fill-green"/>')}
${ring(40, 286, 260, 112, 0.92, 0.96, 0.97, 0.99)}
${say(434, [
  [0, 0.04, "Your video is a Rust project of ordinary files"],
  [0.16, 0.22, "Open them in your editor; keep them in Git"],
  [0.3, 0.36, "Studio adds a workspace around them"],
  [0.44, 0.5, "It builds the project from those files"],
  [0.8, 0.86, "Studio prepares a video preview"],
])}`,
};

// Split B, move the outro to the start, trim the intro; each edit writes timeline.json.
const ticks = Array.from(
  { length: 13 },
  (_, i) => `M${(12 + 26.33 * i).toFixed(1)} 62v8`
).join("");
const clip = (x, w, tone, label, inset = 8) =>
  `<rect x="${x}" y="82" width="${w}" height="56" rx="2" class="pf-${tone}"/><text x="${x + inset}" y="115" class="pf-code">${label}</text>`;
export const timing = {
  height: 352,
  body: `<g transform="translate(0 -20)">
<text x="12" y="44" class="pf-micro">TIMELINE</text><path d="M12 66H328" class="pf-guide"/><path d="${ticks}" class="pf-tick"/>
<rect x="12" y="76" width="316" height="68" class="pf-faint"/>
${edge("timing-write-m", "M66 144V198", 0.04, 0.12, "sc-quiet", "sc-quiet")}
${move(99, 0.4, 0.58, `${clip(12, 92, "blue", "A / INTRO", 18)}${part("fade", 0.66, 0.74, '<rect x="12" y="82" width="13" height="56" class="pf-trimmed"/>')}${part("fade", 0.62, 0.66, move(13, 0.66, 0.78, '<path d="M12 74v72" class="pf-trim-edge"/>'))}`)}
${move(99, 0.4, 0.58, `<rect x="111" y="82" width="45" height="56" rx="2" class="pf-green"/>${move(4, 0.16, 0.26, '<rect x="156" y="82" width="45" height="56" rx="2" class="pf-green"/>')}${part("hide", 0.16, 0.2, '<text x="119" y="115" class="pf-code">B / MAIN</text>')}${part("fade", 0.18, 0.24, `<text x="121" y="115" class="pf-code">B₁</text>${move(4, 0.16, 0.26, '<text x="166" y="115" class="pf-code">B₂</text>')}`)}<path d="M156 72v76" class="sc sc-blink pf-split-mark" ${blink(0.08, 0.12, 0.26, 0.3)}/>`)}
${move(-198, 0.4, 0.58, `<g class="sc sc-undip" ${win(0.32, 0.4)}><g class="sc sc-dip" ${win(0.58, 0.66)}>${clip(210, 92, "pink", "C / OUTRO")}</g></g>`)}
${token("timing-write-m", 0.24, 0.32, chip("blue"))}
${token("timing-write-m", 0.6, 0.68, chip("blue"))}
${token("timing-write-m", 0.78, 0.86, chip("blue"))}
<rect x="12" y="198" width="316" height="54" rx="3" class="pf-green"/><text x="26" y="221" class="pf-code">timeline.json</text>
<text x="314" y="221" text-anchor="end" class="pf-micro sc sc-blink" ${blink(-0.01, 0, 0.3, 0.32)}>REV 1</text>
<text x="314" y="221" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.3, 0.32, 0.66, 0.68)}>REV 2</text>
<text x="314" y="221" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.66, 0.68, 0.84, 0.86)}>REV 3</text>
<text x="314" y="221" text-anchor="end" class="pf-micro sc sc-fade" ${win(0.84, 0.86)}>REV 4</text>
${part("fade", 0.3, 0.32, '<text x="26" y="241" class="pf-micro">SPLIT</text>')}
${part("fade", 0.66, 0.68, '<text x="70" y="241" class="pf-micro">· MOVE</text>')}
${part("fade", 0.84, 0.86, '<text x="122" y="241" class="pf-micro">· TRIM</text>')}
${ring(12, 198, 316, 54, 0.3, 0.32, 0.38, 0.42)}
${ring(12, 198, 316, 54, 0.66, 0.68, 0.74, 0.78)}
${ring(12, 198, 316, 54, 0.84, 0.86, 0.96, 0.99)}
<rect x="12" y="266" width="316" height="54" rx="3" class="pf-paper"/><text x="26" y="289" class="pf-code">Rust source → unchanged</text>
${part("fade", 0.88, 0.94, `<text x="26" y="309" class="pf-micro">NO REBUILD</text>${check(98, 301)}`)}
${say(346, [
  [0.04, 0.1, "Split the main scene"],
  [0.32, 0.38, "Move the outro to the start"],
  [0.62, 0.68, "Trim the intro’s edge"],
  [0.88, 0.94, "timeline.json changes; Rust does not"],
])}</g>`,
};

// Install the SDK, open an example, build it, prepare worker, frame and audio, swap.
const prep = (y, label, a) =>
  `<rect x="186" y="${y - 10}" width="12" height="12" rx="2" class="pf-paper"/>${part("pop", a, a + 0.04, `<path d="m188 ${y - 4} 3 3 6-7" class="sc-check"/>`)}${part("lit", a, a + 0.04, `<text x="206" y="${y}" class="pf-micro">${label}</text>`)}`;
export const sdk = {
  height: 452,
  body: `
<path d="m70 104 100-54 100 54-100 54Z" class="pf-green"/><path d="m70 104v26l100 54 100-54v-26l-100 54Z" class="pf-paper"/><text x="170" y="120" text-anchor="middle" class="pf-code">MANAGED SDK</text>
<g class="sc sc-drop" ${win(0.04, 0.14)}><rect x="78" y="10" width="84" height="34" rx="3" class="pf-blue"/><text x="96" y="32" class="pf-code">Rust</text></g>
<g class="sc sc-drop" ${win(0.1, 0.2)}><rect x="176" y="10" width="96" height="34" rx="3" class="pf-pink"/><text x="190" y="32" class="pf-code">fframes</text></g>
<rect x="90" y="200" width="160" height="5" rx="2.5" class="sc-bar-track"/><rect x="90" y="200" width="160" height="5" rx="2.5" class="sc sc-bar" ${win(0.06, 0.26)}/>
${part("fade", 0.26, 0.3, `<text x="90" y="222" class="pf-micro">SDK INSTALLED</text>${check(186, 214)}`)}
${edge("sdk-open-m", "M90 230V250", 0.3, 0.36)}
${part("rise", 0.34, 0.42, '<rect x="20" y="252" width="150" height="132" rx="4" class="pf-paper"/><text x="34" y="272" class="pf-micro">EXAMPLE</text><path d="M20 282h150" class="pf-edge"/><path d="M34 292h40l12 12v34h-52Z" class="pf-blue"/><text x="46" y="324" class="pf-code">&lt;/&gt;</text><text x="34" y="360" class="pf-micro">cargo build</text><rect x="34" y="370" width="122" height="5" rx="2.5" class="sc-bar-track"/>')}
<rect x="34" y="370" width="122" height="5" rx="2.5" class="sc sc-bar" ${win(0.44, 0.62)}/>
${edge("sdk-prepare-m", "M170 270H180", 0.62, 0.66)}
${prep(268, "WORKER", 0.66)}
${prep(290, "FIRST FRAME", 0.72)}
${prep(312, "AUDIO", 0.78)}
<g><rect x="186" y="326" width="134" height="58" rx="4" class="pf-paper"/><text x="196" y="342" class="pf-micro">PREVIEW</text><rect x="196" y="348" width="114" height="28" class="pf-faint"/>
${part("hide", 0.86, 0.9, '<text x="253" y="367" text-anchor="middle" class="pf-micro">CURRENT</text>')}
${part("fade", 0.86, 0.92, '<rect x="196" y="348" width="114" height="28" class="pf-green"/><path d="m246 354 13 8-13 8Z" class="pf-fill-ink"/>')}</g>
${ring(186, 326, 134, 58, 0.9, 0.94, 0.97, 0.99)}
${say(412, [
  [0.02, 0.08, "Install the managed SDK from Setup"],
  [0.3, 0.36, "Open a compatible example"],
  [0.42, 0.48, "Studio builds it with the SDK"],
  [0.64, 0.7, "and prepares its worker, first frame and audio"],
  [0.86, 0.92, "The new preview replaces the current one"],
])}`,
};

// The worker states what it can export; render with progress, cancel, save the file.
const frame = (x, n, a) =>
  part(
    "rise",
    a,
    a + 0.06,
    `<rect x="${x}" y="254" width="72" height="92" rx="3" class="pf-paper"/><rect x="${x + 7}" y="261" width="58" height="50" class="pf-blue"/><circle cx="${x + 20 + (n === "045" ? 14 : n === "090" ? 32 : 0)}" cy="286" r="10" class="pf-pink"/><text x="${x + 7}" y="334" class="pf-micro">${n}</text>`
  );
const field = (y, label, value, a) =>
  `<text x="24" y="${y}" class="pf-micro">${label}</text><rect x="110" y="${y - 14}" width="130" height="20" rx="2" class="pf-faint"/>${part("fade", a, a + 0.04, `<text x="116" y="${y}" class="pf-micro">${value}</text>`)}`;
export const exporting = {
  height: 476,
  body: `
<rect x="12" y="4" width="200" height="50" rx="4" class="pf-green"/><text x="24" y="25" class="pf-micro">PREVIEW WORKER</text><text x="24" y="43" class="pf-micro">supported formats</text>
${edge("export-offer-m", "M112 54V82", 0.02, 0.08)}
${token("export-offer-m", 0.06, 0.16, chip("green"))}
<rect x="12" y="82" width="240" height="158" rx="4" class="pf-paper"/><text x="24" y="102" class="pf-micro">EXPORT</text><path d="M12 112h240" class="pf-edge"/>
${field(136, "FORMAT", "from worker ▾", 0.16)}
${field(164, "SIZE", "1920×1080", 0.22)}
${field(192, "FRAME RATE", "30 fps", 0.28)}
<g class="sc-button"><rect x="24" y="208" width="84" height="22" rx="3"/><text x="66" y="223" text-anchor="middle">RENDER</text></g>
<rect x="118" y="208" width="84" height="22" rx="3" class="pf-paper"/><text x="160" y="223" text-anchor="middle" class="pf-micro">CANCEL</text>
${ring(24, 208, 84, 22, 0.36, 0.4, 0.46, 0.5, 4)}
<rect x="118" y="208" width="84" height="22" rx="3" class="sc sc-blink pf-active" ${blink(0.42, 0.46, 0.84, 0.88)}/>
${frame(12, "001", 0.46)}
${frame(90, "045", 0.58)}
${frame(168, "090", 0.7)}
${edge("export-save-m", "M244 300H262", 0.8, 0.84)}
${part("pop", 0.84, 0.9, '<path d="M266 254h40l20 20v72h-60Z" class="pf-green"/><path d="M306 254v20h20" class="pf-edge"/><path d="m286 290 18 11-18 11Z" class="pf-fill-green"/><text x="296" y="336" text-anchor="middle" class="pf-micro">VIDEO</text>')}
${ring(266, 254, 60, 92, 0.9, 0.94, 0.97, 0.99)}
${part("fade", 0.42, 0.46, '<text x="12" y="374" class="pf-micro">RENDERING</text>')}
<text x="326" y="374" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.44, 0.46, 0.56, 0.58)}>FRAME 001 / 090</text>
<text x="326" y="374" text-anchor="end" class="pf-micro sc sc-blink" ${blink(0.58, 0.6, 0.68, 0.7)}>FRAME 045 / 090</text>
<text x="326" y="374" text-anchor="end" class="pf-micro sc sc-fade" ${win(0.7, 0.72)}>FRAME 090 / 090</text>
<rect x="12" y="384" width="314" height="8" rx="4" class="pf-faint"/><rect x="12" y="384" width="314" height="8" rx="4" class="sc pf-render-bar" ${win(0.44, 0.84)}/>
${part("fade", 0.9, 0.96, `<text x="326" y="414" text-anchor="end" class="pf-micro">SAVED</text>${check(270, 406)}`)}
${say(440, [
  [0.02, 0.08, "The worker reports what it can export"],
  [0.16, 0.22, "Choose format, size and frame rate"],
  [0.38, 0.44, "Render: follow progress, or cancel"],
  [0.84, 0.9, "The video file is saved"],
])}`,
};
