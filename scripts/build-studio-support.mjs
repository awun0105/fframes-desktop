// Supplementary scroll diagrams. Static geometry stays readable without JavaScript.
import { readFileSync, writeFileSync } from "node:fs";
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
    <title id="${id}-title">${title}</title><desc id="${id}-desc">${description}</desc>${body}</svg>
  </div><figcaption>${caption}</figcaption>
</figure>`;
const context = figure(
  "context",
  "THE SELECTION BECOMES CONTEXT",
  "A selected title supplies project context",
  "A selected title supplies a stable editor key. A registered source anchor can add source context. It is optional and supplied by the project.",
  `
<path d="M44 285H856" class="support-guide"/>
<g class="support-part support-a"><rect x="30" y="56" width="260" height="206" rx="4" class="support-paper"/><path d="M30 87H290" class="support-line"/><text x="47" y="76" class="support-small">01 / CURRENT FRAME</text><text x="79" y="184" class="support-headline">Hello.</text><path d="M67 134v-12h15m133 12v-12h-15M67 195v12h15m133-12v12h-15" class="support-selection"/><text x="47" y="242" class="support-small">Selected object · headline</text></g>
<path d="M302 158H346" class="support-connector" pathLength="1"/><g class="support-packet"><rect x="303" y="152" width="12" height="12" rx="2" class="support-pink"/></g>
<g class="support-part support-b"><rect x="355" y="91" width="250" height="171" rx="4" class="support-blue"/><text x="375" y="118" class="support-small">02 / STABLE EDITOR KEY</text><text x="375" y="159" class="support-code">intro / starter-title</text><text x="375" y="184" class="support-code">headline / primary</text><path d="M375 207H584" class="support-line"/><text x="375" y="239" class="support-small">Identity from the project</text></g>
<path d="M617 176H656" class="support-connector" pathLength="1"/>
<g class="support-part support-c"><rect x="665" y="126" width="205" height="136" rx="4" class="support-green"/><text x="685" y="153" class="support-small">03 / OPTIONAL ANCHOR</text><text x="685" y="192" class="support-code">src/lib.rs</text><text x="685" y="218" class="support-code">render_frame</text><text x="685" y="244" class="support-small">When registered</text></g>
<text x="30" y="320" class="support-small">VISIBLE OBJECT</text><text x="870" y="320" text-anchor="end" class="support-small">USEFUL CONTEXT FOR THE AGENT →</text>`,
  "Illustrative title with a project-registered anchor. Selection identifies context; it does not edit the source."
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
<g class="support-part support-a"><rect x="25" y="45" width="250" height="170" rx="4" class="support-green"/><text x="47" y="76" class="support-small">LAST ACCEPTED REVISION</text><text x="47" y="130" class="support-headline">Kept.</text><path d="m48 172 7 7 14-17" class="support-check"/><text x="81" y="179" class="support-code">Recovery point</text></g>
<g class="support-part support-b"><rect x="326" y="71" width="236" height="144" rx="4" class="support-paper"/><text x="348" y="102" class="support-small">UNFINISHED TASK</text><path d="M350 126h8v27h-8Zm15 0h8v27h-8Z" class="support-pause"/><text x="348" y="183" class="support-code">Marked interrupted</text></g>
<path d="M574 144H618" class="support-connector" pathLength="1"/><g class="support-packet"><rect x="579" y="138" width="12" height="12" rx="2" class="support-pink"/></g>
<g class="support-part support-c"><rect x="632" y="98" width="243" height="117" rx="4" class="support-blue"/><text x="654" y="130" class="support-small">RETAINED DRAFT</text><text x="654" y="169" class="support-code">Unfinished work remains.</text><text x="654" y="193" class="support-small">Available after recovery</text></g>
<path d="M26 247H875" class="support-guide"/><text x="26" y="278" class="support-small">ACCEPTED FILES REMAIN THE BASELINE</text><text x="875" y="278" text-anchor="end" class="support-small">DRAFT ≠ ACCEPTED REVISION</text>`,
  "Recovery model, not a recording of the application. An exited process does not prove that an edit succeeded.",
  900,
  310
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
writeFileSync(file, html);
