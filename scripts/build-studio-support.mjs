// Supplementary scroll diagrams. Static geometry stays readable without JavaScript.
import { readFileSync, writeFileSync } from "node:fs";
import { mobileDiagrams } from "./studio-mobile-diagrams.mjs";
const file = "landing/studio/how-it-works.html";
const figure = (
  id,
  label,
  title,
  description,
  body,
  caption,
  width = 900,
  height = 350
) => `
<figure class="technical-figure support-figure support-${id}" data-scroll-figure>
  <div class="figure-heading"><span class="eyebrow">${label}</span><span class="illustration-label">WORKFLOW ILLUSTRATION</span></div>
  <p class="diagram-hint">Scroll the diagram sideways <span aria-hidden="true">↔</span></p>
  <div class="support-scroll" tabindex="0" role="region" aria-label="${title}; scroll horizontally on small screens">
    <svg viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="${id}-title ${id}-desc">
    <title id="${id}-title">${title}</title><desc id="${id}-desc">${description}</desc><defs><marker id="support-${id}-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="m1 1 8 4-8 4" class="support-arrow"/></marker></defs>${body.replaceAll('class="support-connector"', `class="support-connector" marker-end="url(#support-${id}-arrow)"`).replaceAll('class="support-return"', `class="support-return" marker-end="url(#support-${id}-arrow)"`)}</svg>
  </div><figcaption>${caption}</figcaption>
</figure>`;
const context = figure(
  "context",
  "THE SELECTION BECOMES CONTEXT",
  "A selected title supplies project context",
  "A selected title supplies a stable editor key. A registered source anchor can add source context. It is optional and supplied by the project.",
  `
<g class="support-part support-a" data-support-node="selected-frame"><rect x="30" y="65" width="250" height="210" rx="4" class="support-paper"/><path d="M30 99H280" class="support-line"/><text x="48" y="88" class="support-small">01 / SELECTED FRAME</text><text x="78" y="183" class="support-headline">Hello.</text><path d="M66 133v-12h15m127 12v-12h-15M66 197v12h15m127-12v12h-15" class="support-selection"/><text x="48" y="252" class="support-small">Selected object · headline</text></g>
<path d="M292 167H333" class="support-connector" pathLength="1"/><g class="support-packet"><rect x="296" y="161" width="12" height="12" rx="2" class="support-pink"/></g>
<g class="support-part support-b" data-support-node="object-identity"><rect x="345" y="65" width="250" height="210" rx="4" class="support-blue"/><text x="365" y="95" class="support-small">02 / OBJECT IDENTITY</text><text x="365" y="143" class="support-code">intro / starter-title</text><text x="365" y="171" class="support-code">headline / primary</text><path d="M365 204H575" class="support-line"/><text x="365" y="230" class="support-small">From this frame’s</text><text x="365" y="250" class="support-small">object map</text></g>
<path d="M607 185H648" class="support-connector" data-optional-link/>
<g class="support-part support-c" data-support-node="optional-anchor"><rect x="660" y="110" width="210" height="165" rx="4" class="support-paper support-optional"/><text x="680" y="141" class="support-small">03 / SOURCE ANCHOR</text><text x="680" y="180" class="support-code">src/lib.rs</text><text x="680" y="207" class="support-code">render_frame</text><text x="680" y="250" class="support-small">Only if registered</text></g>
<text x="30" y="322" class="support-small">SELECTION PROVIDES CONTEXT. IT DOES NOT EDIT SOURCE.</text>`,
  "Frame → stable object identity → optional source anchor. Without a registered anchor, Studio does not infer a source location."
);
const broker = figure(
  "broker",
  "LOOK THROUGH THE BROKER",
  "The agent requests read-only project information",
  "The coding agent requests information from Studio's read-only broker. Project facts and rendered output return to the agent. The agent's own editing tools remain separate.",
  `
<g class="support-part support-a"><rect x="16" y="54" width="130" height="164" rx="4" class="support-paper"/><text x="35" y="83" class="support-small">CODING AGENT</text><path d="M37 111h82m-82 18h61m-61 18h72" class="support-line"/><text x="35" y="193" class="support-code">request</text></g>
<path d="M147 125H194M351 125H394" class="support-connector" pathLength="1"/>
<g class="support-packet"><rect x="151" y="119" width="12" height="12" rx="2" class="support-pink"/></g>
<g class="support-part support-b"><rect x="196" y="35" width="156" height="183" rx="4" class="support-green"/><text x="215" y="67" class="support-small">STUDIO BROKER</text><text x="215" y="116" class="support-number">09</text><text x="215" y="146" class="support-code">read-only tools</text><path d="M215 168H333" class="support-line"/><text x="215" y="195" class="support-small">No file writes</text></g>
<g class="support-part support-c"><rect x="396" y="72" width="108" height="146" rx="4" class="support-blue"/><text x="409" y="99" class="support-small">PROJECT</text><text x="409" y="133" class="support-code">Frames</text><text x="409" y="160" class="support-code">Timing</text><text x="409" y="187" class="support-code">Checks</text></g>
<path d="M450 232v27H80v-27" class="support-return" pathLength="1"/><text x="265" y="285" text-anchor="middle" class="support-small">PROJECT CONTEXT RETURNS TO THE AGENT</text>`,
  "Read-only applies to Studio’s broker. The agent’s own tools can edit under your permission policy.",
  520,
  310
);
const recovery = figure(
  "recovery",
  "AN INTERRUPTED TASK IS NOT AN ACCEPTED EDIT",
  "Recovery keeps the accepted revision and retains the draft",
  "After an interruption, the last accepted revision remains the recovery point. Unfinished work is marked interrupted and its draft retained. The draft does not become the accepted revision.",
  `
<g class="support-part support-a" data-support-node="accepted-baseline"><rect x="25" y="65" width="250" height="185" rx="4" class="support-green"/><text x="45" y="96" class="support-small">ACCEPTED REVISION</text><text x="45" y="153" class="support-headline">Kept.</text><path d="m46 200 7 7 14-17" class="support-check"/><text x="80" y="207" class="support-code">Recovery baseline</text></g>
<g class="support-part support-b" data-support-node="interrupted-task"><rect x="335" y="65" width="230" height="145" rx="4" class="support-paper"/><text x="355" y="96" class="support-small">UNFINISHED TASK</text><path d="M357 116h7v24h-7Zm14 0h7v24h-7Z" class="support-pause"/><text x="355" y="181" class="support-code">Marked interrupted</text></g>
<path d="M577 137H623" class="support-connector" pathLength="1"/><g class="support-packet"><rect x="582" y="131" width="12" height="12" rx="2" class="support-blue"/></g>
<g class="support-part support-c" data-support-node="retained-draft"><rect x="635" y="65" width="240" height="145" rx="4" class="support-blue"/><text x="655" y="96" class="support-small">RETAINED DRAFT</text><text x="655" y="143" class="support-code">Unfinished work remains.</text><text x="655" y="181" class="support-small">Available after recovery</text></g>
<path d="M25 285H275" class="support-baseline"/><path d="M335 285H875" class="support-guide"/><text x="25" y="324" class="support-small">ACCEPTED FILES ARE THE BASELINE</text><text x="875" y="324" text-anchor="end" class="support-small">RETAINED DRAFT ≠ ACCEPTED REVISION</text>`,
  "Two separate outcomes: recover the accepted revision and retain the interrupted draft. Retaining a draft does not accept it.",
  900,
  350
);
let html = readFileSync(file, "utf8");
if (!html.includes("<!-- support:context -->")) {
  const start = html.indexOf(
    '<figure class="technical-figure">',
    html.indexOf('id="grounding"')
  );
  const end = html.indexOf("</figure>", start) + "</figure>".length;
  html =
    html.slice(0, start) +
    "<!-- support:context --><!-- /support:context -->" +
    html.slice(end);
  const tools = html.indexOf('id="tools"');
  const location = html.indexOf("</div>", tools);
  html =
    html.slice(0, location) +
    "<!-- support:broker --><!-- /support:broker -->" +
    html.slice(location);
  const recoveryEnd = html.indexOf("</section>", html.indexOf('id="recovery"'));
  html =
    html.slice(0, recoveryEnd) +
    "<!-- support:recovery --><!-- /support:recovery -->" +
    html.slice(recoveryEnd);
}
for (const [id, drawing] of Object.entries({ context, broker, recovery }))
  html = html.replace(
    new RegExp(`<!-- support:${id} -->[\\s\\S]*?<!-- /support:${id} -->`),
    `<!-- support:${id} -->${drawing}<!-- /support:${id} -->`
  );
writeFileSync(file, mobileDiagrams(html));
