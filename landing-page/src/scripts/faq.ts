export function initFaq(): void {
  const buttons = document.querySelectorAll<HTMLButtonElement>(".qa .q");

  buttons.forEach((btn) => {
    btn.addEventListener("click", () => {
      const qa = btn.parentElement;
      if (!qa) return;
      const wasOpen = qa.classList.contains("open");

      document.querySelectorAll<HTMLElement>(".qa.open").forEach((open) => {
        open.classList.remove("open");
        const q = open.querySelector<HTMLButtonElement>(".q");
        q?.setAttribute("aria-expanded", "false");
      });

      if (!wasOpen) {
        qa.classList.add("open");
        btn.setAttribute("aria-expanded", "true");
      }
    });
  });
}
