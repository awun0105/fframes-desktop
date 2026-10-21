// Small-screen compositions share the same facts as the wide SVGs, but reflow
// as real text. Keep these outside SVG coordinates so zoom and wrapping work.
const card = (title, detail, tone = "") =>
  `<div class="compact-card ${tone}"><strong>${title}</strong><div class="compact-card-detail">${detail}</div></div>`;
const arrow = '<span class="compact-arrow" aria-hidden="true">↓</span>';
const flow = (...cards) => cards.join(arrow);
const chips = (...labels) =>
  `<div class="compact-chips">${labels.map(label => `<span>${label}</span>`).join("")}</div>`;
const diagrams = {
  workflow: flow(
    card(
      "01 / Preview",
      '<span class="compact-preview-title"><span class="compact-before">Hello.</span><span class="compact-after">Make your move.</span></span><small>Select the title</small>',
      "compact-preview"
    ),
    card(
      "02 / Agent draft",
      '<code>src/lib.rs</code><small>Edits a separate working copy</small><span class="compact-edit">+ “Make your move.”</span>',
      "compact-draft"
    ),
    card(
      "03 / Checks",
      '<span class="compact-check-build">Build ✓</span> → <span class="compact-check-inspect">Inspect ✓</span><small>Accepted edit updates the preview</small>',
      "compact-checks"
    )
  ),
  project: flow(
    card(
      "Your project files",
      "<code>src/lib.rs</code><code>media/</code><code>timeline.json</code>"
    ),
    card(
      "Build & prepare",
      "Compatible project · successful build",
      "compact-blue"
    ),
    card("Studio preview", "Your source, rendered as a frame", "compact-green")
  ),
  timing: flow(
    card("Original scenes", chips("A / Intro", "B / Main", "C / Outro")),
    card(
      "Split → reorder → trim",
      chips("C / Outro", "A / Intro · trimmed", "B₁", "B₂"),
      "compact-blue"
    ),
    card(
      "timeline.json updated",
      "Rust source stays unchanged. No rebuild.",
      "compact-green"
    )
  ),
  sdk: flow(
    card(
      "01 / Install the SDK",
      "Rust · fframes · FFmpeg · Skia",
      "compact-blue"
    ),
    card("02 / Build an example", "Use the managed toolchain"),
    card(
      "03 / Prepare preview",
      "Worker + first frame + audio",
      "compact-green"
    )
  ),
  export: flow(
    card("Your project", "Editable source + media + timing"),
    card(
      "Render frame by frame",
      chips("Frame 001", "Frame 002", "Frame 003"),
      "compact-blue"
    ),
    card(
      "Save a video file",
      "Choose a worker-supported format",
      "compact-green"
    )
  ),
  process: flow(
    card(
      "Studio application",
      "UI + engine state + agent workflow<small>Shared builds · journal + store · read-only broker</small>"
    ),
    '<div class="compact-parallel">' +
      card(
        "Coding agent",
        "Separate process<small>Edits its own draft copy</small>",
        "compact-blue"
      ) +
      card(
        "Preview worker",
        "Separate process<small>Your compiled video: frames, timing & audio</small>",
        "compact-green"
      ) +
      "</div>"
  ),
  gates:
    flow(
      card("01 / Capture twice", "Both immutable copies must match"),
      card("02 / Build", "Isolated copy · pinned SDK", "compact-blue"),
      card("03 / Inspect", "Frames · boundaries · audio"),
      card("04 / Review policy", "Automatic, or wait for manual Apply"),
      card("05 / Accept", "Journaled file-set publication", "compact-green")
    ) +
    '<p class="compact-note">Failure retains the draft. Review is optional.</p>',
  capture: flow(
    card("Writer quiescent", "Draft is ready for capture"),
    card(
      "Capture A + Capture B",
      "Compare their captured bytes",
      "compact-blue"
    ),
    '<div class="compact-parallel">' +
      card("Same bytes", "Proceed to validation", "compact-green") +
      card("Different bytes", "Refuse capture", "compact-pink") +
      "</div>"
  ),
  review:
    flow(
      card(
        "Validated candidate",
        "Checks completed for this candidate",
        "compact-blue"
      ),
      '<div class="compact-parallel">' +
        card("Automatic", "Default policy") +
        card("Manual", "Wait for Apply") +
        "</div>",
      card("Accepted", "Guarded file-set publication", "compact-green")
    ) +
    '<p class="compact-note">If the selected subset changes, validate that subset again.</p>',
  context:
    flow(
      card(
        "01 / Selected frame",
        "Hello.<small>Selected object: headline</small>",
        "compact-pink"
      ),
      card(
        "02 / Object identity",
        "<code>intro / starter-title</code><code>headline / primary</code><small>From this frame’s object map</small>"
      ),
      card(
        "03 / Source anchor",
        "<code>src/lib.rs</code><code>render_frame</code><small>Only if registered</small>",
        "compact-blue"
      )
    ) +
    '<p class="compact-note">Selection supplies context. It does not edit source.</p>',
  broker: flow(
    card("Coding agent", "Requests project information"),
    card("Studio broker", "9 read-only tools · no file writes", "compact-blue"),
    card(
      "Project facts",
      "Frames · timing · checks<small>Context returns to the agent ↑</small>",
      "compact-green"
    )
  ),
  readiness:
    card(
      "Current preview",
      "Remains available while the candidate prepares",
      "compact-green"
    ) +
    flow(
      card("Candidate / Compile", "Build the new revision"),
      card("Prepare", "Worker + first frame + audio", "compact-blue"),
      card(
        "Ready → switch at the playhead",
        "Worker, frame and audio switch together",
        "compact-green"
      )
    ) +
    '<p class="compact-note">Compile alone does not mean ready.</p>',
  containment:
    flow(
      card(
        "Observed writer process tree",
        "Managed agent + child processes<small>For example: node or cargo</small>",
        "compact-blue"
      ),
      card(
        "Observed writer escape",
        "A process leaves the tracked tree",
        "compact-pink"
      ),
      card("Candidate refused", "Capture does not proceed")
    ) +
    '<p class="compact-note">Windows: job object. Linux/macOS: process group + checks.</p>',
  recovery:
    '<div class="compact-parallel">' +
    card(
      "Accepted revision",
      "Kept as the recovery baseline",
      "compact-green"
    ) +
    card(
      "Unfinished task",
      "Marked interrupted<small>The draft remains available after recovery</small>",
      "compact-blue"
    ) +
    '</div><p class="compact-note">Retained draft ≠ accepted revision.</p>',
  "timing-detail": flow(
    card("Compiled scenes", "Stable scene keys · current built source"),
    card(
      "Timeline gesture",
      "Split · trim · move<small>Undo / redo</small>",
      "compact-blue"
    ),
    card(
      "timeline.json",
      "Order · splits · trims<small>Separate timing revision</small>"
    ),
    card(
      "Preview worker",
      "Applies timing using the same Rust build",
      "compact-green"
    )
  ),
  "export-detail": flow(
    card("Export settings", "Worker-supported format · size · fps"),
    '<div class="compact-parallel">' +
      card("Frozen project", "Selected code revision") +
      card("Session timing", "Current timeline") +
      "</div>",
    card("Render job", "Worker encoder · progress + Cancel", "compact-blue"),
    card("Video file", "Saved render output", "compact-green")
  ),
};

export function mobileDiagrams(page) {
  // Safe to rerun after any individual figure generator.
  page = page.replace(
    /<!-- mobile-diagram -->[\s\S]*?<!-- \/mobile-diagram -->\s*/g,
    ""
  );
  return page.replace(/<figure\b[\s\S]*?<\/figure>/g, figure => {
    let key = figure.match(/data-product-figure="([^"]+)"/)?.[1];
    const explanation = figure.match(/data-explanation="([^"]+)"/)?.[1];
    if (explanation)
      key = ["timing", "export"].includes(explanation)
        ? `${explanation}-detail`
        : explanation;
    if (figure.includes('class="workflow-figure"')) key = "workflow";
    if (figure.includes('class="transaction-figure"')) key = "gates";
    for (const name of ["context", "broker", "recovery"])
      if (figure.includes(`support-${name}`)) key = name;
    if (figure.includes("readiness-figure")) key = "readiness";
    if (!diagrams[key]) return figure;
    figure = figure.replace(/; scroll horizontally on small screens/g, "");
    figure = figure.replace(
      /<p class="(?:product-figure-hint|diagram-hint)">[\s\S]*?<\/p>\s*/g,
      ""
    );
    // Put the mobile composition outside all generated SVG markers and wrappers.
    const compact = `<!-- mobile-diagram --><div class="compact-diagram" data-compact="${key}" role="group" aria-label="${key === "workflow" ? "Anatomy of an edit" : "Diagram"}">${diagrams[key]}</div><!-- /mobile-diagram -->`;
    return figure.replace(/<figcaption\b/, `${compact}<figcaption`);
  });
}
