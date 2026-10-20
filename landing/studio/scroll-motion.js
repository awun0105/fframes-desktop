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
  window.studioScrollMotion = {
    get enabled() {
      return enabled();
    },
    subscribe(listener) {
      listeners.add(listener);
    },
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
