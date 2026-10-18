// src/ui/hooks/note_scrubber.ts
var CFG = {
  rootId: "__ROOT__",
  barId: "__BAR__",
  chapter: "__CHAPTER__",
  start: "__START__",
  end: "__END__"
};
var LONG_SCREENS = 3;
var HIDE_MS = 1800;
var SNAP = 0.018;
var LEAD_PX = 4;
var GAP_PX = 8;
var BUBBLE_EDGE_PX = 28;
function start() {
  const w0 = window;
  const KEY = "__scrubber_" + CFG.rootId;
  const prev = w0[KEY];
  if (prev)
    prev();
  const found = document.querySelector(".note-scroll");
  const root = document.getElementById(CFG.rootId);
  if (!found || !root)
    return false;
  const sc = found;
  const zone = root;
  const knob = zone.querySelector("[data-scrub=knob]");
  const bubble = zone.querySelector("[data-scrub=bubble]");
  if (!knob || !bubble)
    return false;
  const kn = knob;
  const bu = bubble;
  const max = () => Math.max(1, sc.scrollHeight - sc.clientHeight);
  const long = () => sc.scrollHeight >= sc.clientHeight * LONG_SCREENS;
  const bar = () => document.getElementById(CFG.barId);
  function headings() {
    return Array.from(sc.querySelectorAll("[data-chapter]"));
  }
  function marks() {
    const top = sc.getBoundingClientRect().top;
    const barH = bar()?.offsetHeight ?? 0;
    return headings().map((h) => {
      const y = h.getBoundingClientRect().top - top + sc.scrollTop;
      return Math.min(1, Math.max(0, (y - barH + LEAD_PX) / max()));
    });
  }
  function anchor() {
    const b = bar();
    const top = b ? b.getBoundingClientRect().bottom : sc.getBoundingClientRect().top;
    zone.style.top = `${top + GAP_PX}px`;
  }
  function place(r) {
    const h = zone.clientHeight;
    kn.style.top = `${r * h - kn.offsetHeight / 2}px`;
    const y = Math.min(h - BUBBLE_EDGE_PX, Math.max(BUBBLE_EDGE_PX, r * h));
    bu.style.top = `${y}px`;
  }
  function text(key, value) {
    const node = bu.querySelector(`[data-scrub=${key}]`);
    if (node)
      node.textContent = value;
  }
  function label(r, at2) {
    const hs = headings();
    let i = -1;
    at2.forEach((m, k) => {
      if (m <= r + 0.000001)
        i = k;
    });
    zone.querySelectorAll("[data-tick]").forEach((t) => {
      t.toggleAttribute("data-current", Number(t.dataset.tick) === i);
    });
    bu.removeAttribute("data-empty");
    if (r <= 0 || r >= 1) {
      text("n", r <= 0 ? "↑" : "↓");
      text("clock", "");
      text("title", r <= 0 ? CFG.start : CFG.end);
    } else if (i >= 0) {
      const read = (key) => hs[i].querySelector(`[data-heading="${key}"]`)?.textContent ?? "";
      text("n", `${CFG.chapter} ${i + 1}/${hs.length}`);
      text("clock", read("clock"));
      text("title", read("title"));
    } else {
      bu.setAttribute("data-empty", "");
    }
  }
  let held = false;
  let timer;
  let raf = 0;
  let at = [];
  function show() {
    zone.setAttribute("data-on", "");
    clearTimeout(timer);
    if (!held)
      timer = setTimeout(() => zone.removeAttribute("data-on"), HIDE_MS);
  }
  function follow() {
    raf = 0;
    if (!held) {
      anchor();
      place(sc.scrollTop / max());
    }
    show();
  }
  function onScroll() {
    if (!long())
      return;
    sc.setAttribute("data-scrubber", "");
    if (!raf)
      raf = requestAnimationFrame(follow);
  }
  function to(e) {
    const box = zone.getBoundingClientRect();
    let r = Math.min(1, Math.max(0, (e.clientY - box.top) / box.height));
    if (r < SNAP)
      r = 0;
    else if (r > 1 - SNAP)
      r = 1;
    else
      for (const m of at)
        if (Math.abs(m - r) < SNAP)
          r = m;
    sc.scrollTop = r * max();
    place(r);
    label(r, at);
  }
  function grab(e) {
    held = true;
    at = marks();
    const h = zone.clientHeight;
    zone.querySelectorAll("[data-tick]").forEach((t) => {
      const m = at[Number(t.dataset.tick)];
      if (m !== undefined)
        t.style.top = `${m * h}px`;
    });
    zone.setAttribute("data-drag", "");
    sc.setAttribute("data-scrubbing", "");
    zone.setPointerCapture(e.pointerId);
    show();
    dioxus.send("grab");
    to(e);
  }
  function move(e) {
    if (held)
      to(e);
  }
  function release() {
    if (!held)
      return;
    held = false;
    zone.removeAttribute("data-drag");
    sc.removeAttribute("data-scrubbing");
    show();
  }
  sc.addEventListener("scroll", onScroll, { passive: true });
  zone.addEventListener("pointerdown", grab);
  zone.addEventListener("pointermove", move);
  zone.addEventListener("pointerup", release);
  zone.addEventListener("pointercancel", release);
  w0[KEY] = () => {
    sc.removeEventListener("scroll", onScroll);
    zone.removeEventListener("pointerdown", grab);
    zone.removeEventListener("pointermove", move);
    zone.removeEventListener("pointerup", release);
    zone.removeEventListener("pointercancel", release);
    clearTimeout(timer);
    if (raf)
      cancelAnimationFrame(raf);
  };
  return true;
}
if (start()) {
  for (;; )
    await dioxus.recv();
}
