function initProgressBar(): void {
  const bar = document.getElementById("progress");
  if (!bar) return;

  let ticking = false;

  const update = (): void => {
    const max = document.documentElement.scrollHeight - innerHeight;
    const ratio = max > 0 ? scrollY / max : 0;
    bar.style.width = `${ratio * 100}%`;
    ticking = false;
  };

  function onScroll(): void {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(update);
  }

  addEventListener("scroll", onScroll, { passive: true });
  update();
}

function initReveal(): void {
  const targets = document.querySelectorAll<HTMLElement>(".reveal:not(.in)");
  if (!targets.length) return;

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced) {
    for (const el of targets) {
      el.classList.add("in");
    }
    return;
  }

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.classList.add("in");
          observer.unobserve(entry.target);
        }
      }
    },
    { threshold: 0.16 },
  );

  for (const el of targets) {
    observer.observe(el);
  }
}

function init(): void {
  initProgressBar();
  initReveal();
}

if (document.readyState === "loading") {
  addEventListener("DOMContentLoaded", init);
} else {
  init();
}

export { initProgressBar, initReveal };
