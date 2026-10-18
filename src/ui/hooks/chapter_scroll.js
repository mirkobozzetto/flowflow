// src/ui/hooks/chapter_scroll.ts
(function() {
  const CFG = { barId: "__BAR__" };
  const LEAD_PX = 8;
  const w0 = window;
  const KEY = "__chapters_" + CFG.barId;
  const prev = w0[KEY];
  if (prev)
    prev();
  const scroller = document.querySelector(".note-scroll");
  if (!scroller)
    return;
  const sc = scroller;
  function headings() {
    return Array.from(sc.querySelectorAll("[data-chapter]"));
  }
  function barBottom() {
    const bar = document.getElementById(CFG.barId);
    return bar ? bar.getBoundingClientRect().bottom : sc.getBoundingClientRect().top;
  }
  function current() {
    const line = barBottom() + LEAD_PX;
    let cur = 0;
    for (const h of headings()) {
      if (h.getBoundingClientRect().top > line)
        break;
      cur = Number(h.dataset.chapter);
    }
    return cur;
  }
  let last = -1;
  let raf = 0;
  function report() {
    raf = 0;
    const c = current();
    if (c !== last) {
      last = c;
      dioxus.send(c);
    }
  }
  function onScroll() {
    if (!raf)
      raf = requestAnimationFrame(report);
  }
  sc.addEventListener("scroll", onScroll, { passive: true });
  w0[KEY] = () => {
    sc.removeEventListener("scroll", onScroll);
    if (raf)
      cancelAnimationFrame(raf);
  };
  report();
  function jump(n) {
    if (n < 0) {
      sc.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    const h = sc.querySelector('[data-chapter="' + n + '"]');
    if (!h)
      return;
    const bar = document.getElementById(CFG.barId);
    const barH = bar ? bar.offsetHeight : 0;
    const y = h.getBoundingClientRect().top - sc.getBoundingClientRect().top;
    sc.scrollTo({ top: sc.scrollTop + y - barH + LEAD_PX / 2, behavior: "smooth" });
  }
  (async () => {
    for (;; )
      jump(Number(await dioxus.recv()));
  })();
})();
