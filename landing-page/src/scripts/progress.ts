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

function init(): void {
  initProgressBar();
}

if (document.readyState === "loading") {
  addEventListener("DOMContentLoaded", init);
} else {
  init();
}

export { initProgressBar };
