// The note crosses from the iPhone to the Mac: readable at both ends,
// scrambled on the way, which is what end-to-end encryption means.
const TRIP_MS = 2800;
const REST_MS = 700;
const SCRAMBLE_MS = 90;
const CLEAR_EDGE = 0.16;
const GLYPHS = "abcdef0123456789#%&*";
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

function scramble(length: number): string {
  let out = "";
  for (let i = 0; i < length; i += 1) {
    out += GLYPHS[Math.floor(Math.random() * GLYPHS.length)];
  }
  return out;
}

function travel(chip: HTMLElement): void {
  const word = chip.dataset.cipher ?? "";
  if (reduced) {
    chip.textContent = scramble(word.length);
    return;
  }
  let start = performance.now();
  let lastScramble = 0;

  const frame = (now: number): void => {
    const elapsed = now - start;
    if (elapsed > TRIP_MS + REST_MS) start = now;
    const progress = Math.min(1, elapsed / TRIP_MS);
    chip.style.left = `${progress * 100}%`;
    const clear = progress < CLEAR_EDGE || progress > 1 - CLEAR_EDGE;
    if (clear) {
      chip.textContent = word;
    } else if (now - lastScramble > SCRAMBLE_MS) {
      chip.textContent = scramble(word.length);
      lastScramble = now;
    }
    requestAnimationFrame(frame);
  };

  requestAnimationFrame(frame);
}

for (const chip of document.querySelectorAll<HTMLElement>("[data-cipher]")) {
  travel(chip);
}

export {};
