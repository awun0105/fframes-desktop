// Every figure property is derived from document position. There is no playback clock.
(() => {
  document.documentElement.classList.add("js-enabled");
  const story = document.querySelector("[data-story]");
  if (!story) return;
  const steps = [...story.querySelectorAll("[data-step]")];
  const links = [...story.querySelectorAll(".step-nav a")];
  const figure = story.querySelector(".workflow-figure");
  const features = [...document.querySelectorAll("[data-product-figure]")].map(
    element => ({ element, top: 0, height: 0, last: -1 })
  );
  const label = story.querySelector("[data-state-label]");
  const index = story.querySelector(".readout-index");
  const motion = window.studioScrollMotion;
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
    for (const item of features) {
      const q = clamp(
        (y + innerHeight * 0.88 - item.top) / (item.height + innerHeight * 0.18)
      );
      if (q === item.last) continue;
      item.last = q;
      const first = ramp(q, 0, 0.32),
        middle = ramp(q, 0.22, 0.62),
        end = ramp(q, 0.6, 0.95);
      const reorder = ramp(q, 0.35, 0.78);
      const values = {
        "file-one": `${30 * (1 - first)}px`,
        "file-two": `${24 * (1 - middle)}px`,
        "file-three": `${18 * (1 - end)}px`,
        packet: first * (1 - end),
        travel: `${24 * middle}px`,
        ready: 0.25 + 0.75 * end,
        reorder: `${180 * reorder}px`,
        "outro-x": `${-360 * reorder}px`,
        "outro-y": `${-42 * 4 * reorder * (1 - reorder)}px`,
        split: `${6 * ramp(q, 0.05, 0.3)}px`,
        "split-alpha": ramp(q, 0.05, 0.3),
        trim: end,
        "trim-x": `${24 * end}px`,
        "tool-one": `${25 * first}px`,
        "tool-two": `${25 * middle}px`,
        render: ramp(q, 0.1, 0.9),
      };
      for (const [key, value] of Object.entries(values))
        item.element.style.setProperty(`--feature-${key}`, value);
    }
    let p = 0;
    for (let i = 0; i < positions.length - 1; i++) {
      if (y >= positions[i])
        p = i + clamp((y - positions[i]) / (positions[i + 1] - positions[i]));
    }
    const edit = ramp(p, 0.65, 1);
    const arrival = ramp(y, 0, Math.max(1, positions[0]));
    // Once the story is settled, scrolling the remaining page needs no SVG work.
    const progress = `${p}:${arrival}`;
    if (progress === lastProgress) return;
    lastProgress = progress;
    const build = ramp(p, 1.35, 1.7);
    const inspect = ramp(p, 1.8, 2.15);
    const after = ramp(p, 2.9, 3);
    const settle = ramp(p, 2.7, 3);
    const values = {
      "preview-y": `${-32 * arrival - 18 * edit + 12 * settle}px`,
      "draft-x": `${-16 * edit + 16 * settle}px`,
      "draft-y": `${10 * arrival + 8 * edit - 8 * settle}px`,
      "check-y": `${16 * arrival + 8 * edit - 8 * settle}px`,
      selection: 1 - ramp(p, 0.35, 0.9),
      "selection-x": `${8 * (1 - ramp(p, 0, 0.35))}px`,
      "context-opacity": ramp(p, 0.1, 0.3) * (1 - ramp(p, 0.85, 1)),
      "context-y": `${80 * ramp(p, 0.1, 0.9)}px`,
      edit,
      prompt: edit,
      "prompt-x": `${-12 * (1 - edit)}px`,
      "candidate-opacity": ramp(p, 1.05, 1.2) * (1 - ramp(p, 1.8, 2)),
      "candidate-y": `${60 * ramp(p, 1.1, 1.9)}px`,
      build,
      inspect,
      "result-opacity": ramp(p, 2.15, 2.3) * (1 - after),
      "result-y": `${210 * (1 - ramp(p, 2.25, 2.9))}px`,
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
      item.element.removeAttribute("style");
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
