// A clip starts once a third of it is on screen, from its first frame.
const SEEN_RATIO = 0.3;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

function initLazyVideos(): void {
  const videos =
    document.querySelectorAll<HTMLVideoElement>("video[data-lazy]");
  if (reduced || !videos.length) return;

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const video = entry.target as HTMLVideoElement;
        if (entry.intersectionRatio < SEEN_RATIO) {
          video.pause();
          continue;
        }
        if (!video.getAttribute("src") && video.dataset.src) {
          video.src = video.dataset.src;
        }
        video.play().catch(() => undefined);
      }
    },
    { threshold: SEEN_RATIO },
  );

  for (const video of videos) {
    observer.observe(video);
  }
}

if (document.readyState === "loading") {
  addEventListener("DOMContentLoaded", initLazyVideos);
} else {
  initLazyVideos();
}

export {};
