// Chapter tracking for the note's reading bar (webview only).
//
// Reports which chapter heading ([data-chapter="n"]) sits under the sticky
// reading bar (#<barId>) as the note scrolls, and scrolls to a chapter when
// Rust sends its number (-1 = top of the note). A string is the id of the
// chapter list just opened, to center on its [data-current] row, and
// {audio, word} a word to bring mid-screen (a moment played from the
// summary): its paragraph carries [data-audio][data-first]. Positions
// are read with getBoundingClientRect on rAF, so headings that arrive later
// (chapters are generated after the note opens) need no re-eval.
//
// Source of truth: this .ts; `make js` -> .js, filled by build_chapter_scroll.

// A module, for the top-level await below; bun emits no export for it.
export {};

const CFG = { barId: "__BAR__" };
// A heading this close under the bar already counts as the current chapter,
// so a jump that tucks it just under the bar reports that chapter.
const LEAD_PX = 8;
const TEXT_NODE = 3;

type Reveal = { audio: string; word: number };

// The list opens on the current chapter instead of its top.
function center(listId: string): void {
  const list = document.getElementById(listId);
  const row = list?.querySelector<HTMLElement>("[data-current]");
  if (!list || !row) return;
  list.scrollTop =
    row.offsetTop - list.clientHeight / 2 + row.offsetHeight / 2;
}

function controller(): ((msg: number | Reveal) => void) | null {
  // The reading view remounts after each edit: drop the previous listener.
  const w0 = window as unknown as Record<string, (() => void) | undefined>;
  const KEY = "__chapters_" + CFG.barId;
  const prev = w0[KEY];
  if (prev) prev();

  const scroller = document.querySelector<HTMLElement>(".note-scroll");
  if (!scroller) return null;
  const sc: HTMLElement = scroller;

  function headings(): HTMLElement[] {
    return Array.from(sc.querySelectorAll<HTMLElement>("[data-chapter]"));
  }
  function bar(): HTMLElement | null {
    return document.getElementById(CFG.barId);
  }
  function current(hs: HTMLElement[]): number {
    const b = bar();
    const line =
      (b ? b.getBoundingClientRect().bottom : sc.getBoundingClientRect().top) +
      LEAD_PX;
    let cur = 0;
    for (const h of hs) {
      if (h.getBoundingClientRect().top > line) break;
      cur = Number(h.dataset.chapter);
    }
    return cur;
  }

  // Writes the bar's label in the same frame as the scroll, ahead of the
  // Rust round trip. In place: Dioxus keeps a handle on each text node and
  // patches it with the same value right after.
  function write(b: HTMLElement, key: string, text: string | null): void {
    const node = b.querySelector(`[data-bar="${key}"]`)?.firstChild;
    if (text !== null && node && node.nodeType === TEXT_NODE) {
      node.nodeValue = text;
    }
  }
  function paint(hs: HTMLElement[], n: number): void {
    const b = bar();
    const h = hs[n];
    if (!b || !h) return;
    const read = (key: string) =>
      h.querySelector(`[data-heading="${key}"]`)?.textContent ?? null;
    write(b, "title", read("title"));
    write(b, "count", `${n + 1}/${hs.length}`);
    // While a clip is loaded, the clock follows the audio instead.
    if (b.dataset.idle === "true") write(b, "clock", read("clock"));
  }

  let last = -1;
  let raf = 0;
  function report(): void {
    raf = 0;
    const hs = headings();
    const c = current(hs);
    if (c !== last) {
      last = c;
      paint(hs, c);
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

  // The clip's paragraphs come in word order: the last one starting at or
  // before the word holds it, as its span child there.
  function reveal({ audio, word }: Reveal): void {
    let target: Element | null = null;
    const ps = sc.querySelectorAll<HTMLElement>(`[data-audio="${audio}"]`);
    for (const p of Array.from(ps)) {
      const first = Number(p.dataset.first);
      if (first > word) break;
      target = p.querySelector("p")?.children[word - first] ?? p;
    }
    if (!target) return;
    const y =
      target.getBoundingClientRect().top - sc.getBoundingClientRect().top;
    sc.scrollTo({
      top: sc.scrollTop + y - sc.clientHeight / 2,
      behavior: "smooth",
    });
  }

  return (msg: number | Reveal) => {
    if (typeof msg === "object") {
      reveal(msg);
      return;
    }
    const n = msg;
    if (n < 0) {
      sc.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    const h = sc.querySelector<HTMLElement>(`[data-chapter="${n}"]`);
    if (!h) return;
    const b = bar();
    const barH = b ? b.offsetHeight : 0;
    const y = h.getBoundingClientRect().top - sc.getBoundingClientRect().top;
    sc.scrollTo({
      top: sc.scrollTop + y - barH + LEAD_PX / 2,
      behavior: "smooth",
    });
  };
}

const handle = controller();
// Awaiting keeps this eval alive: Dioxus closes its channel as soon as the
// script returns, and Rust's jumps would then go nowhere.
while (handle) {
  const msg = await dioxus.recv();
  if (typeof msg === "string") center(msg);
  else if (msg && typeof msg === "object") handle(msg as Reveal);
  else handle(Number(msg));
}
