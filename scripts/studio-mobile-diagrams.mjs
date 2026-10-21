// Small screens get their own portrait drawing of each figure (scripts/studio-phone-*.mjs):
// the same facts and scroll story as the wide SVG, laid out 340 units wide so its text
// stays readable at phone size.
import * as home from "./studio-phone-home.mjs";
import * as internals from "./studio-phone-internals.mjs";

const phones = {
  workflow: home.workflow,
  project: home.project,
  timing: home.timing,
  sdk: home.sdk,
  export: home.exporting,
  process: internals.process,
  gates: internals.gates,
  capture: internals.capture,
  review: internals.review,
  context: internals.context,
  broker: internals.broker,
  readiness: internals.readiness,
  containment: internals.containment,
  recovery: internals.recovery,
  "timing-detail": internals.timing,
  "export-detail": internals.exporting,
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
    const phone = phones[key];
    if (!phone) return figure;
    figure = figure.replace(/; scroll horizontally on small screens/g, "");
    figure = figure.replace(
      /<p class="(?:product-figure-hint|diagram-hint)">[\s\S]*?<\/p>\s*/g,
      ""
    );
    // The wide drawing's title and description describe the phone drawing too.
    // The phone drawing is named by that title, so each figure keeps a single <title>.
    const [, titleId] = figure.match(/<title\b[^>]*\bid="([^"]+)"/) ?? [];
    const [, desc] = figure.match(/<desc\b[^>]*>([\s\S]*?)<\/desc>/) ?? [];
    // Put the phone drawing outside all generated SVG markers and wrappers.
    const drawing = `<!-- mobile-diagram --><div class="phone-figure" data-phone="${key}"><svg viewBox="0 0 340 ${phone.height}" role="img" aria-labelledby="${titleId}" aria-describedby="phone-${key}-d"><desc id="phone-${key}-d">${desc}</desc>${phone.body}</svg></div><!-- /mobile-diagram -->`;
    return figure.replace(/<figcaption\b/, `${drawing}<figcaption`);
  });
}
