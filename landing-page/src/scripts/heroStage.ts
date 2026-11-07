const PROMO = "promo";
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

function initStage(root: HTMLElement): void {
  const video = root.querySelector<HTMLVideoElement>("[data-stage-video]");
  const sound = root.querySelector<HTMLButtonElement>("[data-sound]");
  const play = root.querySelector<HTMLButtonElement>("[data-play]");
  const choices = root.querySelectorAll<HTMLButtonElement>("[data-clip]");
  const captions = root.querySelectorAll<HTMLElement>("[data-caption]");
  const device = root.querySelector<HTMLElement>("[data-device]");
  const lang = root.dataset.lang;
  if (!video || !lang) return;

  const start = (): void => {
    video
      .play()
      .then(() => {
        if (play) play.hidden = true;
      })
      .catch(() => {
        if (play) play.hidden = false;
      });
  };

  const setSound = (on: boolean): void => {
    video.muted = !on;
    if (!sound) return;
    sound.setAttribute("aria-pressed", String(on));
    sound.setAttribute(
      "aria-label",
      (on ? sound.dataset.off : sound.dataset.on) ?? "",
    );
  };

  const show = (clip: string): void => {
    const base = `/videos/${lang}/${clip}`;
    setSound(false);
    video.poster = `${base}.webp`;
    video.src = `${base}.mp4`;
    if (sound) sound.hidden = clip !== PROMO;
    if (device) device.dataset.format = clip === PROMO ? "story" : "screen";
    for (const choice of choices) {
      choice.setAttribute("aria-pressed", String(choice.dataset.clip === clip));
    }
    for (const caption of captions) {
      caption.hidden = caption.dataset.caption !== clip;
    }
    start();
  };

  for (const choice of choices) {
    choice.addEventListener("click", () => {
      if (choice.dataset.clip) show(choice.dataset.clip);
    });
  }

  sound?.addEventListener("click", () => {
    const on = video.muted;
    if (on) video.currentTime = 0;
    setSound(on);
    start();
  });

  play?.addEventListener("click", start);

  if (reduced) {
    video.removeAttribute("autoplay");
    video.pause();
    if (play) play.hidden = false;
    return;
  }
  start();
}

function init(): void {
  for (const root of document.querySelectorAll<HTMLElement>("[data-stage]")) {
    initStage(root);
  }
}

if (document.readyState === "loading") {
  addEventListener("DOMContentLoaded", init);
} else {
  init();
}

export {};
