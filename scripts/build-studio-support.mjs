// Supplementary scroll diagrams, played as scenes by scroll (scripts/studio-scenes.mjs).
// Static geometry stays readable without JavaScript.
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
  <div class="figure-heading"><span class="eyebrow">${label}</span><span class="illustration-label"><span class="scroll-cue">SCROLL TO FOLLOW · </span>ILLUSTRATION</span></div>
  <p class="diagram-hint">Scroll the diagram sideways <span aria-hidden="true">↔</span></p>
  <div class="support-scroll" tabindex="0" role="region" aria-label="${title}; scroll horizontally on small screens">
    <svg viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="${id}-title ${id}-desc">
    <title id="${id}-title">${title}</title><desc id="${id}-desc">${description}</desc>${body}</svg>
  </div><figcaption>${caption}</figcaption>
</figure>`;
// A node that brightens when the scene reaches it. Opacity only: connectors stay
// attached to fixed node geometry.
const lit = (cls, node, a, b, inner) =>
  `<g class="support-part ${cls} sc sc-lit"${node ? ` data-support-node="${node}"` : ""} ${win(a, b)}>${inner}</g>`;
const check = (x, y) => `<path d="m${x} ${y} 6 6 13-15" class="sc-check"/>`;
const pointer = (x, y) =>
  `<path d="M${x} ${y}v15l4-4 3 7 3-1-3-7h6Z" class="ex-pointer"/>`;

// Click the title, the selection closes on it, its editor key resolves, then the
// optional source anchor joins.
const context = figure(
  "context",
  "THE SELECTION BECOMES CONTEXT",
  "A selected title supplies project context",
  "A selected title supplies a stable editor key. A registered source anchor can add source context. It is optional and supplied by the project.",
  `
${ring(30, 65, 250, 210, 0.04, 0.08, 0.22, 0.26)}
${ring(345, 65, 250, 210, 0.3, 0.34, 0.56, 0.6)}
${ring(660, 110, 210, 165, 0.7, 0.74, 0.97, 1)}
<g class="support-part support-a" data-support-node="selected-frame"><rect x="30" y="65" width="250" height="210" rx="4" class="support-paper"/><path d="M30 99H280" class="support-line"/><text x="48" y="88" class="support-small">01 / SELECTED FRAME</text><text x="78" y="183" class="support-headline">Hello.</text>${part("pop", 0.1, 0.18, '<path d="M66 133v-12h15m127 12v-12h-15M66 197v12h15m127-12v12h-15" class="support-selection"/>')}${part("fade", 0.16, 0.22, '<text x="48" y="252" class="support-small">Selected object · headline</text>')}</g>
<g class="sc sc-blink" ${blink(0.02, 0.08, 0.26, 0.3)}>${part("pop", 0.08, 0.12, '<circle cx="150" cy="166" r="11" class="support-click"/>')}${pointer(150, 166)}</g>
${edge("support-context-key", "M280 167H345", 0.24, 0.32)}
${token("support-context-key", 0.24, 0.32, chip("pink"))}
${lit("support-b", "object-identity", 0.28, 0.34, `<rect x="345" y="65" width="250" height="210" rx="4" class="support-blue"/><text x="365" y="95" class="support-small">02 / OBJECT IDENTITY</text>${part("rise", 0.36, 0.44, '<text x="365" y="143" class="support-code">intro / starter-title</text>')}${part("rise", 0.44, 0.52, '<text x="365" y="171" class="support-code">headline / primary</text>')}<path d="M365 204H575" class="support-line"/><text x="365" y="230" class="support-small">From this frame’s</text><text x="365" y="250" class="support-small">object map</text>`)}
${part("fade", 0.62, 0.7, '<path d="M595 185H660" class="support-connector" data-optional-link/><path d="m-8-4.5 8 4.5-8 4.5" transform="translate(660 185)" class="sc-head sc-quiet"/>')}
${lit("support-c", "optional-anchor", 0.66, 0.74, '<rect x="660" y="110" width="210" height="165" rx="4" class="support-paper support-optional"/><text x="680" y="141" class="support-small">03 / SOURCE ANCHOR</text><text x="680" y="180" class="support-code">src/lib.rs</text><text x="680" y="207" class="support-code">render_frame</text><text x="680" y="250" class="support-small">Only if registered</text>')}
<text x="30" y="322" class="support-small">SELECTION PROVIDES CONTEXT. IT DOES NOT EDIT SOURCE.</text>
${narration(30, 362, [
  [-0.01, 0, "Click an object in the frame"],
  [0.24, 0.28, "Studio resolves its stable editor key"],
  [0.62, 0.66, "A registered anchor can add a source location"],
])}`,
  "Frame → stable object identity → optional source anchor. Without a registered anchor, Studio does not infer a source location.",
  900,
  380
);

// The broker's tools pass by one at a time, then one call reads the project and its
// answer returns to the agent.
const tools = [
  "project_context",
  "selection_context",
  "source_lookup",
  "timeline",
  "render_frame",
  "render_strip",
  "inspect",
  "style_context",
  "build_status",
];
const toolCycle = tools
  .map((name, i) => {
    const a = 0.2 + i * 0.032;
    return `<g class="sc sc-blink" ${blink(a, a + 0.006, a + 0.026, a + 0.032)}><text x="215" y="116" class="support-number">${String(i + 1).padStart(2, "0")}</text><text x="215" y="146" class="support-small">${name}</text></g>`;
  })
  .join("");
const broker = figure(
  "broker",
  "LOOK THROUGH THE BROKER",
  "The agent requests read-only project information",
  "The coding agent requests information from Studio's read-only broker. Project facts and rendered output return to the agent. The agent's own editing tools remain separate.",
  `
${ring(196, 35, 156, 183, 0.16, 0.2, 0.5, 0.54)}
<g class="support-part support-a"><rect x="16" y="54" width="130" height="164" rx="4" class="support-paper"/><text x="35" y="83" class="support-small">CODING AGENT</text><path d="M37 111h82m-82 18h61m-61 18h72" class="support-line"/><text x="35" y="193" class="support-code">request</text>${part("pop", 0.84, 0.9, check(112, 182))}</g>
${edge("support-broker-ask", "M146 125H196", 0.06, 0.16)}
${token("support-broker-ask", 0.06, 0.16, chip("pink"))}
<g class="support-part support-b"><rect x="196" y="35" width="156" height="183" rx="4" class="support-green"/><text x="215" y="67" class="support-small">STUDIO BROKER</text>${toolCycle}${part("fade", 0.49, 0.53, '<text x="215" y="116" class="support-number">09</text><text x="215" y="146" class="support-small">read-only tools</text>')}<path d="M215 168H333" class="support-line"/><text x="215" y="195" class="support-small">No file writes</text></g>
${edge("support-broker-read", "M352 125H396", 0.52, 0.6)}
${token("support-broker-read", 0.52, 0.6, chip("green"))}
${lit("support-c", "", 0.56, 0.62, '<rect x="396" y="72" width="108" height="146" rx="4" class="support-blue"/><text x="409" y="99" class="support-small">PROJECT</text><text x="409" y="133" class="support-code">Frames</text><text x="409" y="160" class="support-code">Timing</text><text x="409" y="187" class="support-code">Checks</text>')}
${ring(400, 114, 96, 28, 0.58, 0.62, 0.82, 0.86, 4)}
${edge("support-broker-return", "M450 232V259H80V232", 0.64, 0.82)}
${token("support-broker-return", 0.64, 0.82, chip("blue"))}
<text x="265" y="285" text-anchor="middle" class="support-small">PROJECT CONTEXT RETURNS TO THE AGENT</text>
${narration(16, 322, [
  [-0.01, 0, "The agent sends a request"],
  [0.2, 0.24, "The broker offers nine read-only tools"],
  [0.52, 0.56, "One call reads project facts or output"],
  [0.64, 0.68, "The answer returns to the agent"],
])}`,
  "Read-only applies to Studio’s broker. The agent’s own tools can edit under your permission policy.",
  520,
  340
);

// A session rail: accept, start a task, stop, recover. The rail draws over the scene,
// so each event lights up when the line reaches it.
const reach = x => ((x - 40) / 820) * 0.8;
const stop = (x, label, mark) =>
  part(
    "pop",
    reach(x),
    reach(x) + 0.04,
    `${mark}<text x="${x}" y="28" text-anchor="middle" class="support-small">${label}</text>`
  );
const recovery = figure(
  "recovery",
  "AN INTERRUPTED TASK IS NOT AN ACCEPTED EDIT",
  "Recovery keeps the accepted revision and retains the draft",
  "After an interruption, the last accepted revision remains the recovery point. Unfinished work is marked interrupted and its draft retained. The draft does not become the accepted revision.",
  `
<path d="M40 50H860" class="support-guide"/><path d="M40 50H860" class="sc sc-draw support-rail" ${win(0, 0.8)} pathLength="1"/>
${stop(150, "ACCEPTED", '<circle cx="150" cy="50" r="6" class="sc-chip sc-chip-green"/>')}
${stop(450, "TASK STARTS", '<circle cx="450" cy="50" r="6" class="sc-chip sc-chip-blue"/>')}
${stop(600, "INTERRUPTION", '<path d="m594 44 12 12m0-12-12 12" class="sc-cross"/>')}
${stop(780, "RECOVERY", '<circle cx="780" cy="50" r="6" class="sc-chip sc-chip-green"/>')}
${ring(25, 85, 250, 185, 0.72, 0.76, 1.1, 1.2)}
${lit("support-a", "accepted-baseline", 0.08, 0.14, `<rect x="25" y="85" width="250" height="185" rx="4" class="support-green"/><text x="45" y="116" class="support-small">ACCEPTED REVISION</text><text x="45" y="173" class="support-headline">Kept.</text>${part("pop", 0.1, 0.14, '<path d="m46 220 7 7 14-17" class="support-check"/>')}<text x="80" y="227" class="support-code">Recovery baseline</text>`)}
${lit("support-b", "interrupted-task", 0.36, 0.4, `<rect x="335" y="85" width="230" height="165" rx="4" class="support-paper"/><text x="355" y="116" class="support-small">UNFINISHED TASK</text><rect x="355" y="136" width="190" height="6" rx="3" class="sc-bar-track"/><rect x="355" y="136" width="118" height="6" rx="3" class="sc sc-bar" ${win(0.4, 0.55)}/>${part("fade", 0.56, 0.62, '<path d="M357 160h7v24h-7Zm14 0h7v24h-7Z" class="support-pause"/>')}<text x="355" y="226" class="support-code sc sc-blink" ${blink(0.4, 0.44, 0.55, 0.58)}>Editing its draft…</text>${part("fade", 0.58, 0.64, '<text x="355" y="226" class="support-code">Marked interrupted</text>')}`)}
${edge("support-recovery-keep", "M565 167H635", 0.74, 0.84)}
${token("support-recovery-keep", 0.74, 0.84, chip("blue"))}
${lit("support-c", "retained-draft", 0.8, 0.88, '<rect x="635" y="85" width="240" height="165" rx="4" class="support-blue"/><text x="655" y="116" class="support-small">RETAINED DRAFT</text><text x="655" y="173" class="support-code">Draft remains.</text><text x="655" y="211" class="support-small">Available after recovery</text>')}
<path d="M25 305H275" class="support-baseline"/><path d="M335 305H875" class="support-guide"/><text x="25" y="340" class="support-small">ACCEPTED FILES ARE THE BASELINE</text><text x="875" y="340" text-anchor="end" class="support-small">RETAINED DRAFT ≠ ACCEPTED REVISION</text>
${narration(25, 382, [
  [-0.01, 0, "You accept a revision"],
  [0.36, 0.4, "A new task edits its own draft"],
  [0.55, 0.58, "The task is interrupted before it finishes"],
  [0.72, 0.76, "Recovery restores the accepted revision and keeps the draft"],
])}`,
  "Two separate outcomes: recover the accepted revision and retain the interrupted draft. Retaining a draft does not accept it.",
  900,
  400
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
