// The note's scrubber (webview only).
//
// While a long note scrolls, a glass knob shows on the right edge
// (#<rootId>) and follows the scroll. Held, it moves the note to the same
// height: the track marks each chapter heading ([data-chapter]) where a
// chapter jump would land, the knob snaps onto those marks and onto both
// ends, the paragraphs dim, and the bubble names the landing. Rust only
// hears "grab", for the haptic tap; chapter_scroll ticks each chapter crossed.
//
// Source of truth: this .ts; `make js` -> .js, filled by build_note_scrubber.

// A module, for the top-level await below; bun emits no export for it.
export {};

const CFG = {
  rootId: "__ROOT__",
  barId: "__BAR__",
  chapter: "__CHAPTER__",
  start: "__START__",
  end: "__END__",
};
// A note shorter than this many screens scrolls fine by hand.
const LONG_SCREENS = 3;
const HIDE_MS = 1800;
// Within this share of the track, the knob lands on a chapter or an end.
const SNAP = 0.018;
// chapter_scroll.ts tucks a heading this far under the bar on a jump.
const LEAD_PX = 4;
const GAP_PX = 8;
const BUBBLE_EDGE_PX = 28;

function start(): boolean {
  // The reading view remounts after each edit: drop the previous listeners.
  const w0 = window as unknown as Record<string, (() => void) | undefined>;
  const KEY = "__scrubber_" + CFG.rootId;
  const prev = w0[KEY];
  if (prev) prev();

  const found = document.querySelector<HTMLElement>(".note-scroll");
  const root = document.getElementById(CFG.rootId);
  if (!found || !root) return false;
  const sc: HTMLElement = found;
  const zone: HTMLElement = root;
  const knob = zone.querySelector<HTMLElement>("[data-scrub=knob]");
  const bubble = zone.querySelector<HTMLElement>("[data-scrub=bubble]");
  if (!knob || !bubble) return false;
  const kn: HTMLElement = knob;
  const bu: HTMLElement = bubble;

  const max = () => Math.max(1, sc.scrollHeight - sc.clientHeight);
  const long = () => sc.scrollHeight >= sc.clientHeight * LONG_SCREENS;
  const bar = () => document.getElementById(CFG.barId);
  function headings(): HTMLElement[] {
    return Array.from(sc.querySelectorAll<HTMLElement>("[data-chapter]"));
  }
  // Where each chapter jump lands, as a share of the whole scroll.
  function marks(): number[] {
    const top = sc.getBoundingClientRect().top;
    const barH = bar()?.offsetHeight ?? 0;
    return headings().map((h) => {
      const y = h.getBoundingClientRect().top - top + sc.scrollTop;
      return Math.min(1, Math.max(0, (y - barH + LEAD_PX) / max()));
    });
  }

  // The zone runs from under the reading bar down to the composer (CSS).
  function anchor(): void {
    const b = bar();
    const top = b ? b.getBoundingClientRect().bottom : sc.getBoundingClientRect().top;
    zone.style.top = `${top + GAP_PX}px`;
  }
  function place(r: number): void {
    const h = zone.clientHeight;
    kn.style.top = `${r * h - kn.offsetHeight / 2}px`;
    const y = Math.min(h - BUBBLE_EDGE_PX, Math.max(BUBBLE_EDGE_PX, r * h));
    bu.style.top = `${y}px`;
  }
  function text(key: string, value: string): void {
    const node = bu.querySelector(`[data-scrub=${key}]`);
    if (node) node.textContent = value;
  }
  function label(r: number, at: number[]): void {
    const hs = headings();
    let i = -1;
    at.forEach((m, k) => {
      if (m <= r + 1e-6) i = k;
    });
    zone.querySelectorAll<HTMLElement>("[data-tick]").forEach((t) => {
      t.toggleAttribute("data-current", Number(t.dataset.tick) === i);
    });
    bu.removeAttribute("data-empty");
    if (r <= 0 || r >= 1) {
      text("n", r <= 0 ? "↑" : "↓");
      text("clock", "");
      text("title", r <= 0 ? CFG.start : CFG.end);
    } else if (i >= 0) {
      const read = (key: string) =>
        hs[i].querySelector(`[data-heading="${key}"]`)?.textContent ?? "";
      text("n", `${CFG.chapter} ${i + 1}/${hs.length}`);
      text("clock", read("clock"));
      text("title", read("title"));
    } else {
      bu.setAttribute("data-empty", "");
    }
  }

  let held = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let raf = 0;
  let at: number[] = [];
  function show(): void {
    zone.setAttribute("data-on", "");
    clearTimeout(timer);
    if (!held) timer = setTimeout(() => zone.removeAttribute("data-on"), HIDE_MS);
  }
  function follow(): void {
    raf = 0;
    if (!held) {
      anchor();
      place(sc.scrollTop / max());
    }
    show();
  }
  function onScroll(): void {
    if (!long()) return;
    sc.setAttribute("data-scrubber", "");
    if (!raf) raf = requestAnimationFrame(follow);
  }

  function to(e: PointerEvent): void {
    const box = zone.getBoundingClientRect();
    let r = Math.min(1, Math.max(0, (e.clientY - box.top) / box.height));
    if (r < SNAP) r = 0;
    else if (r > 1 - SNAP) r = 1;
    else for (const m of at) if (Math.abs(m - r) < SNAP) r = m;
    sc.scrollTop = r * max();
    place(r);
    label(r, at);
  }
  function grab(e: PointerEvent): void {
    held = true;
    at = marks();
    const h = zone.clientHeight;
    zone.querySelectorAll<HTMLElement>("[data-tick]").forEach((t) => {
      const m = at[Number(t.dataset.tick)];
      if (m !== undefined) t.style.top = `${m * h}px`;
    });
    zone.setAttribute("data-drag", "");
    sc.setAttribute("data-scrubbing", "");
    zone.setPointerCapture(e.pointerId);
    show();
    dioxus.send("grab");
    to(e);
  }
  function move(e: PointerEvent): void {
    if (held) to(e);
  }
  function release(): void {
    if (!held) return;
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
    if (raf) cancelAnimationFrame(raf);
  };
  return true;
}

// Awaiting keeps this eval alive: Dioxus closes its channel as soon as the
// script returns, and "grab" would then go nowhere.
if (start()) {
  for (;;) await dioxus.recv();
}
