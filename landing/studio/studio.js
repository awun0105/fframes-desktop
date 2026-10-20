// Copy a command block without its prompt characters
for (const button of document.querySelectorAll(".copy")) {
  button.addEventListener("click", async () => {
    let feedback = button.closest(".card")?.querySelector(".copy-feedback");
    if (!feedback && document.querySelector("[data-story]")) {
      feedback = document.createElement("span");
      feedback.className = "copy-feedback";
      feedback.setAttribute("role", "status");
      button.closest(".command").after(feedback);
    }
    const text = button.previousElementSibling.textContent
      .replace(/^\$ /gm, "")
      .replace(/^PS> /gm, "");
    try {
      await navigator.clipboard.writeText(text);
      window.plausible?.("Copy command", {
        props: { page: location.pathname },
      });
      button.classList.add("copied");
      button.setAttribute("aria-label", "Copied");
      if (feedback) feedback.textContent = "Command copied.";
      setTimeout(() => {
        button.classList.remove("copied");
        button.setAttribute("aria-label", "Copy command");
        if (feedback) feedback.textContent = "";
      }, 1600);
    } catch {
      button.setAttribute(
        "aria-label",
        "Copy failed, select the command instead"
      );
      if (feedback)
        feedback.textContent =
          "Copy unavailable. Select the command to copy it.";
    }
  });
}
