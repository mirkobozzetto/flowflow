// src/ui/hooks/chapter_scroll.ts
var CFG = { barId: "__BAR__" };
var LEAD_PX = 8;
var TEXT_NODE = 3;
function center(listId) {
  const list = document.getElementById(listId);
  const row = list?.querySelector("[data-current]");
  if (!list || !row)
    return;
  list.scrollTop = row.offsetTop - list.clientHeight / 2 + row.offsetHeight / 2;
}
function controller() {
  const w0 = window;
  const KEY = "__chapters_" + CFG.barId;
  const prev = w0[KEY];
  if (prev)
    prev();
  const scroller = document.querySelector(".note-scroll");
  if (!scroller)
    return null;
  const sc = scroller;
  function headings() {
    return Array.from(sc.querySelectorAll("[data-chapter]"));
  }
  function bar() {
    return document.getElementById(CFG.barId);
  }
  function current(hs) {
    const b = bar();
    const line = (b ? b.getBoundingClientRect().bottom : sc.getBoundingClientRect().top) + LEAD_PX;
    let cur = 0;
    for (const h of hs) {
      if (h.getBoundingClientRect().top > line)
        break;
      cur = Number(h.dataset.chapter);
    }
    return cur;
  }
  function write(b, key, text) {
    const node = b.querySelector(`[data-bar="${key}"]`)?.firstChild;
    if (text !== null && node && node.nodeType === TEXT_NODE) {
      node.nodeValue = text;
    }
  }
  function paint(hs, n) {
    const b = bar();
    const h = hs[n];
    if (!b || !h)
      return;
    const read = (key) => h.querySelector(`[data-heading="${key}"]`)?.textContent ?? null;
    write(b, "title", read("title"));
    write(b, "count", `${n + 1}/${hs.length}`);
    if (b.dataset.idle === "true")
      write(b, "clock", read("clock"));
  }
  let last = -1;
  let raf = 0;
  function report() {
    raf = 0;
    const hs = headings();
    const c = current(hs);
    if (c !== last) {
      last = c;
      paint(hs, c);
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
  return (n) => {
    if (n < 0) {
      sc.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    const h = sc.querySelector(`[data-chapter="${n}"]`);
    if (!h)
      return;
    const b = bar();
    const barH = b ? b.offsetHeight : 0;
    const y = h.getBoundingClientRect().top - sc.getBoundingClientRect().top;
    sc.scrollTo({
      top: sc.scrollTop + y - barH + LEAD_PX / 2,
      behavior: "smooth"
    });
  };
}
var jump = controller();
while (jump) {
  const msg = await dioxus.recv();
  if (typeof msg === "string")
    center(msg);
  else
    jump(Number(msg));
}
