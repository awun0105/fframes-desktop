// Scroll-derived state, independent of animation-timeline support. No play-once motion.
(() => {
  const transaction = document.querySelector("[data-transaction]");
  if (!transaction) return;
  const figure = transaction.querySelector(".transaction-figure");
  const steps = [...transaction.querySelectorAll("[data-gate-step]")];
  const planes = [...figure.querySelectorAll("[data-plane]")];
  const links = [...figure.querySelectorAll(".gate-nav a")];
  const label = figure.querySelector("[data-gate-label]");
  const index = figure.querySelector(".readout-index");
  const ready = document.querySelector("[data-readiness]");
  const motion = window.studioScrollMotion;
  const supporting = [
    ...document.querySelectorAll("[data-scroll-figure], [data-explanation]"),
  ].map(element => ({
    element,
    top: 0,
    height: 0,
    last: -1,
    frame: -1,
    scene: motion.scene(element),
  }));
  const mobile = matchMedia("(max-width: 760px)");
  const clamp = x => Math.max(0, Math.min(1, x));
  const ramp = (p, a, b) => {
    const t = clamp((p - a) / (b - a));
    return t * t * (3 - 2 * t);
  };
  // The rendering figure steps through a 3 s, 30 fps clip one frame at a time;
  // x follows slide = 40 → 220 over 0..3 s with an ease-in-out curve.
  const FRAMES = 90;
  const easeInOut = u =>
    u < 0.5 ? 4 * u * u * u : 1 - Math.pow(2 - 2 * u, 3) / 2;
  const slide = n => 180 * easeInOut(clamp(n / 30 / 3));
  function renderText(item, n) {
    if (n === item.frame) return;
    item.frame = n;
    const out = (name, value) => {
      for (const element of item.element.querySelectorAll(
        `[data-render-${name}]`
      ))
        element.textContent = value;
    };
    out("frame", n);
    out("time", (n / 30).toFixed(2));
    out("x", (40 + slide(n)).toFixed(1));
    out("count", n + 1);
  }
  function renderAt(item, p) {
    const f = p * (FRAMES - 1),
      n = Math.floor(f),
      style = item.element.style;
    style.setProperty("--render-x", `${slide(n)}px`);
    style.setProperty("--render-x-near", `${slide(Math.max(0, n - 3))}px`);
    style.setProperty("--render-x-far", `${slide(Math.max(0, n - 6))}px`);
    // Within a frame, the scan sweeps the canvas and the rebuilt node flashes.
    style.setProperty("--render-scan", `${273 * (f - n)}px`);
    style.setProperty("--render-tick", n === FRAMES - 1 ? 0 : 1 - (f - n));
    style.setProperty("--render-progress", n / (FRAMES - 1));
    style.setProperty("--render-ease", easeInOut(n / 30 / 3));
    style.setProperty("--render-count", 1 + (11 * n) / (FRAMES - 1));
    renderText(item, n);
  }
  const labels = [
    "Capture twice. Both copies must match.",
    "Build the captured candidate with its SDK.",
    "Inspect frames and audio; report coverage gaps.",
    "Automatic apply, or wait for manual review.",
    "Publish the accepted file set.",
  ];
  let positions = [],
    transactionTop = 0,
    readyTop = 0,
    readyHeight = 0,
    frame = 0,
    last = -1;
  let lastProgress = "",
    lastReady = -1;
  function measure() {
    const offset = mobile.matches ? figure.offsetHeight + 90 : 120;
    transactionTop = transaction.getBoundingClientRect().top + scrollY;
    positions = steps.map(
      s => s.getBoundingClientRect().top + scrollY - offset
    );
    const bounds = ready
      .querySelector(".readiness-figure")
      .getBoundingClientRect();
    readyTop = bounds.top + scrollY;
    readyHeight = bounds.height;
    for (const item of supporting) {
      const bounds = item.element.getBoundingClientRect();
      item.top = bounds.top + scrollY;
      item.height = bounds.height;
    }
    schedule();
  }
  function draw() {
    frame = 0;
    if (!motion.enabled) return;
    for (const item of supporting) {
      const progress = clamp(
        (scrollY + innerHeight * 0.88 - item.top) /
          (item.height + innerHeight * 0.28)
      );
      if (progress === item.last) continue;
      item.last = progress;
      if (item.element.dataset.scrollFigure === "renderer") {
        renderAt(item, clamp((progress - 0.08) / 0.8));
        continue;
      }
      // A scene plays while its figure is fully on screen. Nodes and connectors
      // keep fixed geometry; only signals travel along them.
      item.scene.play(clamp((progress - 0.1) / 0.68));
    }
    let p = 0;
    for (let i = 0; i < positions.length - 1; i++)
      if (scrollY >= positions[i])
        p =
          i +
          clamp((scrollY - positions[i]) / (positions[i + 1] - positions[i]));
    const opened = ramp(
      scrollY + innerHeight * 0.85,
      transactionTop,
      transactionTop + innerHeight * 0.5
    );
    // The two diagrams have independent ranges. Do not invalidate an entire SVG
    // while the reader is scrolling a different section.
    const progress = `${p}:${opened}`;
    if (progress !== lastProgress) {
      lastProgress = progress;
      figure.style.setProperty("--candidate-y", `${-26 - 67 * p}px`);
      figure.style.setProperty("--candidate-alpha", 1 - ramp(p, 3.8, 4));
      figure.style.setProperty("--accepted-alpha", ramp(p, 3.7, 4));
      // The phone drawing places its candidate and stages from p itself.
      figure.style.setProperty("--gate-p", p);
      planes.forEach((plane, i) => {
        const passed = ramp(p, Math.max(0, i - 0.18), i === 4 ? 4 : i + 0.15);
        plane.style.setProperty("--passed", passed);
        plane.style.setProperty("--plane-alpha", 0.12 + 0.64 * passed);
        plane.style.setProperty(
          "--plane-y",
          `${(i - 2) * 42 * (1 - opened)}px`
        );
        plane.style.setProperty("--plane-x", "0px");
      });
      const current = Math.min(4, Math.floor(p + 0.12));
      if (current !== last) {
        label.textContent = labels[current];
        index.textContent = `0${current + 1} / 05`;
        links.forEach((a, i) =>
          i === current
            ? a.setAttribute("aria-current", "step")
            : a.removeAttribute("aria-current")
        );
        last = current;
      }
    }
    const q = clamp(
      (scrollY + innerHeight * 0.8 - readyTop) /
        (readyHeight + innerHeight * 0.1)
    );
    if (q === lastReady) return;
    lastReady = q;
    ready.style.setProperty("--compile", ramp(q, 0, 0.32));
    ready.style.setProperty("--prepare", ramp(q, 0.32, 0.66));
    ready.style.setProperty("--ready", ramp(q, 0.66, 0.87));
    ready.style.setProperty("--handoff", ramp(q, 0.87, 1));
    ready.style.setProperty("--marker-x", `${712 * q}px`);
  }
  function schedule() {
    if (!frame && motion.enabled) frame = requestAnimationFrame(draw);
  }
  function configure() {
    cancelAnimationFrame(frame);
    frame = 0;
    last = -1;
    lastProgress = "";
    lastReady = -1;
    for (const item of supporting) {
      item.last = -1;
      item.scene.reset();
      // Inline styles fall back to the CSS end state; match the readouts to it.
      if (item.element.dataset.scrollFigure === "renderer")
        renderText(item, FRAMES - 1);
    }
    document.documentElement.classList.toggle(
      "internals-motion",
      motion.enabled
    );
    figure.removeAttribute("style");
    ready.removeAttribute("style");
    planes.forEach(p => p.removeAttribute("style"));
    if (!motion.enabled) {
      label.textContent = "Capture → build → inspect → review → accept";
      index.textContent = "01—05";
      links.forEach(a => a.removeAttribute("aria-current"));
    }
    measure();
  }
  addEventListener("scroll", schedule, { passive: true });
  addEventListener("resize", measure);
  addEventListener("pageshow", measure);
  motion.subscribe(configure);
  mobile.addEventListener("change", configure);
  document.fonts?.ready.then(measure);
  configure();
})();
