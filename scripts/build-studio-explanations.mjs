// Fixed node geometry and directed paths: scroll draws the explanation without
// detaching connected nodes. Stage distances are illustrative, not elapsed time.
import { readFileSync, writeFileSync } from "node:fs";
const file = "landing/studio/how-it-works.html";
const node = (
  id,
  x,
  y,
  width,
  height,
  title,
  lines,
  tone = "paper",
  extra = ""
) =>
  `<g class="ex-node ${extra}" data-ex-node="${id}"><rect x="${x}" y="${y}" width="${width}" height="${height}" rx="4" class="ex-${tone}"/><text x="${x + 18}" y="${y + 30}" class="ex-title">${title}</text>${lines.map((line, i) => `<text x="${x + 18}" y="${y + 55 + i * 20}" class="ex-copy">${line}</text>`).join("")}</g>`;
const path = (id, d, stage = "a", extra = "") =>
  `<path class="ex-path ex-step-${stage} ${extra}" d="${d}" pathLength="1" marker-end="url(#ex-${id}-arrow)"/>`;
const fig = (id, heading, title, desc, body, caption, width, height) =>
  `<figure class="technical-figure explanation-figure explanation-${id}" data-explanation="${id}"><div class="figure-heading"><span class="eyebrow">${heading}</span><span class="illustration-label">ARCHITECTURE ILLUSTRATION</span></div><p class="diagram-hint">Scroll the diagram sideways ↔</p><div class="diagram-scroll" tabindex="0" role="region" aria-label="${title}; scroll horizontally on small screens"><svg viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="ex-${id}-t ex-${id}-d"><title id="ex-${id}-t">${title}</title><desc id="ex-${id}-d">${desc}</desc><defs><marker id="ex-${id}-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="m1 1 8 4-8 4" class="ex-arrow"/></marker></defs>${body}</svg></div><figcaption>${caption}</figcaption></figure>`;
const process = fig(
  "process",
  "WHO DOES WHAT",
  "Studio, the coding agent and preview worker are separate processes",
  "Studio holds the UI, engine, task workflow, build service, journal and read-only broker. A managed coding-agent process edits its draft. The build service prepares a separate preview worker. Directed paths illustrate launch, tool calls and preview preparation.",
  `
<rect x="25" y="48" width="570" height="450" rx="5" class="ex-boundary"/><text x="43" y="33" class="ex-label">STUDIO APPLICATION</text>
${node("ui", 55, 90, 220, 90, "UI + interaction", ["Draws; forwards input"], "blue")}
${node("engine", 315, 90, 250, 90, "Engine state", ["Serialized commands"])}
${node("workflow", 55, 240, 220, 95, "Agent workflow", ["Task + draft lifecycle"])}
${node("build", 315, 240, 250, 95, "Build service", ["Shared, keyed builds"])}
${node("journal", 55, 390, 220, 80, "Journal + store", ["History + recovery"])}
${node("broker", 315, 390, 250, 80, "Read-only broker", ["Project facts + output"], "green")}
<path d="M275 135H315M440 180V240M165 180V240M165 335V390" class="ex-structure"/>
${node("agent", 650, 80, 300, 110, "Coding agent", ["Managed child process", "Edits its draft copy"], "blue")}
${node("draft", 650, 230, 300, 90, "Agent draft", ["Separate working files"])}
${node("worker", 650, 380, 300, 110, "Preview worker", ["Your compiled video", "Frames · timing · audio"], "green")}
${path("process", "M275 280H292V210H630V125H650", "a")}
${path("process", "M800 190V230", "a")}
${path("process", "M650 165H610V430H565", "b")}
<text x="650" y="354" class="ex-label">TOOLS</text>
${path("process", "M565 288H628V435H650", "c")}
<text x="650" y="530" class="ex-label">SEPARATE WORKER, SEPARATE PROCESS</text>`,
  "Responsibility map, not a complete call graph. The agent owns draft edits; the preview worker remains separate from the Studio UI.",
  1000,
  555
);
const capture = fig(
  "capture",
  "WHY CAPTURE TWICE?",
  "Only matching captures proceed to validation",
  "Once the writer is quiescent, Studio captures the draft twice. Matching bytes can proceed to validation. Any difference fails the capture.",
  `
${node("writer", 25, 125, 180, 125, "Writer quiescent", ["Draft is ready", "for capture"])}
${node("capture-a", 265, 45, 205, 100, "Capture A", ["Captured bytes"], "blue")}
${node("capture-b", 265, 225, 205, 100, "Capture B", ["Captured bytes"], "blue")}
${path("capture", "M205 175H235V95H265", "a")}
${path("capture", "M205 195H235V275H265", "b")}
${path("capture", "M470 95H510V180H550", "c")}
${path("capture", "M470 275H510V200H550", "c")}
${node("matching", 550, 135, 185, 120, "Same bytes?", ["Yes → validate", "No → refuse capture"], "green", "ex-confirm")}
<text x="25" y="370" class="ex-label">A CHANGING DRAFT DOES NOT PROCEED</text>`,
  "Two captures are compared after writer quiescence. A compile success cannot replace this capture check.",
  760,
  395
);
const review = fig(
  "review",
  "TWO POLICIES, ONE ACCEPTANCE BOUNDARY",
  "Automatic application or manual Apply",
  "The default policy applies a validated candidate automatically. The optional manual policy waits for Apply. Both publish through the same guarded transaction. A changed file subset is validated again.",
  `
${node("candidate", 25, 135, 200, 120, "Validated candidate", ["Checks completed", "for this candidate"], "blue")}
${node("automatic", 300, 40, 195, 100, "Automatic", ["Default policy"], "green")}
${node("manual", 300, 240, 195, 100, "Manual", ["Wait for Apply"])}
<text x="398" y="200" text-anchor="middle" class="ex-or">OR</text>
${path("review", "M225 175H260V90H300", "a")}
${path("review", "M225 205H260V290H300", "b")}
${path("review", "M495 90H530V175H565", "c")}
${path("review", "M495 290H530V205H565", "c")}
${node("accepted", 565, 135, 170, 120, "Accepted", ["Guarded file-set", "publication"], "green", "ex-confirm")}
<text x="25" y="380" class="ex-label">SUBSET CHANGED? VALIDATE THAT SUBSET AGAIN.</text>`,
  "Alternative policies, not sequential steps. Manual review is optional; neither route bypasses the publication guards.",
  760,
  405
);
const containment = fig(
  "containment",
  "OBSERVED ESCAPE → CAPTURE REFUSED",
  "An observed writer escape blocks candidate capture",
  "The agent and child processes are tracked in a Windows job object or a process group with escape checks. If Studio observes a writer escape, its candidate is refused. This is not a general-purpose sandbox.",
  `
<rect x="25" y="55" width="550" height="285" rx="5" class="ex-boundary"/><text x="43" y="38" class="ex-label">OBSERVED WRITER PROCESS TREE</text>
${node("agent", 55, 145, 185, 100, "Agent", ["Managed writer"], "blue")}
${node("node", 310, 90, 220, 85, "Child process", ["For example: node"])}
${node("cargo", 310, 225, 220, 85, "Child process", ["For example: cargo"])}
${path("containment", "M240 180H270V132H310", "a")}
${path("containment", "M240 205H270V267H310", "a")}
${path("containment", "M530 132H612V180H655", "b", "ex-escape")}
${node("escaped", 655, 130, 280, 120, "Observed escape", ["Outside tracked tree", "Candidate refused"], "amber", "ex-confirm")}
<path d="m598 173 13 13m0-13-13 13" class="ex-stop"/>
<text x="655" y="303" class="ex-label">CAPTURE DOES NOT PROCEED</text>
<text x="25" y="390" class="ex-label">WINDOWS: JOB OBJECT · LINUX/macOS: PROCESS GROUP + CHECKS</text>`,
  "The dashed line is an observed process boundary, not a universal sandbox. Work delegated to pre-existing services can be outside this boundary.",
  960,
  420
);
const timing = fig(
  "timing",
  "TIMING HAS ITS OWN DOCUMENT",
  "Timeline edits reuse the current Rust build",
  "Compiled scenes supply stable keys. Timeline gestures update order, splits and trims in timeline.json. The worker applies timing with the same source build.",
  `
${node("scenes", 25, 45, 210, 110, "Compiled scenes", ["Stable scene keys", "Current built source"], "blue")}
${node("gesture", 25, 240, 210, 110, "Timeline gesture", ["Split · trim · move", "Undo / redo"])}
${path("timing", "M235 100H265V150H290", "a")}
${path("timing", "M235 295H265V210H290", "b")}
${node("timing-doc", 290, 115, 210, 130, "timeline.json", ["Order · splits · trims", "Separate timing revision"], "green")}
${path("timing", "M500 180H540", "c")}
${node("timed-worker", 540, 115, 195, 130, "Preview worker", ["Applies timing", "Same Rust build"], "green", "ex-confirm")}
<text x="290" y="330" class="ex-label">TIMING CHANGE ≠ SOURCE REBUILD</text>`,
  "Timing-only changes reuse the current build. Source changes still need compilation; timing edits are unavailable during an agent task.",
  760,
  395
);
const output = fig(
  "export",
  "A FROZEN PROJECT, THE SESSION’S TIMING",
  "Export combines a frozen project with session timing",
  "The worker reports supported formats. Export uses a frozen project revision plus the current session timing document. Its worker encoder renders the chosen output with progress and cancellation.",
  `
${node("capabilities", 25, 30, 210, 100, "Worker capabilities", ["Supported formats"], "blue")}
${path("export", "M235 80H295", "a")}
${node("settings", 295, 30, 200, 100, "Export settings", ["Format · size · fps"])}
${path("export", "M495 80H645V175", "b")}
${node("frozen-source", 25, 220, 210, 110, "Frozen project", ["Selected code revision", "Immutable render input"])}
${path("export", "M235 275H260V220H555", "b")}
${node("session-timing", 295, 250, 200, 90, "Session timing", ["Current timeline"], "green")}
${path("export", "M495 295H525V265H555", "c")}
${node("render", 555, 175, 180, 130, "Render job", ["Worker encoder", "Progress + Cancel"], "green", "ex-confirm")}
${path("export", "M645 305V340", "c")}
${node("output-file", 555, 340, 180, 70, "Video file", [], "green")}`,
  "Formats are worker-dependent. The frozen project's source and the session’s timing are separate inputs; rendering does not hold the live project open.",
  760,
  430
);
let html = readFileSync(file, "utf8");
function replaceFigure(needle, id) {
  if (html.includes(`<!-- explanations:${id} -->`)) return;
  const n = html.indexOf(needle);
  if (n < 0) throw Error(`Missing ${needle}`);
  const start = html.lastIndexOf("<figure", n),
    end = html.indexOf("</figure>", n) + 9;
  html =
    html.slice(0, start) +
    `<!-- explanations:${id} --><!-- /explanations:${id} -->` +
    html.slice(end);
}
replaceFigure('class="technical-figure process-figure"', "process");
replaceFigure("<!-- fig:fence -->", "containment");
if (!html.includes("<!-- explanations:capture -->")) {
  const start = html.lastIndexOf("<section", html.indexOf('id="grounding"'));
  html =
    html.slice(0, start) +
    `<div class="transaction-explanations"><!-- explanations:capture --><!-- /explanations:capture --><!-- explanations:review --><!-- /explanations:review --></div>\n` +
    html.slice(start);
}
if (!html.includes("<!-- explanations:timing -->")) {
  const start = html.indexOf('<aside class="guide-install"');
  html =
    html.slice(0, start) +
    '<div class="pipeline-explanations"><!-- explanations:timing --><!-- /explanations:timing --><!-- explanations:export --><!-- /explanations:export --></div>\n' +
    html.slice(start);
}
for (const [id, drawing] of Object.entries({
  process,
  capture,
  review,
  containment,
  timing,
  export: output,
}))
  html = html.replace(
    new RegExp(
      `<!-- explanations:${id} -->[\\s\\S]*?<!-- /explanations:${id} -->`
    ),
    `<!-- explanations:${id} -->${drawing}<!-- /explanations:${id} -->`
  );
writeFileSync(file, html);
