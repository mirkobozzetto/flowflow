// Teleprompter for the Mac window: the said words turn black, the current
// one lights up, and the pace changes word by word like a real voice.
const WORD_MS = 150;
const LETTER_MS = 16;
const PAUSE_MS = 420;
const PAUSE_AT = /[.,:?!»”]$/;
const SPEAKING_WAVE = 2.2;
const RESTING_WAVE = "0.7";

function clockLabel(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

function initPrompter(): void {
  const column = document.querySelector<HTMLElement>("[data-spoken]");
  const view = column?.parentElement;
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (!column || !view || reduced) return;

  const words = [...column.querySelectorAll<HTMLElement>("span")];
  const wave = document.querySelector<HTMLCanvasElement>(".capsule canvas");
  const clock = document.querySelector<HTMLElement>("[data-voice-clock]");
  let started = performance.now();
  let at = 0;

  const step = (): void => {
    if (at >= words.length) {
      at = 0;
      started = performance.now();
    }
    const word = words[at];
    if (!word) return;
    words.forEach((w, i) => {
      w.classList.toggle("said", i < at);
      w.classList.toggle("now", i === at);
    });

    const line = word.parentElement as HTMLElement;
    const offset =
      line.offsetTop + line.offsetHeight / 2 - view.clientHeight / 2;
    column.style.transform = `translateY(${-Math.max(0, offset)}px)`;

    const text = word.textContent?.trim() ?? "";
    const pause = PAUSE_AT.test(text);
    if (wave) {
      wave.dataset.speed = pause
        ? RESTING_WAVE
        : String(SPEAKING_WAVE + Math.random() * 1.4);
    }
    if (clock) clock.textContent = clockLabel(performance.now() - started);

    const ms =
      (WORD_MS + text.length * LETTER_MS) * (0.7 + Math.random() * 0.6) +
      (pause ? PAUSE_MS : 0);
    at += 1;
    setTimeout(step, ms);
  };
  step();
}

initPrompter();

export {};
