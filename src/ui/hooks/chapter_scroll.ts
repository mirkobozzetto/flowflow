// Chapter tracking for the note's reading bar (webview only).
//
// Reports which chapter heading ([data-chapter="n"]) sits under the sticky
// reading bar (#<barId>) as the note scrolls, and scrolls to a chapter when
// Rust sends its number (-1 = top of the note). Positions are read with
// getBoundingClientRect on rAF, so headings that arrive later (chapters are
// generated after the note opens) are picked up without a re-eval.
//
// Source of truth: this .ts; `make js` -> .js, filled by build_chapter_scroll.

(function () {
  const CFG = { barId: "__BAR__" };
  // A heading this close under the bar already counts as the current chapter,
  // so a jump that tucks it just under the bar reports that chapter.
  const LEAD_PX = 8;

  // The reading view remounts after each edit: drop the previous listener.
  const w0 = window as unknown as Record<string, (() => void) | undefined>;
  const KEY = "__chapters_" + CFG.barId;
  const prev = w0[KEY];
  if (prev) prev();

  const scroller = document.querySelector<HTMLElement>(".note-scroll");
  if (!scroller) return;
  const sc: HTMLElement = scroller;

  function headings(): HTMLElement[] {
    return Array.from(sc.querySelectorAll<HTMLElement>("[data-chapter]"));
  }
  function barBottom(): number {
    const bar = document.getElementById(CFG.barId);
    return bar
      ? bar.getBoundingClientRect().bottom
      : sc.getBoundingClientRect().top;
  }
  function current(): number {
    const line = barBottom() + LEAD_PX;
    let cur = 0;
    for (const h of headings()) {
      if (h.getBoundingClientRect().top > line) break;
      cur = Number(h.dataset.chapter);
    }
    return cur;
  }

  let last = -1;
  let raf = 0;
  function report(): void {
    raf = 0;
    const c = current();
    if (c !== last) {
      last = c;
      dioxus.send(c);
    }
  }
  function onScroll(): void {
    if (!raf) raf = requestAnimationFrame(report);
  }
  sc.addEventListener("scroll", onScroll, { passive: true });
  w0[KEY] = () => {
    sc.removeEventListener("scroll", onScroll);
    if (raf) cancelAnimationFrame(raf);
  };
  report();

  function jump(n: number): void {
    if (n < 0) {
      sc.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    const h = sc.querySelector<HTMLElement>('[data-chapter="' + n + '"]');
    if (!h) return;
    const bar = document.getElementById(CFG.barId);
    const barH = bar ? bar.offsetHeight : 0;
    const y = h.getBoundingClientRect().top - sc.getBoundingClientRect().top;
    sc.scrollTo({ top: sc.scrollTop + y - barH + LEAD_PX / 2, behavior: "smooth" });
  }

  (async () => {
    for (;;) jump(Number(await dioxus.recv()));
  })();
})();
