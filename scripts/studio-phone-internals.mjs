// Phone compositions of the How it works figures: portrait SVGs, 340 units wide, with
// the same facts and scroll windows as the wide figures. Every id ends in -m so tokens
// follow these paths, not the wide ones. Mono text is about 6.6 units per character at
// 11px, so lines are written short enough for their boxes.
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

const say = (y, steps) => narration(12, y, steps, "start", 42);
const node = (x, y, w, h, title, lines, tone = "paper", inner = "") =>
  `<g class="ex-node"><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="4" class="ex-${tone}"/><text x="${x + 12}" y="${y + 22}" class="ex-title">${title}</text>${lines.map((line, i) => `<text x="${x + 12}" y="${y + 40 + i * 16}" class="ex-copy">${line}</text>`).join("")}${inner}</g>`;
const path = (id, d, a, b, step, cls = "") =>
  edge(`${id}-m`, d, a, b, `ex-path ex-step-${step} ${cls}`, cls);
const move = (dx, a, b, inner) =>
  `<g class="sc sc-move" style="--a: ${a}; --b: ${b}; --dx: ${dx}px">${inner}</g>`;
const check = (x, y) => `<path d="m${x} ${y} 6 6 13-15" class="sc-check"/>`;
const pointer = (x, y) =>
  `<path d="M${x} ${y}v15l4-4 3 7 3-1-3-7h6Z" class="ex-pointer"/>`;

// Studio's parts inside its boundary; the agent, its draft and the worker outside.
export const process = {
  height: 572,
  body: `
<rect x="4" y="20" width="332" height="238" rx="5" class="ex-boundary"/><text x="8" y="13" class="ex-label">STUDIO APPLICATION</text>
${ring(10, 182, 152, 66, -0.01, 0, 0.18, 0.22)}
${ring(10, 290, 152, 82, 0.18, 0.22, 0.4, 0.44)}
${ring(10, 400, 152, 84, 0.3, 0.34, 0.42, 0.46)}
${ring(178, 182, 152, 66, 0.56, 0.6, 0.66, 0.7)}
${ring(178, 290, 152, 82, 0.8, 0.84, 0.97, 1)}
${node(10, 30, 152, 66, "UI + interaction", ["Draws; forwards", "input"], "blue")}
${node(178, 30, 152, 66, "Engine state", ["Serialized", "commands"])}
${node(10, 106, 152, 66, "Journal + store", ["History +", "recovery"])}
${node(178, 106, 152, 66, "Read-only broker", ["Project facts", "+ output"], "green")}
${node(10, 182, 152, 66, "Agent workflow", ["Task + draft", "lifecycle"])}
${node(178, 182, 152, 66, "Build service", ["Shared builds"], "paper", part("fade", 0.62, 0.68, '<text x="190" y="238" class="ex-copy">Preparing preview…</text>'))}
${node(10, 290, 152, 82, "Coding agent", ["Managed child", "process; edits", "its draft copy"], "blue")}
${node(178, 290, 152, 82, "Preview worker", ["Your compiled", "video: frames,", "timing, audio"], "green")}
${node(10, 400, 152, 84, "Agent draft", ["Separate", "working files"], "paper", part("rise", 0.32, 0.38, '<text x="22" y="472" class="ex-copy"><tspan class="ex-added">+ src/lib.rs</tspan> edited</text>'))}
${path("process-launch", "M86 248V290", 0.06, 0.2, "a")}
${path("process-edit", "M86 372V400", 0.24, 0.32, "a")}
${path("process-tools", "M162 310H170V139H178", 0.42, 0.58, "b")}
<text x="166" y="282" text-anchor="end" class="ex-label">TOOLS</text>
${path("process-prepare", "M254 248V290", 0.66, 0.82, "c")}
${token("process-launch-m", 0.06, 0.2, chip("blue"))}
${token("process-edit-m", 0.24, 0.32, chip("pink"))}
${token("process-tools-m", 0.42, 0.58, chip("pink"))}
${token("process-prepare-m", 0.66, 0.82, chip("green"))}
<text x="330" y="506" text-anchor="end" class="ex-label">SEPARATE WORKER, SEPARATE PROCESS</text>
${say(534, [
  [-0.01, 0, "Studio coordinates the work"],
  [0.06, 0.1, "The workflow launches the coding agent"],
  [0.24, 0.28, "The agent edits its own draft"],
  [0.42, 0.46, "It asks the read-only broker for project facts"],
  [0.66, 0.7, "The build service prepares the preview worker"],
])}`,
};

// Bytes captured twice, compared column by column; only a match proceeds.
const bytes = ["7f", "3a", "c1", "09", "e4", "52"];
const cells = (y, start) =>
  bytes
    .map((byte, i) =>
      part(
        "pop",
        start + i * 0.03,
        start + i * 0.03 + 0.05,
        `<rect x="${150 + i * 28}" y="${y}" width="24" height="22" rx="3" class="sc-cell"/><text x="${162 + i * 28}" y="${y + 15.5}" text-anchor="middle" class="sc-cell-text">${byte}</text>`
      )
    )
    .join("");
const activity = [16, 9, 20, 7, 14, 18]
  .map(
    (h, i) =>
      `<rect x="${230 + i * 10}" y="${62 - h}" width="6" height="${h}" rx="1" class="sc sc-settle" ${win(0, 0.14)}/>`
  )
  .join("");
export const capture = {
  height: 452,
  body: `
${node(12, 8, 316, 66, "Writer ready", ["Draft is ready for capture"], "paper", activity + part("fade", 0.12, 0.18, '<text x="292" y="60" class="ex-copy">quiet</text>'))}
${node(40, 104, 288, 66, "Capture A", ["Captured bytes"], "blue", cells(128, 0.22))}
${node(40, 204, 288, 66, "Capture B", ["Captured bytes"], "blue", cells(228, 0.38))}
${bytes.map((_, i) => part("pop", 0.58 + i * 0.03, 0.62 + i * 0.03, `<path d="M${162 + i * 28} 152V178m0 16v32" class="sc-compare"/><text x="${162 + i * 28}" y="191" text-anchor="middle" class="sc-equal">=</text>`)).join("")}
${path("capture-to-a", "M22 74V137H40", 0.14, 0.22, "a")}
${path("capture-to-b", "M30 74V237H40", 0.3, 0.38, "b")}
${path("capture-a-out", "M328 137H334V328H328", 0.76, 0.84, "c")}
${path("capture-b-out", "M184 270V296", 0.76, 0.84, "c")}
${token("capture-to-a-m", 0.14, 0.22, chip("pink"))}
${token("capture-to-b-m", 0.3, 0.38, chip("pink"))}
${ring(40, 296, 288, 66, 0.84, 0.88, 0.97, 1)}
${node(40, 296, 288, 66, "Same bytes?", ["Yes → validate · No → refuse"], "green", part("pop", 0.86, 0.92, '<path d="m296 310 7 7 15-17" class="sc-check"/>'))}
<text x="12" y="392" class="ex-label">A CHANGING DRAFT DOES NOT PROCEED</text>
${say(420, [
  [-0.01, 0, "Writer goes quiet"],
  [0.14, 0.18, "Capture A"],
  [0.3, 0.34, "Capture B"],
  [0.56, 0.6, "Compare byte by byte"],
  [0.8, 0.84, "Match → validate"],
])}`,
};

// The automatic route plays first, then the manual one; both cross the same guards.
export const review = {
  height: 404,
  body: `
${ring(12, 104, 146, 84, 0.08, 0.12, 0.38, 0.42)}
${ring(182, 104, 146, 84, 0.48, 0.52, 0.78, 0.82)}
${node(12, 8, 316, 58, "Valid candidate", ["Checks completed for this candidate"], "blue")}
${node(12, 104, 146, 84, "Automatic", ["Default policy"], "green", part("fade", 0.16, 0.22, '<text x="24" y="164" class="ex-copy">Applies on its own</text>'))}
${node(182, 104, 146, 84, "Manual", ["Wait for Apply"], "paper", part("lit", 0.58, 0.64, '<g class="sc-button"><rect x="194" y="152" width="66" height="24" rx="4"/><text x="227" y="168" text-anchor="middle">Apply</text></g>') + part("pop", 0.56, 0.6, pointer(252, 162)))}
<text x="170" y="153" text-anchor="middle" class="ex-or">OR</text>
${path("review-auto-in", "M85 66V104", 0.08, 0.18, "a")}
${path("review-auto-out", "M85 188V250", 0.22, 0.34, "b")}
${path("review-manual-in", "M255 66V104", 0.46, 0.56, "b")}
${path("review-manual-out", "M255 188V250", 0.66, 0.78, "c")}
${token("review-auto-in-m", 0.08, 0.18, chip("blue"))}
${token("review-auto-out-m", 0.22, 0.34, chip("green"))}
${token("review-manual-in-m", 0.46, 0.56, chip("blue"))}
${token("review-manual-out-m", 0.66, 0.78, chip("green"))}
<rect x="40" y="214" width="260" height="6" rx="2" class="ex-guard"/>
<text x="170" y="238" text-anchor="middle" class="ex-label">GUARDS</text>
${ring(40, 214, 260, 6, 0.3, 0.33, 0.38, 0.42, 3)}
${ring(40, 214, 260, 6, 0.74, 0.77, 0.84, 0.88, 3)}
${node(40, 250, 260, 66, "Accepted", ["Guarded file-set publication"], "green", part("pop", 0.34, 0.4, '<path d="m262 264 7 7 15-17" class="sc-check"/>'))}
<text x="12" y="344" class="ex-label">SUBSET CHANGED? VALIDATE THAT SUBSET AGAIN.</text>
${say(372, [
  [-0.01, 0, "A candidate passed its checks"],
  [0.08, 0.12, "Automatic: applied by default"],
  [0.46, 0.5, "Manual: waits for Apply"],
  [0.58, 0.62, "You press Apply"],
  [0.7, 0.74, "Both pass the same guards"],
])}`,
};

// Child processes stay inside the observed tree; one that leaves it is caught at the
// boundary and its candidate is refused.
export const containment = {
  height: 492,
  body: `
<rect x="6" y="24" width="328" height="196" rx="5" class="ex-boundary"/><text x="8" y="15" class="ex-label">OBSERVED WRITER PROCESS TREE</text>
${ring(6, 24, 328, 196, 0.26, 0.3, 0.36, 0.4)}
${ring(6, 24, 328, 196, 0.58, 0.6, 0.7, 0.74, 9).replace("sc-ring", "sc-ring sc-ring-warn")}
${node(18, 82, 130, 66, "Agent", ["Managed writer"], "blue")}
${path("containment-cargo", "M148 104H166V73H186", 0.04, 0.14, "a")}
${path("containment-node", "M148 126H166V173H186", 0.1, 0.2, "a")}
${part("fade", 0.1, 0.16, node(186, 36, 136, 74, "Child process", ["For example:", "cargo"]))}
${part("fade", 0.16, 0.22, node(186, 136, 136, 74, "Child process", ["For example:", "node"]))}
${path("containment-escape", "M254 210V262", 0.44, 0.62, "b", "sc-warn")}
${token("containment-escape-m", 0.44, 0.62, chip("amber"))}
${part("pop", 0.58, 0.64, '<path d="m247 213 14 14m0-14-14 14" class="sc-cross"/>')}
${part("fade", 0.62, 0.7, node(120, 262, 208, 84, "Observed escape", ["Outside tracked tree", "Candidate refused"], "amber"))}
${part("pop", 0.72, 0.8, '<g class="sc-stamp"><rect x="120" y="358" width="208" height="30" rx="4"/><text x="224" y="378" text-anchor="middle">CAPTURE DOES NOT PROCEED</text></g>')}
<text x="12" y="414" class="ex-label">WINDOWS: JOB OBJECT</text><text x="12" y="430" class="ex-label">LINUX/macOS: PROCESS GROUP + CHECKS</text>
${say(456, [
  [-0.01, 0, "The agent starts child processes"],
  [0.26, 0.3, "Studio tracks the agent and its children"],
  [0.44, 0.48, "A writer leaves the tree"],
  [0.66, 0.7, "Its candidate is refused"],
])}`,
};

// Clips are split, reordered and trimmed; each gesture writes a new timing revision
// and the worker applies it with the build it already has. Clip positions are the
// wide figure's, scaled by 0.605.
const clip = (x, width, tone, label, extra = "") =>
  `<rect x="${x}" y="44" width="${width}" height="40" rx="4" class="ex-${tone}"/><text x="${x + 6}" y="69" class="ex-copy">${label}</text>${extra}`;
const revision = (n, a, b, next) =>
  `<text x="316" y="232" text-anchor="end" class="sc ${next ? "sc-blink" : "sc-fade"} ex-label" style="--a: ${a}; --b: ${b}${next ? `; --c: ${next - 0.02}; --d: ${next}` : ""}">REV ${n}</text>`;
export const timing = {
  height: 424,
  body: `
<rect x="6" y="8" width="328" height="114" rx="5" class="ex-track"/>
<text x="14" y="28" class="ex-label">TIMELINE · SPLIT · MOVE · TRIM · UNDO / REDO</text>
${move(79, 0.32, 0.48, clip(12, 91, "blue", "A / Intro", part("fade", 0.56, 0.7, '<rect x="82" y="45" width="21" height="38" class="ex-trimmed"/><text x="92" y="100" text-anchor="middle" class="ex-label">TRIM</text>')))}
${move(79, 0.32, 0.48, clip(106, 68, "paper", "B / Main") + move(4, 0.14, 0.22, clip(174, 71, "paper", "", part("fade", 0.16, 0.22, '<text x="180" y="69" class="ex-copy">B₂</text>'))) + part("pop", 0.1, 0.14, '<path d="M175 38v52" class="ex-cut"/>'))}
<g class="sc sc-dip" ${win(0.3, 0.36)}><g class="sc sc-undip" ${win(0.44, 0.5)}>${move(-236, 0.32, 0.48, clip(248, 76, "green", "C / Outro"))}</g></g>
${node(12, 142, 146, 84, "Compiled scenes", ["Stable scene", "keys · current", "built source"], "blue")}
${node(182, 142, 146, 100, "timeline.json", ["Order · splits", "Trims · separate", "timing revision"], "green", revision(1, -0.01, 0, 0.22) + revision(2, 0.2, 0.22, 0.5) + revision(3, 0.48, 0.5, 0.7) + revision(4, 0.68, 0.7))}
${node(182, 272, 146, 84, "Preview worker", ["Applies timing", "Same Rust build"], "green", part("pop", 0.84, 0.9, '<path d="m298 284 6 6 13-15" class="sc-check"/>'))}
${path("timing-keys", "M158 184H182", -0.01, 0, "a")}
${path("timing-write", "M255 122V142", -0.01, 0, "a")}
${token("timing-write-m", 0.14, 0.22, chip("blue"))}
${token("timing-write-m", 0.42, 0.5, chip("blue"))}
${token("timing-write-m", 0.62, 0.7, chip("blue"))}
${path("timing-apply", "M255 242V272", 0.72, 0.8, "c")}
${token("timing-apply-m", 0.72, 0.8, chip("green"))}
<text x="12" y="300" class="ex-label">TIMING CHANGE ≠</text><text x="12" y="316" class="ex-label">SOURCE REBUILD</text>
${say(388, [
  [-0.01, 0, "Scenes in code order"],
  [0.1, 0.14, "Split Main in two"],
  [0.3, 0.34, "Move Outro to the front"],
  [0.56, 0.6, "Trim the end of Intro"],
  [0.72, 0.76, "Worker applies it · same build"],
])}`,
};

// Inputs gather at the render job, which encodes until the file is written.
export const exporting = {
  height: 372,
  body: `
${ring(182, 126, 146, 100, 0.44, 0.48, 0.86, 0.9)}
${node(12, 8, 146, 76, "Worker formats", ["Supported", "formats"], "blue")}
${node(182, 8, 146, 76, "Export settings", ["Format · size", "· fps"])}
${node(12, 116, 146, 84, "Frozen project", ["Code revision", "Immutable input"], "paper", '<path d="M134 150h14v11h-14Zm3 0v-4a4 4 0 0 1 8 0v4" class="ex-lock"/>')}
${node(12, 220, 146, 66, "Session timing", ["Current timeline"], "green")}
${node(182, 126, 146, 100, "Render job", ["Worker encoder", "Progress + Cancel"], "green", '<rect x="194" y="202" width="122" height="6" rx="3" class="sc-bar-track"/>' + part("span", 0.48, 0.86, '<rect x="194" y="202" width="122" height="6" rx="3" class="sc-bar"/>'))}
${node(182, 256, 146, 50, "Video file", [], "green", part("pop", 0.92, 0.98, '<path d="m298 270 6 6 13-15" class="sc-check"/>'))}
${path("export-formats", "M158 46H182", 0.04, 0.12, "a")}
${path("export-settings", "M255 84V126", 0.14, 0.26, "b")}
${path("export-frozen", "M158 158H182", 0.24, 0.36, "b")}
${path("export-timing", "M158 253H170V206H182", 0.32, 0.44, "c")}
${path("export-file", "M255 226V256", 0.86, 0.94, "c")}
${token("export-formats-m", 0.04, 0.12, chip("blue"))}
${token("export-settings-m", 0.14, 0.26, chip("blue"))}
${token("export-frozen-m", 0.24, 0.36, chip("pink"))}
${token("export-timing-m", 0.32, 0.44, chip("green"))}
${token("export-file-m", 0.86, 0.94, chip("green"))}
${say(336, [
  [-0.01, 0, "The worker reports its formats"],
  [0.14, 0.18, "Settings pick format · size · fps"],
  [0.24, 0.28, "A frozen project and the session’s timing"],
  [0.48, 0.52, "The worker encoder renders"],
  [0.86, 0.9, "A video file is written"],
])}`,
};

// Click the title, the selection closes on it, its editor key resolves, then the
// optional source anchor joins.
const lit = (cls, a, b, inner) =>
  `<g class="support-part ${cls} sc sc-lit" ${win(a, b)}>${inner}</g>`;
export const context = {
  height: 480,
  body: `
${ring(12, 8, 316, 120, 0.04, 0.08, 0.22, 0.26)}
${ring(12, 152, 316, 104, 0.3, 0.34, 0.56, 0.6)}
${ring(12, 280, 316, 92, 0.7, 0.74, 0.97, 1)}
<g class="support-part support-a"><rect x="12" y="8" width="316" height="120" rx="4" class="support-paper"/><path d="M12 38H328" class="support-line"/><text x="26" y="28" class="support-small">01 / SELECTED FRAME</text><text x="122" y="94" class="support-headline">Hello.</text>${part("pop", 0.1, 0.18, '<path d="M108 76V64h12m94 12V64h-12M108 96v12h12m94-12v12h-12" class="support-selection"/>')}${part("fade", 0.16, 0.22, '<text x="26" y="120" class="support-small">Selected object · headline</text>')}</g>
<g class="sc sc-blink" ${blink(0.02, 0.08, 0.26, 0.3)}>${part("pop", 0.08, 0.12, '<circle cx="168" cy="86" r="11" class="support-click"/>')}${pointer(168, 86)}</g>
${edge("support-context-key-m", "M170 128V152", 0.24, 0.32)}
${token("support-context-key-m", 0.24, 0.32, chip("pink"))}
${lit("support-b", 0.28, 0.34, `<rect x="12" y="152" width="316" height="104" rx="4" class="support-blue"/><text x="26" y="174" class="support-small">02 / OBJECT IDENTITY</text>${part("rise", 0.36, 0.44, '<text x="26" y="200" class="support-code">intro / starter-title</text>')}${part("rise", 0.44, 0.52, '<text x="26" y="222" class="support-code">headline / primary</text>')}<path d="M26 232H314" class="support-line"/><text x="26" y="248" class="support-small">From this frame’s object map</text>`)}
${part("fade", 0.62, 0.7, '<path d="M170 256V280" class="support-connector"/><path d="m-8-4.5 8 4.5-8 4.5" transform="translate(170 280) rotate(90)" class="sc-head sc-quiet"/>')}
${lit("support-c", 0.66, 0.74, '<rect x="12" y="280" width="316" height="92" rx="4" class="support-paper support-optional"/><text x="26" y="302" class="support-small">03 / SOURCE ANCHOR</text><text x="26" y="326" class="support-code">src/lib.rs</text><text x="26" y="346" class="support-code">render_frame</text><text x="26" y="364" class="support-small">Only if registered</text>')}
<text x="12" y="398" class="support-small">SELECTION PROVIDES CONTEXT.</text><text x="12" y="414" class="support-small">IT DOES NOT EDIT SOURCE.</text>
${say(442, [
  [-0.01, 0, "Click an object in the frame"],
  [0.24, 0.28, "Studio resolves its stable editor key"],
  [0.62, 0.66, "A registered anchor can add a source location"],
])}`,
};

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
    return `<g class="sc sc-blink" ${blink(a, a + 0.006, a + 0.026, a + 0.032)}><text x="190" y="72" class="support-number">${String(i + 1).padStart(2, "0")}</text><text x="190" y="98" class="support-small">${name}</text></g>`;
  })
  .join("");
export const broker = {
  height: 394,
  body: `
${ring(176, 8, 152, 150, 0.16, 0.2, 0.5, 0.54)}
<g class="support-part support-a"><rect x="12" y="8" width="130" height="120" rx="4" class="support-paper"/><text x="26" y="32" class="support-small">CODING AGENT</text><path d="M28 54h82m-82 18h61m-61 18h72" class="support-line"/><text x="26" y="114" class="support-code">request</text>${part("pop", 0.84, 0.9, check(104, 102))}</g>
${edge("support-broker-ask-m", "M142 68H176", 0.06, 0.16)}
${token("support-broker-ask-m", 0.06, 0.16, chip("pink"))}
<g class="support-part support-b"><rect x="176" y="8" width="152" height="150" rx="4" class="support-green"/><text x="190" y="32" class="support-small">STUDIO BROKER</text>${toolCycle}${part("fade", 0.49, 0.53, '<text x="190" y="72" class="support-number">09</text><text x="190" y="98" class="support-small">read-only tools</text>')}<path d="M190 116H314" class="support-line"/><text x="190" y="140" class="support-small">No file writes</text></g>
${edge("support-broker-read-m", "M252 158V190", 0.52, 0.6)}
${token("support-broker-read-m", 0.52, 0.6, chip("green"))}
${lit("support-c", 0.56, 0.62, '<rect x="176" y="190" width="152" height="112" rx="4" class="support-blue"/><text x="190" y="212" class="support-small">PROJECT</text><text x="190" y="240" class="support-code">Frames</text><text x="190" y="264" class="support-code">Timing</text><text x="190" y="288" class="support-code">Checks</text>')}
${ring(184, 224, 136, 24, 0.58, 0.62, 0.82, 0.86, 4)}
${edge("support-broker-return-m", "M176 280H77V128", 0.64, 0.82)}
${token("support-broker-return-m", 0.64, 0.82, chip("blue"))}
<text x="12" y="330" class="support-small">PROJECT CONTEXT RETURNS TO THE AGENT</text>
${say(358, [
  [-0.01, 0, "The agent sends a request"],
  [0.2, 0.24, "The broker offers nine read-only tools"],
  [0.52, 0.56, "One call reads project facts or output"],
  [0.64, 0.68, "The answer returns to the agent"],
])}`,
};

// A session rail: accept, start a task, stop, recover. Stops keep the wide figure's
// timing (its rail runs from x 40 to 860).
const reach = x => ((x - 40) / 820) * 0.8;
const railX = x => +(20 + ((x - 40) / 820) * 300).toFixed(1);
const stop = (x, label, mark, below) =>
  part(
    "pop",
    reach(x),
    reach(x) + 0.04,
    `${mark(railX(x))}<text x="${railX(x)}" y="${below ? 64 : 26}" text-anchor="middle" class="support-small">${label}</text>`
  );
export const recovery = {
  height: 504,
  body: `
<path d="M20 44H320" class="support-guide"/><path d="M20 44H320" class="sc sc-draw support-rail" ${win(0, 0.8)} pathLength="1"/>
${stop(150, "ACCEPTED", x => `<circle cx="${x}" cy="44" r="6" class="sc-chip sc-chip-green"/>`)}
${stop(450, "TASK STARTS", x => `<circle cx="${x}" cy="44" r="6" class="sc-chip sc-chip-blue"/>`, true)}
${stop(600, "INTERRUPTION", x => `<path d="m${x - 6} 38 12 12m0-12-12 12" class="sc-cross"/>`)}
${stop(780, "RECOVERY", x => `<circle cx="${x}" cy="44" r="6" class="sc-chip sc-chip-green"/>`, true)}
${ring(12, 84, 316, 92, 0.72, 0.76, 1.1, 1.2)}
${lit("support-a", 0.08, 0.14, `<rect x="12" y="84" width="316" height="92" rx="4" class="support-green"/><text x="26" y="106" class="support-small">ACCEPTED REVISION</text><text x="26" y="140" class="support-headline">Kept.</text>${part("pop", 0.1, 0.14, '<path d="m28 156 7 7 14-17" class="support-check"/>')}<text x="56" y="166" class="support-code">Recovery baseline</text>`)}
${lit("support-b", 0.36, 0.4, `<rect x="12" y="192" width="316" height="100" rx="4" class="support-paper"/><text x="26" y="214" class="support-small">UNFINISHED TASK</text><rect x="26" y="226" width="288" height="6" rx="3" class="sc-bar-track"/><rect x="26" y="226" width="180" height="6" rx="3" class="sc sc-bar" ${win(0.4, 0.55)}/>${part("fade", 0.56, 0.62, '<path d="M28 242h7v22h-7Zm14 0h7v22h-7Z" class="support-pause"/>')}<text x="26" y="282" class="support-code sc sc-blink" ${blink(0.4, 0.44, 0.55, 0.58)}>Editing its draft…</text>${part("fade", 0.58, 0.64, '<text x="26" y="282" class="support-code">Marked interrupted</text>')}`)}
${edge("support-recovery-keep-m", "M170 292V316", 0.74, 0.84)}
${token("support-recovery-keep-m", 0.74, 0.84, chip("blue"))}
${lit("support-c", 0.8, 0.88, '<rect x="12" y="316" width="316" height="80" rx="4" class="support-blue"/><text x="26" y="338" class="support-small">RETAINED DRAFT</text><text x="26" y="362" class="support-code">Draft remains.</text><text x="26" y="384" class="support-small">Available after recovery</text>')}
<text x="12" y="422" class="support-small">ACCEPTED FILES ARE THE BASELINE</text><text x="12" y="438" class="support-small">RETAINED DRAFT ≠ ACCEPTED REVISION</text>
${say(466, [
  [-0.01, 0, "You accept a revision"],
  [0.36, 0.4, "A new task edits its own draft"],
  [0.55, 0.58, "The task is interrupted before it finishes"],
  [0.72, 0.76, "Recovery restores the accepted revision and keeps the draft"],
])}`,
};

// The candidate climbs five stages; how-it-works.js sets --gate-p (0..4) on the figure.
// A stage passes inside --a..--b, the same window as on the wide figure's planes.
const stages = [
  ["Capture twice", "Matching, immutable bytes"],
  ["Build", "Isolated copy · pinned SDK"],
  ["Inspect", "Frames · boundaries · audio"],
  ["Review policy", "Automatic, or manual Apply"],
  ["Accept", "Journaled file-set transaction"],
];
export const gates = {
  height: 300,
  body: `
<g class="accepted-result"><path d="M44 14 50 20 62 6"/><text x="70" y="18">ACCEPTED REVISION</text></g>
<path class="candidate-track" d="M22 282V30"/>
${stages
  .map(([name, sub], i) => {
    const y = 230 - i * 50;
    return `<g class="phone-gate" style="--a: ${Math.max(0, i - 0.18)}; --b: ${i === 4 ? 4 : i + 0.15}"><rect x="44" y="${y}" width="286" height="40" rx="4" class="phone-gate-plane"/><g class="phone-gate-tick"><circle cx="62" cy="${y + 20}" r="9"/><path d="m57 ${y + 20} 3.5 3.5 7-8"/></g><text x="80" y="${y + 17}" class="gate-title">0${i + 1} / ${name}</text><text x="80" y="${y + 33}" class="gate-subtitle">${sub}</text></g>`;
  })
  .join("")}
<g class="phone-candidate"><path d="M16 262v-12l-6 3 12-20 12 20-6-3v12Z"/></g>
<text x="44" y="294" class="gate-base-label">CAPTURED AGENT DRAFT</text><text x="330" y="294" text-anchor="end" class="gate-subtitle">Failure keeps draft</text>`,
};

// The current preview stays while the candidate compiles, prepares and becomes ready;
// only then does it switch. how-it-works.js sets --compile, --prepare, --ready, --handoff.
export const readiness = {
  height: 300,
  body: `
<text x="12" y="16" class="lane-label">CURRENT</text>
<rect x="12" y="24" width="316" height="38" rx="2" class="old-lane"/><text x="24" y="48" class="lane-copy">Working preview remains available</text>
<g class="preview-chip"><rect x="188" y="70" width="140" height="28" rx="3" class="sc-dark"/><text x="258" y="89" text-anchor="middle" class="sc-on-dark preview-old">Hello.</text><text x="258" y="89" text-anchor="middle" class="sc-on-dark preview-new">Make your move.</text></g>
<text x="12" y="128" class="lane-label">CANDIDATE</text><text x="328" y="128" text-anchor="end" class="gate-subtitle">Compile alone is not readiness.</text>
<rect x="12" y="136" width="316" height="122" class="candidate-outline"/>
<rect x="13" y="138" width="314" height="38" class="compile-fill phone-lane phone-compile"/><text x="24" y="162" class="lane-copy">Compile</text>
<rect x="13" y="178" width="314" height="38" class="prepare-fill phone-lane phone-prepare"/><text x="24" y="202" class="lane-copy">Prepare worker, frame, audio</text>
<rect x="13" y="218" width="314" height="38" class="ready-fill phone-lane phone-ready"/><text x="24" y="242" class="lane-copy">Ready to replace</text>
<g class="handoff-marker"><path d="M318 216V104m-6 7 6-7 6 7"/><text x="328" y="280" text-anchor="end" class="lane-label">SWITCH AT THE PLAYHEAD</text><text x="328" y="296" text-anchor="end" class="gate-subtitle">Worker · frame · audio together</text></g>`,
};
