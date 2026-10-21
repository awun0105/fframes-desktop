// Code-native illustrations for the technical Studio page. No raster assets/dependencies.
import { readFileSync, writeFileSync } from "node:fs";
import { mobileDiagrams } from "./studio-mobile-diagrams.mjs";
const file = new URL("../landing/studio/how-it-works.html", import.meta.url);
const stages = [
  ["Capture twice", "Matching, immutable bytes"],
  ["Build", "Isolated copy · pinned SDK"],
  ["Inspect", "Frames · boundaries · audio"],
  ["Review policy", "Automatic, or manual Apply"],
  ["Accept", "Journaled file-set transaction"],
];
const gates = `<svg class="gate-diagram" viewBox="0 0 700 620" role="img" aria-labelledby="gate-t gate-d">
<title id="gate-t">From captured draft to accepted revision</title><desc id="gate-d">The candidate rises through capture, build, inspection, review policy and acceptance. Manual review is optional. Failure retains a draft; successful acceptance publishes a journaled file-set transaction. This is an architecture illustration.</desc>
<g class="gate-guides"><path d="M175 175v300M305 250v300M435 175v300"/></g>
${stages
  .map(([name, sub], i) => {
    const y = 410 - i * 67;
    return `<g class="check-layer" data-plane="${i}"><g class="plane-highlight" data-highlight="${i}"><path d="M175 ${y} 305 ${y - 75} 435 ${y} 305 ${y + 75}Z"/></g><path class="slab-side" d="M175 ${y}v7l130 75 130-75v-7l-130 75Z"/><path class="slab-top" d="M175 ${y} 305 ${y - 75} 435 ${y} 305 ${y + 75}Z"/><path class="leader" d="M443 ${y}h29"/><text x="484" y="${y - 5}" class="gate-title">0${i + 1} / ${name}</text><text x="484" y="${y + 13}" class="gate-subtitle">${sub}</text><g class="gate-tick" data-tick="${i}"><circle cx="305" cy="${y}" r="12"/><path d="m299 ${y} 4 4 9-10"/></g></g>`;
  })
  .join("")}
<path class="candidate-track" d="M305 507V78"/>
<g class="travelling-candidate"><path d="M295 506v-37l-12 7 22-40 22 18-12 7v37Z"/><path d="m300 472 5-3 5 3" fill="none" stroke="#407eaf"/></g>
<g class="accepted-result"><path d="M293 78 302 87 320 61"/><text x="305" y="42" text-anchor="middle">ACCEPTED REVISION</text></g>
<text x="305" y="550" text-anchor="middle" class="gate-base-label">CAPTURED AGENT DRAFT</text>
<path d="M154 276H91" class="repair-leader"/><text x="83" y="273" text-anchor="end" class="gate-subtitle">Failure</text><text x="83" y="290" text-anchor="end" class="gate-subtitle">retains draft</text>
<text x="305" y="595" text-anchor="middle" class="gate-subtitle">STAGES, NOT ELAPSED TIME · REVIEW IS OPTIONAL</text>
</svg>`;
const readiness = `<p class="diagram-hint">Scroll the diagram sideways <span aria-hidden="true">↔</span></p><svg class="readiness-diagram" viewBox="0 0 900 280" role="img" aria-labelledby="ready-t ready-d"><title id="ready-t">Prepare a revision before switching the preview</title><desc id="ready-d">The current revision remains available while the candidate compiles and prepares its worker, frame and audio. Only once ready can the revision replace it at the playhead. Diagram spacing is illustrative, not elapsed build time.</desc>
<text x="20" y="64" class="lane-label">CURRENT</text><text x="20" y="158" class="lane-label">CANDIDATE</text>
<rect x="152" y="39" width="712" height="42" rx="2" class="old-lane"/><text x="170" y="65" class="lane-copy">Working preview remains available</text>
<path d="M152 133H864v42H152Z" class="candidate-outline"/>
<g class="compile-lane"><rect x="152" y="133" width="233" height="42" class="compile-fill"/></g><text x="168" y="159" class="lane-copy">Compile</text>
<g class="prepare-lane"><rect x="385" y="133" width="255" height="42" class="prepare-fill"/></g><text x="401" y="159" class="lane-copy">Prepare worker, frame, audio</text>
<g class="ready-lane"><rect x="640" y="133" width="224" height="42" class="ready-fill"/></g><text x="660" y="159" class="lane-copy">Ready to replace</text>
<g class="readiness-marker"><path d="M152 22V190"/><path d="m146 22 6 8 6-8Z"/></g>
<g class="handoff-marker"><path d="M864 82v43m-6-7 6 7 6-7"/><text x="864" y="222" text-anchor="end" class="lane-label">SWITCH AT THE PLAYHEAD</text><text x="864" y="243" text-anchor="end" class="gate-subtitle">Worker · frame · audio together</text></g>
<text x="152" y="243" class="gate-subtitle">Compile alone is not readiness.</text></svg>`;
let html = readFileSync(file, "utf8");
for (const [name, drawing] of Object.entries({ gates, readiness })) {
  html = html.replace(
    new RegExp(
      `<!-- internals:${name} -->[\\s\\S]*?<!-- /internals:${name} -->`
    ),
    `<!-- internals:${name} -->\n${drawing}\n<!-- /internals:${name} -->`
  );
}
writeFileSync(file, mobileDiagrams(html));
