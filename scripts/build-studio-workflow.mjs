// The landing illustration is SVG, so scroll can manipulate its individual parts.
// Regenerate the marked region only; prose and the install path stay in the page.
import { readFileSync, writeFileSync } from "node:fs";
import { mobileDiagrams } from "./studio-mobile-diagrams.mjs";
const file = new URL("../landing/studio/index.html", import.meta.url);
const plane = tone =>
  `<path class="plane-side ${tone}" d="M0 150v10h260v-10Z"/><rect class="plane ${tone}" width="260" height="150"/>`;
const svg = `<svg class="edit-diagram" viewBox="0 0 680 690" role="img" aria-labelledby="diagram-title diagram-desc">
  <title id="diagram-title">An edit, separated into its working parts</title>
  <desc id="diagram-desc">A selected title in the preview supplies context to an agent draft. The draft is built and inspected before a successful edit updates the preview. Pink marks selection, blue marks the draft, and green marks successful checks. This is an illustration, not a recording of the application.</desc>
  <defs><pattern id="diagram-grid" width="26" height="25" patternUnits="userSpaceOnUse"><path d="M26 0H0V25" fill="none" stroke="#cbd1d2" stroke-width=".45"/></pattern></defs>
  <g class="diagram-guides" fill="none" stroke="#ced3d1" stroke-width="1" stroke-dasharray="3 7"><path d="M300 48V465M170 123v418M525 178v430"/></g>
  <g class="check-plane">
    <g transform="matrix(.866 .5 -.866 .5 300 445)">
      ${plane("check-tone")}
      <rect x="13" y="13" width="234" height="124" fill="url(#diagram-grid)"/>
      <path class="gate gate-build" d="M18 113V41h100v72Z"/>
      <path class="gate gate-inspect" d="M141 113V41h100v72Z"/>
      <text x="31" y="68" class="diagram-code">cargo build</text><text x="31" y="91" class="diagram-code gate-build-label">✓ compiled</text>
      <text x="154" y="68" class="diagram-code">inspect</text><text x="154" y="91" class="diagram-code gate-inspect-label">✓ checked</text>
    </g>
    <path class="leader" d="M523 551h8"/><text x="541" y="547" class="diagram-label">03 / CHECKS</text><text x="541" y="563" class="diagram-small">Build → validate</text>
  </g>
  <path class="result-path" d="M335 495V189" fill="none"/>
  <g class="result-packet"><path d="M325 258v-65l-12 7 22-47 22 22-12 6v65Z" fill="#89e9b1" stroke="#35a46a"/><path d="m329 215 5 4 9-20" fill="none" stroke="#186c48" stroke-width="2"/></g>
  <g class="draft-plane">
    <g transform="matrix(.866 .5 -.866 .5 300 255)">
      ${plane("draft-tone")}
      <path d="M0 25h260" class="leader"/><text x="14" y="17" class="diagram-code">src/lib.rs · working copy</text>
      <text x="18" y="53" class="diagram-code">&lt;text id={title_key}&gt;</text>
      <text x="28" y="82" class="diagram-code source-before">− &quot;Hello.&quot;</text>
      <g class="source-after"><rect x="19" y="91" width="222" height="24" fill="#dbeee5"/><text x="28" y="108" class="diagram-code">+ &quot;Make your move.&quot;</text></g>
      <text x="18" y="134" class="diagram-code">&lt;/text&gt;</text>
    </g>
    <path class="leader" d="M523 362h8"/><text x="541" y="358" class="diagram-label">02 / DRAFT</text><text x="541" y="374" class="diagram-small">Agent edits Rust</text>
  </g>
  <g class="candidate-packet"><path d="M321 405v62l-12-7 22 47 22-22-12-6v-62Z" fill="#91c9fa" stroke="#5b9fdc"/></g>
  <g class="context-packet"><path d="M219 201v69l-12-7 22 47 22-22-12-6v-69Z" fill="#ffb6cc" stroke="#de7395"/></g>
  <g class="preview-plane">
    <g transform="matrix(.866 .5 -.866 .5 300 65)">
      ${plane("preview-tone")}
      <rect x="10" y="10" width="240" height="130" fill="url(#diagram-grid)"/>
      <path d="M189 23h44v43h-44Z" fill="#b6d8fb" stroke="#74a7d8"/><circle cx="204" cy="45" r="22" fill="#f9c2d2" stroke="#df8da7"/>
      <text x="23" y="34" class="diagram-small">LAUNCH FILM / INTRO</text>
      <g class="title-before"><text x="23" y="96" class="diagram-title">Hello.</text></g>
      <g class="title-after"><text x="23" y="80" class="diagram-title final-title">Make your</text><text x="23" y="112" class="diagram-title final-title">move.</text></g>
      <g class="object-selection"><path d="M17 58V48h12M121 48h12v10M133 100v10h-12M29 110H17v-10" fill="none" stroke="#d65e88" stroke-width="2"/><path d="m112 103 3 21 5-7 8 5 3-5-9-5 7-4Z" fill="white" stroke="#ad456a"/></g>
      <path d="M12 126h54m5 0h16" fill="none" stroke="#aaa" stroke-width="2"/>
    </g>
    <path class="leader" d="M523 172h8"/><text x="541" y="168" class="diagram-label">01 / PREVIEW</text><text x="541" y="184" class="diagram-small">Select the title</text>
  </g>
  <g class="prompt-note"><rect x="33" y="350" width="196" height="66" rx="3" fill="#fff" stroke="#c9cecc"/><path d="M229 371h26" class="leader"/><text x="46" y="369" class="diagram-small">YOUR REQUEST</text><text x="46" y="388" class="prompt-text">Change the title to</text><text x="46" y="404" class="prompt-text">“Make your move.”</text></g>
  <text x="300" y="677" text-anchor="middle" class="diagram-small diagram-foot">SOURCE → CANDIDATE → ACCEPTED REVISION</text>
</svg>`;
const html = readFileSync(file, "utf8").replace(
  /<!-- workflow-figure -->[\s\S]*?<!-- \/workflow-figure -->/,
  `<!-- workflow-figure -->\n${svg}\n<!-- /workflow-figure -->`
);
writeFileSync(file, mobileDiagrams(html));
