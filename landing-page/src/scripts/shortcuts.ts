// The keyboard card presses one shortcut at a time and the small Mac window
// shows what it does. It starts once the card is in view, so it opens on the
// first shortcut.
const KEY_MS = 1500;

function play(card: HTMLElement): void {
  const keys = [...card.querySelectorAll<HTMLElement>("kbd[data-label]")];
  const label = card.querySelector<HTMLElement>("[data-shortcut-label]");
  const screen = card.querySelector<HTMLElement>("[data-shortcut-card]");
  if (!keys.length || !label) return;

  let at = 0;
  const step = (): void => {
    keys.forEach((key, i) => key.classList.toggle("on", i === at));
    label.textContent = keys[at]?.dataset.label ?? "";
    if (screen) {
      screen.classList.remove("pop");
      void screen.offsetWidth;
      screen.classList.add("pop");
    }
    at = (at + 1) % keys.length;
  };

  step();
  setInterval(step, KEY_MS);
}

function initShortcuts(): void {
  const cards = document.querySelectorAll<HTMLElement>("[data-shortcuts]");
  const seen = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      seen.unobserve(entry.target);
      play(entry.target as HTMLElement);
    }
  });
  for (const card of cards) seen.observe(card);
}

initShortcuts();

export {};
