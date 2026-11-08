const MAX_TRAVEL = 0.22;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

function initEyes(): void {
  const mascot = document.querySelector<HTMLElement>("[data-eyes]");
  if (!mascot || reduced) return;
  const pupils = mascot.querySelectorAll<HTMLElement>(".eye b");

  addEventListener(
    "pointermove",
    (event) => {
      const box = mascot.getBoundingClientRect();
      const dx = event.clientX - (box.left + box.width / 2);
      const dy = event.clientY - (box.top + box.height / 2);
      const length = Math.hypot(dx, dy) || 1;
      const reach = Math.min(1, length / 300) * MAX_TRAVEL;
      for (const pupil of pupils) {
        const size = pupil.offsetWidth * 2;
        pupil.style.setProperty("--x", `${(dx / length) * reach * size}px`);
        pupil.style.setProperty("--y", `${(dy / length) * reach * size}px`);
      }
    },
    { passive: true },
  );
}

initEyes();

export {};
