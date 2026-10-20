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
  const supporting = [...document.querySelectorAll("[data-scroll-figure]")].map(
    element => ({ element, top: 0, height: 0, last: -1 })
  );
  const motion = window.studioScrollMotion;
  const mobile = matchMedia("(max-width: 760px)");
  const clamp = x => Math.max(0, Math.min(1, x));
  const ramp = (p, a, b) => {
    const t = clamp((p - a) / (b - a));
    return t * t * (3 - 2 * t);
  };
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
      const a = ramp(progress, 0, 0.35),
        b = ramp(progress, 0.2, 0.65),
        c = ramp(progress, 0.5, 0.95);
      item.element.style.setProperty("--support-b-y", `${30 * (1 - b)}px`);
      item.element.style.setProperty("--support-c-y", `${48 * (1 - c)}px`);
      item.element.style.setProperty("--support-travel", `${32 * b}px`);
      item.element.style.setProperty("--support-line", a);
      item.element.style.setProperty("--support-return", c);
      item.element.style.setProperty("--support-packet", a * (1 - c));
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
      planes.forEach((plane, i) => {
        const passed = ramp(p, Math.max(0, i - 0.18), i === 4 ? 4 : i + 0.15);
        plane.style.setProperty("--passed", passed);
        plane.style.setProperty("--plane-alpha", 0.12 + 0.64 * passed);
        plane.style.setProperty(
          "--plane-y",
          `${(i - 2) * 42 * (1 - opened)}px`
        );
        const focus = ramp(p, i - 0.4, i) * (1 - ramp(p, i + 0.2, i + 0.6));
        plane.style.setProperty("--plane-x", `${-38 * focus}px`);
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
      item.element.removeAttribute("style");
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
