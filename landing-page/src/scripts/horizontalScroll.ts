const MOBILE_QUERY = "(max-width: 768px)";
const REDUCED_QUERY = "(prefers-reduced-motion: reduce)";

function initHorizontalScroll(): void {
  const section = document.getElementById("hscroll");
  const track = document.getElementById("htrack");

  if (!section || !track) return;

  const reduced = matchMedia(REDUCED_QUERY).matches;
  const mobile = matchMedia(MOBILE_QUERY).matches;

  if (reduced || mobile) {
    section.dataset.fallback = "true";
    track.style.transform = "";
    return;
  }

  section.dataset.fallback = "false";

  const stage = section;
  const rail = track;
  let ticking = false;

  const update = (): void => {
    const rect = stage.getBoundingClientRect();
    const total = stage.offsetHeight - innerHeight;
    if (total <= 0) return;

    const progress = Math.min(1, Math.max(0, -rect.top / total));
    const distance = rail.scrollWidth - innerWidth;
    rail.style.transform = `translateX(${-progress * distance}px)`;
    ticking = false;
  };

  const onScroll = (): void => {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(update);
  };

  addEventListener("scroll", onScroll, { passive: true });
  addEventListener("resize", onScroll, { passive: true });
  update();
}

if (document.readyState !== "loading") {
  initHorizontalScroll();
} else {
  addEventListener("DOMContentLoaded", initHorizontalScroll);
}

export {};
