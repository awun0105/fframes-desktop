// Follow the OS by default; a deliberate visitor choice overrides it on both pages.
(() => {
  const preference = matchMedia("(prefers-reduced-motion: reduce)");
  const key = "fframes-studio-scroll-motion";
  const listeners = new Set();
  let choice = null;
  try {
    choice = localStorage.getItem(key);
  } catch {
    /* Storage is optional. */
  }
  const enabled = () =>
    choice === "on" || (choice !== "off" && !preference.matches);
  function update() {
    for (const controls of document.querySelectorAll(
      "[data-motion-controls]"
    )) {
      controls.hidden = false;
      const button = controls.querySelector("button");
      button.textContent = enabled()
        ? "Pause scroll motion"
        : "Enable scroll motion";
      button.setAttribute("aria-pressed", String(enabled()));
      controls.querySelector("[role=status]").textContent = enabled()
        ? "Scroll to move the diagram."
        : choice === "off"
          ? "Scroll motion is paused."
          : "Your system has reduced motion enabled.";
    }
    for (const listener of listeners) listener();
  }
  // A scroll scene (scripts/studio-scenes.mjs): play(p) sets the figure's --p and
  // moves each [data-along] token along its connector inside the token's window.
  function scene(element) {
    const tokens = [...element.querySelectorAll("[data-along]")].map(token => {
      const path = element.querySelector(token.dataset.along);
      return {
        token,
        path,
        length: path.getTotalLength(),
        a: +token.dataset.a,
        b: +token.dataset.b,
      };
    });
    return {
      play(p) {
        element.style.setProperty("--p", p.toFixed(4));
        for (const { token, path, length, a, b } of tokens) {
          const k = Math.max(0, Math.min(1, (p - a) / (b - a)));
          const at = path.getPointAtLength(k * length);
          token.setAttribute(
            "transform",
            `translate(${at.x.toFixed(1)} ${at.y.toFixed(1)})`
          );
          token.style.opacity = k > 0 && k < 1 ? 1 : 0;
        }
      },
      reset() {
        element.removeAttribute("style");
        for (const { token } of tokens) token.removeAttribute("style");
      },
    };
  }
  window.studioScrollMotion = {
    get enabled() {
      return enabled();
    },
    subscribe(listener) {
      listeners.add(listener);
    },
    scene,
  };
  for (const button of document.querySelectorAll(
    "[data-motion-controls] button"
  )) {
    button.addEventListener("click", () => {
      choice = enabled() ? "off" : "on";
      try {
        localStorage.setItem(key, choice);
      } catch {
        /* The current tab still works. */
      }
      update();
    });
  }
  preference.addEventListener("change", update);
  addEventListener("storage", event => {
    if (event.key === key) {
      choice = event.newValue;
      update();
    }
  });
  update();
})();
