// A dictated sentence appears word by word, holds, then starts again.
const WORD_MS = 380;
const HOLD_STEPS = 4;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

function type(sentence: HTMLElement): void {
  const words = [...sentence.querySelectorAll<HTMLElement>("span")];
  if (reduced) {
    for (const word of words) word.classList.add("on");
    return;
  }
  let at = 0;
  setInterval(() => {
    const shown = at % (words.length + HOLD_STEPS);
    for (const [i, word] of words.entries()) {
      word.classList.toggle("on", i <= shown);
    }
    at += 1;
  }, WORD_MS);
}

function initTyped(): void {
  const seen = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      seen.unobserve(entry.target);
      type(entry.target as HTMLElement);
    }
  });
  for (const sentence of document.querySelectorAll<HTMLElement>(
    "[data-typed]",
  )) {
    seen.observe(sentence);
  }
}

initTyped();

export {};
