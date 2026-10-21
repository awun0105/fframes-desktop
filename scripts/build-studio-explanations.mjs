// Fixed node geometry and directed paths: scroll plays each explanation as a scene
// (scripts/studio-scenes.mjs) without detaching connected nodes. Stage distances are
// illustrative, not elapsed time.
import { readFileSync, writeFileSync } from "node:fs";
import { mobileDiagrams } from "./studio-mobile-diagrams.mjs";
import {
  win,
  part,
  edge,
  token,
  chip,
  ring,
  narration,
} from "./studio-scenes.mjs";
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
  extra = "",
  inner = ""
) =>
  `<g class="ex-node ${extra}" data-ex-node="${id}"><rect x="${x}" y="${y}" width="${width}" height="${height}" rx="4" class="ex-${tone}"/><text x="${x + 18}" y="${y + 30}" class="ex-title">${title}</text>${lines.map((line, i) => `<text x="${x + 18}" y="${y + 55 + i * 20}" class="ex-copy">${line}</text>`).join("")}${inner}</g>`;
const fig = (id, heading, title, desc, body, caption, width, height) =>
  `<figure class="technical-figure explanation-figure explanation-${id}" data-explanation="${id}"><div class="figure-heading"><span class="eyebrow">${heading}</span><span class="illustration-label"><span class="scroll-cue">SCROLL TO FOLLOW · </span>ILLUSTRATION</span></div><p class="diagram-hint">Scroll the diagram sideways ↔</p><div class="diagram-scroll" tabindex="0" role="region" aria-label="${title}; scroll horizontally on small screens"><svg viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="ex-${id}-t ex-${id}-d"><title id="ex-${id}-t">${title}</title><desc id="ex-${id}-d">${desc}</desc>${body}</svg></div><figcaption>${caption}</figcaption></figure>`;
// One request through Studio: launch the agent, edit the draft, ask the broker,
// prepare the worker. Only the edges the responsibility map already states.
const process = fig(
  "process",
  "WHO DOES WHAT",
  "Studio, the coding agent and preview worker are separate processes",
  "Studio holds the UI, engine, task workflow, build service, journal and read-only broker. A managed coding-agent process edits its draft. The build service prepares a separate preview worker. Directed paths illustrate launch, tool calls and preview preparation.",
  `
<rect x="25" y="48" width="575" height="450" rx="5" class="ex-boundary"/><text x="43" y="33" class="ex-label">STUDIO APPLICATION</text>
<path d="M290 135H320M445 180V240M172 180V240M172 335V390" class="ex-structure"/>
${ring(55, 240, 235, 95, -0.01, 0, 0.18, 0.22)}
${ring(650, 80, 300, 110, 0.18, 0.22, 0.4, 0.44)}
${ring(650, 230, 300, 90, 0.3, 0.34, 0.42, 0.46)}
${ring(320, 390, 250, 80, 0.56, 0.6, 0.66, 0.7)}
${ring(650, 380, 300, 110, 0.8, 0.84, 0.97, 1)}
${node("ui", 55, 90, 235, 90, "UI + interaction", ["Draws; forwards input"], "blue")}
${node("engine", 320, 90, 250, 90, "Engine state", ["Serialized commands"])}
${node("workflow", 55, 240, 235, 95, "Agent workflow", ["Task + draft lifecycle"])}
${node("build", 320, 240, 250, 95, "Build service", ["Shared, keyed builds"], "paper", "", part("fade", 0.62, 0.68, '<text x="338" y="315" class="ex-copy">Preparing preview…</text>'))}
${node("journal", 55, 390, 235, 80, "Journal + store", ["History + recovery"])}
${node("broker", 320, 390, 250, 80, "Read-only broker", ["Project facts + output"], "green")}
${node("agent", 650, 80, 300, 110, "Coding agent", ["Managed child process", "Edits its draft copy"], "blue")}
${node("draft", 650, 230, 300, 90, "Agent draft", ["Separate working files"], "paper", "", part("rise", 0.32, 0.38, '<text x="668" y="305" class="ex-copy"><tspan class="ex-added">+ src/lib.rs</tspan> edited</text>'))}
${node("worker", 650, 380, 300, 110, "Preview worker", ["Your compiled video", "Frames · timing · audio"], "green")}
${edge("process-launch", "M290 280H305V210H630V125H650", 0.06, 0.2, "ex-path ex-step-a")}
${edge("process-edit", "M800 190V230", 0.24, 0.32, "ex-path ex-step-a")}
${edge("process-tools", "M650 165H615V430H570", 0.42, 0.58, "ex-path ex-step-b")}
<text x="650" y="354" class="ex-label">TOOLS</text>
${edge("process-prepare", "M570 288H628V435H650", 0.66, 0.82, "ex-path ex-step-c")}
${token("process-launch", 0.06, 0.2, chip("blue"))}
${token("process-edit", 0.24, 0.32, chip("pink"))}
${token("process-tools", 0.42, 0.58, chip("pink"))}
${token("process-prepare", 0.66, 0.82, chip("green"))}
<text x="650" y="530" class="ex-label">SEPARATE WORKER, SEPARATE PROCESS</text>
${narration(25, 585, [
  [-0.01, 0, "Studio coordinates the work"],
  [0.06, 0.1, "The workflow launches the coding agent"],
  [0.24, 0.28, "The agent edits its own draft"],
  [0.42, 0.46, "It asks the read-only broker for project facts"],
  [0.66, 0.7, "The build service prepares the preview worker"],
])}`,
  "Responsibility map, not a complete call graph. The agent owns draft edits; the preview worker remains separate from the Studio UI.",
  1000,
  610
);
// Bytes captured twice, compared column by column; only a match proceeds.
const bytes = ["7f", "3a", "c1", "09", "e4", "52"];
const cells = (y, start) =>
  bytes
    .map((byte, i) =>
      part(
        "pop",
        start + i * 0.03,
        start + i * 0.03 + 0.05,
        `<rect x="${283 + i * 28}" y="${y}" width="24" height="22" rx="3" class="sc-cell"/><text x="${295 + i * 28}" y="${y + 15.5}" text-anchor="middle" class="sc-cell-text">${byte}</text>`
      )
    )
    .join("");
const activity = [16, 9, 20, 7, 14, 18]
  .map(
    (h, i) =>
      `<rect x="${43 + i * 10}" y="${236 - h}" width="6" height="${h}" rx="1" class="sc sc-settle" ${win(0, 0.14)}/>`
  )
  .join("");
const capture = fig(
  "capture",
  "WHY CAPTURE TWICE?",
  "Only matching captures proceed to validation",
  "Once the writer is quiescent, Studio captures the draft twice. Matching bytes can proceed to validation. Any difference fails the capture.",
  `
${node("writer", 25, 125, 180, 125, "Writer ready", ["Draft is ready", "for capture"], "paper", "", activity + part("fade", 0.12, 0.18, '<text x="112" y="235" class="ex-copy">quiet</text>'))}
${node("capture-a", 265, 45, 205, 100, "Capture A", ["Captured bytes"], "blue", "", cells(110, 0.22))}
${node("capture-b", 265, 225, 205, 100, "Capture B", ["Captured bytes"], "blue", "", cells(290, 0.38))}
${bytes.map((_, i) => part("pop", 0.58 + i * 0.03, 0.62 + i * 0.03, `<path d="M${295 + i * 28} 148V166m0 16v20" class="sc-compare"/><text x="${295 + i * 28}" y="179" text-anchor="middle" class="sc-equal">=</text>`)).join("")}
${edge("capture-to-a", "M205 175H235V95H265", 0.14, 0.22, "ex-path ex-step-a")}
${edge("capture-to-b", "M205 195H235V275H265", 0.3, 0.38, "ex-path ex-step-b")}
${edge("capture-a-out", "M470 95H510V180H550", 0.76, 0.84, "ex-path ex-step-c")}
${edge("capture-b-out", "M470 275H510V200H550", 0.76, 0.84, "ex-path ex-step-c")}
${token("capture-to-a", 0.14, 0.22, chip("pink"))}
${token("capture-to-b", 0.3, 0.38, chip("pink"))}
${ring(550, 135, 185, 120, 0.84, 0.88, 0.97, 1)}
${node("matching", 550, 135, 185, 120, "Same bytes?", ["Yes → validate", "No → refuse"], "green", "", part("pop", 0.86, 0.92, '<path d="m692 156 7 7 15-17" class="sc-check"/>'))}
<text x="25" y="372" class="ex-label">A CHANGING DRAFT DOES NOT PROCEED</text>
${narration(
  735,
  372,
  [
    [-0.01, 0, "Writer goes quiet"],
    [0.14, 0.18, "Capture A"],
    [0.3, 0.34, "Capture B"],
    [0.56, 0.6, "Compare byte by byte"],
    [0.8, 0.84, "Match → validate"],
  ],
  "end"
)}`,
  "Two captures are compared after writer quiescence. A compile success cannot replace this capture check.",
  760,
  395
);
// The automatic route plays first, then the manual one; both cross the same guards.
const review = fig(
  "review",
  "TWO POLICIES, ONE ACCEPTANCE BOUNDARY",
  "Automatic application or manual Apply",
  "The default policy applies a validated candidate automatically. The optional manual policy waits for Apply. Both publish through the same guarded transaction. A changed file subset is validated again.",
  `
${ring(300, 40, 195, 100, 0.08, 0.12, 0.38, 0.42)}
${ring(300, 240, 195, 100, 0.48, 0.52, 0.78, 0.82)}
${node("candidate", 25, 135, 200, 120, "Valid candidate", ["Checks completed", "for this candidate"], "blue")}
${node("automatic", 300, 40, 195, 100, "Automatic", ["Default policy"], "green", "", part("fade", 0.16, 0.22, '<text x="318" y="120" class="ex-copy">Applies on its own</text>'))}
${node("manual", 300, 240, 195, 100, "Manual", ["Wait for Apply"], "paper", "", part("lit", 0.58, 0.64, '<g class="sc-button"><rect x="318" y="302" width="66" height="24" rx="4"/><text x="351" y="318" text-anchor="middle">Apply</text></g>') + part("pop", 0.56, 0.6, '<path d="M380 314v15l4-4 3 7 3-1-3-7h6Z" class="ex-pointer"/>'))}
<text x="398" y="200" text-anchor="middle" class="ex-or">OR</text>
${edge("review-auto-in", "M225 175H260V90H300", 0.08, 0.18, "ex-path ex-step-a")}
${edge("review-auto-out", "M495 90H515V175H565", 0.22, 0.34, "ex-path ex-step-b")}
${edge("review-manual-in", "M225 205H260V290H300", 0.46, 0.56, "ex-path ex-step-b")}
${edge("review-manual-out", "M495 290H515V205H565", 0.66, 0.78, "ex-path ex-step-c")}
${token("review-auto-in", 0.08, 0.18, chip("blue"))}
${token("review-auto-out", 0.22, 0.34, chip("green"))}
${token("review-manual-in", 0.46, 0.56, chip("blue"))}
${token("review-manual-out", 0.66, 0.78, chip("green"))}
<rect x="542" y="160" width="6" height="60" rx="2" class="ex-guard"/>
<text x="536" y="240" text-anchor="middle" class="ex-label">GUARDS</text>
${ring(542, 160, 6, 60, 0.3, 0.33, 0.38, 0.42, 3)}
${ring(542, 160, 6, 60, 0.74, 0.77, 0.84, 0.88, 3)}
${node("accepted", 565, 135, 170, 120, "Accepted", ["Guarded file-set", "publication"], "green", "", part("pop", 0.34, 0.4, '<path d="m692 156 7 7 15-17" class="sc-check"/>'))}
<text x="25" y="385" class="ex-label">SUBSET CHANGED? VALIDATE THAT SUBSET AGAIN.</text>
${narration(
  735,
  385,
  [
    [-0.01, 0, "A candidate passed its checks"],
    [0.08, 0.12, "Automatic: applied by default"],
    [0.46, 0.5, "Manual: waits for Apply"],
    [0.58, 0.62, "You press Apply"],
    [0.7, 0.74, "Both pass the same guards"],
  ],
  "end"
)}`,
  "Alternative policies, not sequential steps. Manual review is optional; neither route bypasses the publication guards.",
  760,
  405
);
// Child processes stay inside the observed tree; one that leaves it is caught at
// the boundary and its candidate is refused.
const containment = fig(
  "containment",
  "OBSERVED ESCAPE → CAPTURE REFUSED",
  "An observed writer escape blocks candidate capture",
  "The agent and child processes are tracked in a Windows job object or a process group with escape checks. If Studio observes a writer escape, its candidate is refused. This is not a general-purpose sandbox.",
  `
<rect x="25" y="55" width="550" height="285" rx="5" class="ex-boundary"/><text x="43" y="38" class="ex-label">OBSERVED WRITER PROCESS TREE</text>
${ring(25, 55, 550, 285, 0.26, 0.3, 0.36, 0.4)}
${ring(25, 55, 550, 285, 0.58, 0.6, 0.7, 0.74, 9).replace("sc-ring", "sc-ring sc-ring-warn")}
${node("agent", 55, 145, 185, 100, "Agent", ["Managed writer"], "blue")}
${edge("containment-node", "M240 180H270V132H310", 0.04, 0.14, "ex-path ex-step-a")}
${edge("containment-cargo", "M240 205H270V267H310", 0.1, 0.2, "ex-path ex-step-a")}
${part("fade", 0.1, 0.16, node("node", 310, 90, 220, 85, "Child process", ["For example: node"]))}
${part("fade", 0.16, 0.22, node("cargo", 310, 225, 220, 85, "Child process", ["For example: cargo"]))}
${edge("containment-escape", "M530 132H612V180H655", 0.44, 0.62, "ex-path ex-step-b sc-warn", "sc-warn")}
${token("containment-escape", 0.44, 0.62, chip("amber"))}
${part("pop", 0.58, 0.64, '<path d="m568 125 14 14m0-14-14 14" class="sc-cross"/>')}
${part("fade", 0.62, 0.7, node("escaped", 655, 130, 280, 120, "Observed escape", ["Outside tracked tree", "Candidate refused"], "amber"))}
${part("pop", 0.72, 0.8, '<g class="sc-stamp"><rect x="655" y="282" width="210" height="30" rx="4"/><text x="760" y="302" text-anchor="middle">CAPTURE DOES NOT PROCEED</text></g>')}
<text x="25" y="390" class="ex-label">WINDOWS: JOB OBJECT · LINUX/macOS: PROCESS GROUP + CHECKS</text>
${narration(
  935,
  390,
  [
    [-0.01, 0, "The agent starts child processes"],
    [0.26, 0.3, "Studio tracks the agent and its children"],
    [0.44, 0.48, "A writer leaves the tree"],
    [0.66, 0.7, "Its candidate is refused"],
  ],
  "end"
)}`,
  "The dashed line is an observed process boundary, not a universal sandbox. Work delegated to pre-existing services can be outside this boundary.",
  960,
  420
);
// Clips are split, reordered and trimmed; each gesture writes a new timing revision
// and the worker applies it with the build it already has.
const clip = (x, width, tone, label, extra = "") =>
  `<rect x="${x}" y="66" width="${width}" height="44" rx="4" class="ex-${tone}"/><text x="${x + 12}" y="93" class="ex-copy">${label}</text>${extra}`;
const move = (dx, a, b, inner) =>
  `<g class="sc sc-move" style="--a: ${a}; --b: ${b}; --dx: ${dx}px">${inner}</g>`;
const revision = (n, a, b, next) =>
  `<text x="487" y="234" text-anchor="end" class="sc ${next ? "sc-blink" : "sc-fade"} ex-label" style="--a: ${a}; --b: ${b}${next ? `; --c: ${next - 0.02}; --d: ${next}` : ""}">REV ${n}</text>`;
const timing = fig(
  "timing",
  "TIMING HAS ITS OWN DOCUMENT",
  "Timeline edits reuse the current Rust build",
  "Compiled scenes supply stable keys. Timeline gestures update order, splits and trims in timeline.json. The worker applies timing with the same source build.",
  `
<rect x="25" y="20" width="710" height="130" rx="5" class="ex-track"/>
<text x="43" y="46" class="ex-label">TIMELINE · SPLIT · MOVE · TRIM · UNDO / REDO</text>
${move(131, 0.32, 0.48, clip(45, 150, "blue", "A / Intro", part("fade", 0.56, 0.7, '<rect x="159" y="67" width="35" height="42" class="ex-trimmed"/><text x="176" y="128" text-anchor="middle" class="ex-label">TRIM</text>')))}
${move(131, 0.32, 0.48, clip(200, 112, "paper", "B / Main") + move(6, 0.14, 0.22, clip(312, 118, "paper", "", part("fade", 0.16, 0.22, '<text x="324" y="93" class="ex-copy">B₂</text>'))) + part("pop", 0.1, 0.14, '<path d="M315 58v60" class="ex-cut"/>'))}
<g class="sc sc-dip" ${win(0.3, 0.36)}><g class="sc sc-undip" ${win(0.44, 0.5)}>${move(-390, 0.32, 0.48, clip(435, 125, "green", "C / Outro"))}</g></g>
${node("scenes", 25, 205, 215, 120, "Compiled scenes", ["Stable scene keys", "Current built source"], "blue")}
${node("timing-doc", 275, 205, 230, 130, "timeline.json", ["Order · splits", "Trims · separate", "timing revision"], "green", "", revision(1, -0.01, 0, 0.22) + revision(2, 0.2, 0.22, 0.5) + revision(3, 0.48, 0.5, 0.7) + revision(4, 0.68, 0.7))}
${node("timed-worker", 555, 205, 180, 130, "Preview worker", ["Applies timing", "Same Rust build"], "green", "ex-confirm", part("pop", 0.84, 0.9, '<path d="m706 306 6 6 13-15" class="sc-check"/>'))}
${edge("timing-keys", "M240 265H275", -0.01, 0, "ex-path ex-step-a")}
${edge("timing-write", "M390 150V205", -0.01, 0, "ex-path ex-step-a")}
${token("timing-write", 0.14, 0.22, chip("blue"))}
${token("timing-write", 0.42, 0.5, chip("blue"))}
${token("timing-write", 0.62, 0.7, chip("blue"))}
${edge("timing-apply", "M505 270H555", 0.72, 0.8, "ex-path ex-step-c")}
${token("timing-apply", 0.72, 0.8, chip("green"))}
<text x="25" y="385" class="ex-label">TIMING CHANGE ≠ SOURCE REBUILD</text>
${narration(
  735,
  385,
  [
    [-0.01, 0, "Scenes in code order"],
    [0.1, 0.14, "Split Main in two"],
    [0.3, 0.34, "Move Outro to the front"],
    [0.56, 0.6, "Trim the end of Intro"],
    [0.72, 0.76, "Worker applies it · same build"],
  ],
  "end"
)}`,
  "Timing-only changes reuse the current build. Source changes still need compilation; timing edits are unavailable during an agent task.",
  760,
  405
);
// Inputs gather at the render job, which encodes until the file is written.
const output = fig(
  "export",
  "A FROZEN PROJECT, THE SESSION’S TIMING",
  "Export combines a frozen project with session timing",
  "The worker reports supported formats. Export uses a frozen project revision plus the current session timing document. Its worker encoder renders the chosen output with progress and cancellation.",
  `
${ring(555, 175, 180, 130, 0.44, 0.48, 0.86, 0.9)}
${node("capabilities", 25, 30, 210, 100, "Worker formats", ["Supported formats"], "blue")}
${node("settings", 295, 30, 200, 100, "Export settings", ["Format · size · fps"])}
${node("frozen-source", 25, 220, 210, 110, "Frozen project", ["Code revision", "Immutable input"], "paper", "", '<path d="M204 244h14v11h-14Zm3 0v-4a4 4 0 0 1 8 0v4" class="ex-lock"/>')}
${node("session-timing", 295, 250, 200, 90, "Session timing", ["Current timeline"], "green")}
${node("render", 555, 175, 180, 130, "Render job", ["Worker encoder", "Progress + Cancel"], "green", "ex-confirm", '<rect x="573" y="270" width="144" height="6" rx="3" class="sc-bar-track"/>' + part("span", 0.48, 0.86, '<rect x="573" y="270" width="144" height="6" rx="3" class="sc-bar"/>'))}
${node("output-file", 555, 340, 180, 70, "Video file", [], "green", "", part("pop", 0.92, 0.98, '<path d="m690 368 6 6 13-15" class="sc-check"/>'))}
${edge("export-formats", "M235 80H295", 0.04, 0.12, "ex-path ex-step-a")}
${edge("export-settings", "M495 80H645V175", 0.14, 0.26, "ex-path ex-step-b")}
${edge("export-frozen", "M235 275H260V220H555", 0.24, 0.36, "ex-path ex-step-b")}
${edge("export-timing", "M495 295H525V265H555", 0.32, 0.44, "ex-path ex-step-c")}
${edge("export-file", "M645 305V340", 0.86, 0.94, "ex-path ex-step-c")}
${token("export-formats", 0.04, 0.12, chip("blue"))}
${token("export-settings", 0.14, 0.26, chip("blue"))}
${token("export-frozen", 0.24, 0.36, chip("pink"))}
${token("export-timing", 0.32, 0.44, chip("green"))}
${token("export-file", 0.86, 0.94, chip("green"))}
${narration(25, 450, [
  [-0.01, 0, "The worker reports its formats"],
  [0.14, 0.18, "Settings pick format · size · fps"],
  [0.24, 0.28, "A frozen project and the session’s timing"],
  [0.48, 0.52, "The worker encoder renders"],
  [0.86, 0.9, "A video file is written"],
])}`,
  "Formats are worker-dependent. The frozen project's source and the session’s timing are separate inputs; rendering does not hold the live project open.",
  760,
  470
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
writeFileSync(file, mobileDiagrams(html));
