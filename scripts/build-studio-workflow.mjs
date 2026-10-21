// The landing illustration is SVG, so scroll can manipulate its individual parts.
// Regenerate the marked region only; prose and the install path stay in the page.
import { readFileSync, writeFileSync } from "node:fs";
import { mobileDiagrams } from "./studio-mobile-diagrams.mjs";
const file = new URL("../landing/studio/index.html", import.meta.url);
const plane = tone =>
  `<path class="plane-side ${tone}" d="M0 150v10h260v-10Z"/><rect class="plane ${tone}" width="260" height="150"/>`;
// The planes step down and to the left, so the stack fills a wide column and no
// plane covers the one below it.
const origin = {
  preview: [272, 34],
  draft: [206, 154],
  check: [140, 274],
};
const r = n => Math.round(n * 10) / 10;
// A point (u, v) on a plane's 260×150 surface, in SVG coordinates.
const at = (name, u, v) => {
  const [x, y] = origin[name];
  return [r(x + 0.866 * (u - v)), r(y + 0.5 * (u + v))];
};
const matrix = name => `matrix(.866 .5 -.866 .5 ${origin[name].join(" ")})`;
const corners = name => ({
  top: at(name, 0, 0),
  left: at(name, 0, 150),
  right: at(name, 260, 0),
});
const guide = key =>
  `M${corners("preview")[key].join(" ")}L${corners("check")[key].join(" ")}`;
// Step label beside a plane's right corner.
const label = (name, title, detail) => {
  const [x, y] = corners(name).right;
  return `<path class="leader" d="M${x + 10} ${y}h8"/><text x="${x + 24}" y="${y - 4}" class="diagram-label">${title}</text><text x="${x + 24}" y="${y + 12}" class="diagram-small">${detail}</text>`;
};
// A labelled chip that home.js moves between two planes (drawn around 0,0).
const flowChip = (kind, label, width, from, to) =>
  `<g class="flow-chip chip-${kind}" data-from="${from.join(" ")}" data-to="${to.join(" ")}"><rect x="${-width / 2}" y="-12" width="${width}" height="24" rx="12"/><text y="4" text-anchor="middle">${label}</text></g>`;
const accepted = [at("check", 190, 120), at("preview", 90, 40)];
const svg = `<svg class="edit-diagram" viewBox="0 0 660 532" role="img" aria-labelledby="diagram-title diagram-desc">
  <title id="diagram-title">An edit, separated into its working parts</title>
  <desc id="diagram-desc">A selected title in the preview supplies context to an agent draft. The draft is built and inspected before a successful edit updates the preview. Pink marks selection, blue marks the draft, and green marks successful checks. This is an illustration, not a recording of the application.</desc>
  <defs><pattern id="diagram-grid" width="26" height="25" patternUnits="userSpaceOnUse"><path d="M26 0H0V25" fill="none" stroke="#cbd1d2" stroke-width=".45"/></pattern></defs>
  <g class="diagram-guides" fill="none" stroke="#ced3d1" stroke-width="1" stroke-dasharray="3 7"><path d="${guide("top")}${guide("left")}${guide("right")}"/></g>
  <g class="check-plane">
    <g transform="${matrix("check")}">
      ${plane("check-tone")}
      <rect x="13" y="13" width="234" height="124" fill="url(#diagram-grid)"/>
      <path class="gate gate-build" d="M18 113V41h100v72Z"/>
      <path class="gate gate-inspect" d="M141 113V41h100v72Z"/>
      <text x="31" y="68" class="diagram-code">cargo build</text><text x="31" y="91" class="diagram-code gate-build-label">✓ compiled</text>
      <text x="154" y="68" class="diagram-code">inspect</text><text x="154" y="91" class="diagram-code gate-inspect-label">✓ checked</text>
      <rect x="31" y="99" width="74" height="4" rx="2" class="gate-track"/><rect x="31" y="99" width="74" height="4" rx="2" class="gate-run gate-build-run"/>
      <rect x="154" y="99" width="74" height="4" rx="2" class="gate-track"/><rect x="154" y="99" width="74" height="4" rx="2" class="gate-run gate-inspect-run"/>
    </g>
    ${label("check", "03 / CHECKS", "Build → validate")}
  </g>
  <path class="result-path" d="M${accepted[0].join(" ")}L${accepted[1].join(" ")}" fill="none"/>
  <g class="draft-plane">
    <g transform="${matrix("draft")}">
      ${plane("draft-tone")}
      <path d="M0 25h260" class="leader"/><text x="14" y="17" class="diagram-code">src/lib.rs · working copy</text>
      <rect x="12" y="41" width="150" height="17" rx="2" class="key-mark"/>
      <text x="18" y="53" class="diagram-code">&lt;text id={title_key}&gt;</text>
      <text x="28" y="82" class="diagram-code source-before">− &quot;Hello.&quot;</text><path d="M26 79h62" class="source-strike"/>
      <g class="source-after"><rect x="19" y="91" width="222" height="24" fill="#dbeee5" class="insert-bar"/><text x="28" y="108" class="diagram-code">+ &quot;Make your move.&quot;</text></g>
      <text x="18" y="134" class="diagram-code">&lt;/text&gt;</text>
    </g>
    ${label("draft", "02 / DRAFT", "Agent edits Rust")}
  </g>
  <g class="preview-plane">
    <g transform="${matrix("preview")}">
      ${plane("preview-tone")}
      <rect x="10" y="10" width="240" height="130" fill="url(#diagram-grid)"/>
      <path d="M189 23h44v43h-44Z" fill="#b6d8fb" stroke="#74a7d8"/><circle cx="204" cy="45" r="22" fill="#f9c2d2" stroke="#df8da7"/>
      <text x="23" y="34" class="diagram-small">LAUNCH FILM / INTRO</text>
      <g class="title-before"><text x="23" y="96" class="diagram-title">Hello.</text></g>
      <g class="title-after"><text x="23" y="80" class="diagram-title final-title">Make your</text><text x="23" y="112" class="diagram-title final-title">move.</text></g>
      <path class="accepted-mark" d="M17 58V48h12M151 48h12v10M163 110v10h-12M29 120H17v-10"/>
      <g class="object-selection"><path d="M17 58V48h12M121 48h12v10M133 100v10h-12M29 110H17v-10" fill="none" stroke="#d65e88" stroke-width="2"/><path d="m112 103 3 21 5-7 8 5 3-5-9-5 7-4Z" fill="white" stroke="#ad456a"/></g>
      <circle cx="113" cy="104" r="10" class="click-ring"/>
      <path d="M12 126h54m5 0h16" fill="none" stroke="#aaa" stroke-width="2"/>
    </g>
    ${label("preview", "01 / PREVIEW", "Select the title")}
  </g>
  <g class="prompt-note"><rect x="4" y="30" width="160" height="62" rx="3" fill="#fff" stroke="#c9cecc"/><path d="M110 92L${at("draft", 0, 70).join(" ")}" class="leader"/><text x="17" y="49" class="diagram-small">YOUR REQUEST</text><text x="17" y="68" class="prompt-text">Change the title to</text><text x="17" y="84" class="prompt-text">“Make your move.”</text></g>
  ${flowChip("context", "title_key", 92, at("preview", 70, 125), at("draft", 90, 50))}
  ${flowChip("candidate", "candidate", 92, at("draft", 150, 135), at("check", 80, 75))}
  ${flowChip("result", "accepted", 84, ...accepted)}
  <text x="656" y="522" text-anchor="end" class="diagram-small diagram-foot">SOURCE → CANDIDATE → ACCEPTED REVISION</text>
</svg>`;
const html = readFileSync(file, "utf8").replace(
  /<!-- workflow-figure -->[\s\S]*?<!-- \/workflow-figure -->/,
  `<!-- workflow-figure -->\n${svg}\n<!-- /workflow-figure -->`
);
writeFileSync(file, mobileDiagrams(html));
