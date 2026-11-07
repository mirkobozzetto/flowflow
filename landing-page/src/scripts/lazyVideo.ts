const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

function initLazyVideos(): void {
  const videos =
    document.querySelectorAll<HTMLVideoElement>("video[data-lazy]");
  if (reduced || !videos.length) return;

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const video = entry.target as HTMLVideoElement;
        if (!entry.isIntersecting) {
          video.pause();
          continue;
        }
        if (!video.getAttribute("src") && video.dataset.src) {
          video.src = video.dataset.src;
        }
        video.play().catch(() => undefined);
      }
    },
    { rootMargin: "160px" },
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
