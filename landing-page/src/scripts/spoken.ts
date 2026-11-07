const STEP_MS = 420;
const PAUSE_STEPS = 3;
const START_SECONDS = 14;

function initSpoken(): void {
  const sentence = document.querySelector<HTMLElement>("[data-spoken]");
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (!sentence || reduced) return;

  const words = [...sentence.querySelectorAll("span")];
  const clock = document.querySelector<HTMLElement>("[data-voice-clock]");
  let step = 0;

  setInterval(() => {
    const at = step % (words.length + PAUSE_STEPS);
    words.forEach((word, i) => {
      word.classList.toggle("now", i === at);
    });
    if (clock) {
      const seconds = START_SECONDS + Math.floor((at * STEP_MS) / 1000);
      clock.textContent = `0:${seconds}`;
    }
    step += 1;
  }, STEP_MS);
}

initSpoken();

export {};
