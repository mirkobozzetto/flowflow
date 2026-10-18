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
var KNOB_HALF_PX = 26;
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
  const travel = () => Math.max(1, zone.clientHeight - 2 * KNOB_HALF_PX);
  const center = (r) => KNOB_HALF_PX + r * travel();
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
    const stuck = b ? parseFloat(getComputedStyle(b).top) || 0 : 0;
    const top = sc.getBoundingClientRect().top + stuck + (b?.offsetHeight ?? 0);
    zone.style.top = `${top + GAP_PX}px`;
  }
  function place(r) {
    const y = center(r);
    kn.style.top = `${y - kn.offsetHeight / 2}px`;
    const h = zone.clientHeight;
    bu.style.top = `${Math.min(h - BUBBLE_EDGE_PX, Math.max(BUBBLE_EDGE_PX, y))}px`;
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
  let y0 = 0;
  let r0 = 0;
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
  function grab(e) {
    held = true;
    at = marks();
    y0 = e.clientY;
    r0 = Math.min(1, Math.max(0, sc.scrollTop / max()));
    zone.querySelectorAll("[data-tick]").forEach((t) => {
      const m = at[Number(t.dataset.tick)];
      if (m !== undefined)
        t.style.top = `${center(m)}px`;
    });
    zone.setAttribute("data-drag", "");
    sc.setAttribute("data-scrubbing", "");
    kn.setPointerCapture(e.pointerId);
    show();
    dioxus.send("grab");
    place(r0);
    label(r0, at);
  }
  function move(e) {
    if (!held)
      return;
    let r = Math.min(1, Math.max(0, r0 + (e.clientY - y0) / travel()));
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
  function release() {
    if (!held)
      return;
    held = false;
    zone.removeAttribute("data-drag");
    sc.removeAttribute("data-scrubbing");
    show();
  }
  sc.addEventListener("scroll", onScroll, { passive: true });
  kn.addEventListener("pointerdown", grab);
  kn.addEventListener("pointermove", move);
  kn.addEventListener("pointerup", release);
  kn.addEventListener("pointercancel", release);
  w0[KEY] = () => {
    sc.removeEventListener("scroll", onScroll);
    kn.removeEventListener("pointerdown", grab);
    kn.removeEventListener("pointermove", move);
    kn.removeEventListener("pointerup", release);
    kn.removeEventListener("pointercancel", release);
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
