// Teleprompter for the Mac window: the said words turn black, the current
// one lights up, and the pace changes word by word like a real voice.
const WORD_MS = 150;
const LETTER_MS = 16;
const PAUSE_MS = 420;
const PAUSE_AT = /[.,:?!»”]$/;
const SILENT = "0.06";
const SYLLABLE_DIP = 0.55;
const SEEN_RATIO = 0.3;

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
  let started = 0;
  let at = 0;
  let timer = 0;

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

    // The line said before stays whole at the top: no half-cut text.
    const line = word.parentElement as HTMLElement;
    const before = line.previousElementSibling as HTMLElement | null;
    column.style.transform = `translateY(${-(before ?? line).offsetTop}px)`;

    const text = word.textContent?.trim() ?? "";
    const pause = PAUSE_AT.test(text);
    const spoken =
      (WORD_MS + text.length * LETTER_MS) * (0.7 + Math.random() * 0.6);
    const ms = spoken + (pause ? PAUSE_MS : 0);
    if (wave) {
      const peak = 0.65 + Math.random() * 0.35;
      wave.dataset.level = String(peak);
      setTimeout(() => {
        wave.dataset.level = String(peak * SYLLABLE_DIP);
      }, spoken * 0.45);
      if (pause) {
        setTimeout(() => {
          wave.dataset.level = SILENT;
        }, spoken);
      }
    }
    if (clock) clock.textContent = clockLabel(performance.now() - started);
    at += 1;
    timer = window.setTimeout(step, ms);
  };

  // The dictation starts on its first word when the window comes into view.
  let running = false;
  new IntersectionObserver(
    ([entry]) => {
      const seen = (entry?.intersectionRatio ?? 0) >= SEEN_RATIO;
      if (seen === running) return;
      running = seen;
      if (!seen) {
        clearTimeout(timer);
        if (wave) wave.dataset.level = SILENT;
        return;
      }
      if (at === 0) started = performance.now();
      step();
    },
    { threshold: SEEN_RATIO },
  ).observe(view);
}

initPrompter();

export {};
