// Every figure property is derived from document position. There is no playback clock.
(() => {
  document.documentElement.classList.add("js-enabled");
  const story = document.querySelector("[data-story]");
  if (!story) return;
  const steps = [...story.querySelectorAll("[data-step]")];
  const links = [...story.querySelectorAll(".step-nav a")];
  const figure = story.querySelector(".workflow-figure");
  const motion = window.studioScrollMotion;
  const features = [...document.querySelectorAll("[data-product-figure]")].map(
    element => ({
      element,
      top: 0,
      height: 0,
      last: -1,
      scene: motion.scene(element),
    })
  );
  // Each chip travels between two planes; its end points come from the drawing.
  const chip = kind => {
    const element = figure.querySelector(`.chip-${kind}`);
    const point = name => element.dataset[name].split(" ").map(Number);
    return { from: point("from"), to: point("to") };
  };
  const chips = {
    context: chip("context"),
    candidate: chip("candidate"),
    result: chip("result"),
  };
  const label = story.querySelector("[data-state-label]");
  const index = story.querySelector(".readout-index");
  const narrow = matchMedia("(max-width: 760px)");
  const clamp = x => Math.min(1, Math.max(0, x));
  const ramp = (p, a, b) => {
    const t = clamp((p - a) / (b - a));
    return t * t * (3 - 2 * t);
  };
  let frame = 0;
  let positions = [];
  let last = -1;
  let lastProgress = "";
  const labels = [
    "Select the title. The source stays unchanged.",
    "The agent edits a draft. Preview stays on Hello.",
    "Build first. Then validate the candidate.",
    "Accepted edit → updated preview.",
  ];
  function measure() {
    const offset = narrow.matches ? figure.offsetHeight + 90 : 120;
    positions = steps.map(
      step => step.getBoundingClientRect().top + scrollY - offset
    );
    for (const item of features) {
      const bounds = item.element.getBoundingClientRect();
      item.top = bounds.top + scrollY;
      item.height = bounds.height;
    }
    schedule();
  }
  function draw() {
    frame = 0;
    if (!motion.enabled) return;
    const y = scrollY;
    // A product scene plays while its figure is fully on screen.
    for (const item of features) {
      const q = clamp(
        (y + innerHeight * 0.88 - item.top) / (item.height + innerHeight * 0.28)
      );
      if (q === item.last) continue;
      item.last = q;
      item.scene.play(clamp((q - 0.1) / 0.68));
    }
    let p = 0;
    for (let i = 0; i < positions.length - 1; i++) {
      if (y >= positions[i])
        p = i + clamp((y - positions[i]) / (positions[i + 1] - positions[i]));
    }
    // Every value holds still near a step position, so a step reads the same
    // whichever way the reader arrives.
    const spread = ramp(p, 0.65, 0.97);
    const arrival = ramp(y, 0, Math.max(1, positions[0]));
    // Once the story is settled, scrolling the remaining page needs no SVG work.
    const progress = `${p}:${arrival}`;
    if (progress === lastProgress) return;
    lastProgress = progress;
    const after = ramp(p, 2.88, 2.97);
    const settle = ramp(p, 2.68, 2.97);
    // Plane offsets in SVG units; the chips travel between the moved planes.
    const previewY = -16 * arrival - 10 * spread + 8 * settle;
    const draftX = -10 * spread + 10 * settle;
    const draftY = 6 * arrival + 4 * spread - 4 * settle;
    const checkY = 8 * arrival + 4 * spread - 4 * settle;
    const shift = {
      preview: [0, previewY],
      draft: [draftX, draftY],
      check: [0, checkY],
    };
    // A chip at k between its start on plane a and its end on plane b.
    const travel = (name, k, a, b) => {
      const { from, to } = chips[name];
      const x0 = from[0] + shift[a][0],
        y0 = from[1] + shift[a][1];
      const x1 = to[0] + shift[b][0],
        y1 = to[1] + shift[b][1];
      return [`${x0 + (x1 - x0) * k}px`, `${y0 + (y1 - y0) * k}px`];
    };
    // How far each chip has travelled; the phone drawing places its chips from these.
    const contextK = ramp(p, 0.15, 0.6);
    const candidateK = ramp(p, 1.08, 1.38);
    const resultK = ramp(p, 2.2, 2.85);
    const [contextX, contextY] = travel(
      "context",
      contextK,
      "preview",
      "draft"
    );
    const [candidateX, candidateY] = travel(
      "candidate",
      candidateK,
      "draft",
      "check"
    );
    const [resultX, resultY] = travel("result", resultK, "check", "preview");
    const click = ramp(p, 0, 0.3);
    const prompt = ramp(p, 0.45, 0.75);
    const values = {
      "preview-y": `${previewY}px`,
      "draft-x": `${draftX}px`,
      "draft-y": `${draftY}px`,
      "check-y": `${checkY}px`,
      selection: 1 - ramp(p, 0.35, 0.9),
      "selection-x": `${8 * (1 - ramp(p, 0, 0.35))}px`,
      click: Math.sin(Math.PI * click),
      "click-scale": 0.6 + click,
      "context-opacity": ramp(p, 0.1, 0.2) * (1 - ramp(p, 0.6, 0.7)),
      "context-x": contextX,
      "context-y": contextY,
      "context-k": contextK,
      key: ramp(p, 0.55, 0.65),
      prompt,
      "prompt-x": `${-12 * (1 - prompt)}px`,
      strike: ramp(p, 0.7, 0.85),
      edit: ramp(p, 0.8, 1),
      "candidate-opacity": ramp(p, 1.03, 1.12) * (1 - ramp(p, 1.38, 1.48)),
      "candidate-x": candidateX,
      "candidate-y": candidateY,
      "candidate-k": candidateK,
      "build-run": ramp(p, 1.4, 1.65),
      build: ramp(p, 1.6, 1.7),
      "inspect-run": ramp(p, 1.72, 1.97),
      inspect: ramp(p, 2.03, 2.12),
      "result-opacity": ramp(p, 2.15, 2.25) * (1 - ramp(p, 2.85, 2.95)),
      "result-x": resultX,
      "result-y": resultY,
      "result-k": resultK,
      before: 1 - after,
      after,
    };
    for (const [key, value] of Object.entries(values))
      figure.style.setProperty(`--${key}`, value);
    const current = p < 0.65 ? 0 : p < 1.35 ? 1 : p < 2.95 ? 2 : 3;
    if (current !== last) {
      label.textContent = labels[current];
      index.textContent = `0${current + 1} / 04`;
      links.forEach((link, i) =>
        i === current
          ? link.setAttribute("aria-current", "step")
          : link.removeAttribute("aria-current")
      );
      last = current;
    }
  }
  function schedule() {
    if (!frame && motion.enabled) frame = requestAnimationFrame(draw);
  }
  function configure() {
    cancelAnimationFrame(frame);
    frame = 0;
    last = -1;
    lastProgress = "";
    const enabled = motion.enabled;
    document.documentElement.classList.toggle("story-motion", enabled);
    figure.removeAttribute("style");
    for (const item of features) {
      item.last = -1;
      item.scene.reset();
    }
    if (!enabled) {
      label.textContent = "Selection → draft → checks → preview";
      index.textContent = "01—04";
      links.forEach(link => link.removeAttribute("aria-current"));
    }
    measure();
  }
  addEventListener("scroll", schedule, { passive: true });
  addEventListener("resize", measure);
  addEventListener("pageshow", measure);
  motion.subscribe(configure);
  narrow.addEventListener("change", configure);
  document.fonts?.ready.then(measure);
  configure();
})();
