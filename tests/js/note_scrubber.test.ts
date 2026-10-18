// Runs the built scrubber script (src/ui/hooks/note_scrubber.js, filled the
// way build_note_scrubber fills it) the way Dioxus evaluates it, as an async
// function body given a `dioxus` channel, against a minimal fake DOM: the
// note's scroller, the reading bar, chapter headings and the scrubber zone.
import { beforeEach, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const SCRIPT = readFileSync(
  new URL("../../src/ui/hooks/note_scrubber.js", import.meta.url),
  "utf8",
)
  .replace('"__ROOT__"', '"note-scrubber"')
  .replace('"__BAR__"', '"reading-bar"')
  .replace('"__CHAPTER__"', '"Chapitre"')
  .replace('"__START__"', '"Début de la note"')
  .replace('"__END__"', '"Fin de la note"');
const AsyncFunction = async function () {}.constructor as new (
  ...args: string[]
) => (...args: unknown[]) => Promise<unknown>;

const SCROLLER_TOP = 100;
const BAR_H = 50;
// The bar's sticky top (bar.rs), as getComputedStyle reports it.
const STICK_PX = -12;
const CLIENT_H = 600;
const ZONE_H = 400;
const KNOB_H = 40;
// Half the held knob: the knob's center stays this far inside the rail.
const KNOB_HALF = 26;
const TRAVEL = ZONE_H - 2 * KNOB_HALF;
// Where each heading sits in the note's content.
const OFFSETS = [300, 1500, 3000];
const TITLES = ["Intro", "Budget", "Next steps"];
const CLOCKS = ["0:00", "25:00", "50:00"];

type Listener = (e?: unknown) => void;

// An element that keeps its attributes and listeners.
function node(extra: Record<string, unknown> = {}) {
  const attrs = new Set<string>();
  const on = new Map<string, Listener>();
  return {
    attrs,
    on,
    style: {} as Record<string, string>,
    dataset: {} as Record<string, string>,
    textContent: "",
    setAttribute: (a: string) => attrs.add(a),
    removeAttribute: (a: string) => attrs.delete(a),
    toggleAttribute: (a: string, force: boolean) =>
      force ? attrs.add(a) : attrs.delete(a),
    addEventListener: (type: string, f: Listener) => on.set(type, f),
    removeEventListener: (type: string) => on.delete(type),
    ...extra,
  };
}

function setup(contentH = 6000) {
  const sent: unknown[] = [];
  const frames: FrameRequestCallback[] = [];
  const scroller = node({
    scrollTop: 0,
    scrollHeight: contentH,
    clientHeight: CLIENT_H,
    getBoundingClientRect: () => ({ top: SCROLLER_TOP }),
    querySelectorAll: () => headings,
  }) as ReturnType<typeof node> & { scrollTop: number };
  const headings = OFFSETS.map((offset, i) =>
    node({
      getBoundingClientRect: () => ({
        top: SCROLLER_TOP + offset - scroller.scrollTop,
      }),
      querySelector: (sel: string) =>
        sel.includes("title")
          ? { textContent: TITLES[i] }
          : sel.includes("clock")
            ? { textContent: CLOCKS[i] }
            : null,
    }),
  );
  // At the note's top the bar sits lower, under the title; it sticks once
  // the note has scrolled past it.
  const bar = {
    offsetHeight: BAR_H,
    getBoundingClientRect: () => ({
      bottom: SCROLLER_TOP + BAR_H + Math.max(0, 250 - scroller.scrollTop),
    }),
  };
  const knob = node({ offsetHeight: KNOB_H, setPointerCapture: () => {} });
  const labels = { n: node(), clock: node(), title: node() };
  const bubble = node({
    querySelector: (sel: string) =>
      labels[sel.match(/data-scrub=(\w+)/)?.[1] as keyof typeof labels] ??
      null,
  });
  const ticks = OFFSETS.map((_, i) => {
    const t = node();
    t.dataset.tick = String(i);
    return t;
  });
  const zone = node({
    clientHeight: ZONE_H,
    querySelector: (sel: string) =>
      sel === "[data-scrub=knob]"
        ? knob
        : sel === "[data-scrub=bubble]"
          ? bubble
          : null,
    querySelectorAll: () => ticks,
  });
  const g = globalThis as Record<string, unknown>;
  g.window = g.window ?? {};
  g.document = {
    querySelector: (sel: string) => (sel === ".note-scroll" ? scroller : null),
    getElementById: (id: string) =>
      id === "reading-bar" ? bar : id === "note-scrubber" ? zone : null,
  };
  g.getComputedStyle = () => ({ top: `${STICK_PX}px` });
  g.requestAnimationFrame = (f: FrameRequestCallback) => frames.push(f);
  g.cancelAnimationFrame = () => {};
  const dioxus = {
    send: (m: unknown) => sent.push(m),
    recv: () => new Promise(() => {}),
  };
  const run = async () => {
    new AsyncFunction("dioxus", SCRIPT)(dioxus);
    await Promise.resolve();
  };
  const scroll = (top: number) => {
    scroller.scrollTop = top;
    scroller.on.get("scroll")?.();
    frames.splice(0).forEach((f) => f(0));
  };
  // A finger on the knob at screen height `y`.
  const at = (y: number) => ({ clientY: y, pointerId: 1 });
  const press = (y: number) => knob.on.get("pointerdown")!(at(y));
  const drag = (y: number) => knob.on.get("pointermove")!(at(y));
  const lift = () => knob.on.get("pointerup")!();
  return {
    sent,
    scroller,
    zone,
    knob,
    labels,
    ticks,
    run,
    scroll,
    press,
    drag,
    lift,
  };
}

const MAX = 6000 - CLIENT_H;
// Where the jump to heading `i` lands, as a share of the scroll.
const mark = (i: number) => (OFFSETS[i] - BAR_H + 4) / MAX;
const FINGER = 300;

let env: ReturnType<typeof setup>;
beforeEach(() => {
  env = setup();
});

test("a short note never shows the knob", async () => {
  env = setup(1200);
  await env.run();
  env.scroll(300);
  expect(env.zone.attrs.has("data-on")).toBe(false);
});

test("a long note shows the knob at the scroll's height", async () => {
  await env.run();
  env.scroll(MAX / 2);
  expect(env.zone.attrs.has("data-on")).toBe(true);
  expect(env.knob.style.top).toBe(`${KNOB_HALF + TRAVEL / 2 - KNOB_H / 2}px`);
});

test("the rail keeps one length at the note's top and further down", async () => {
  await env.run();
  env.scroll(10);
  const atTop = env.zone.style.top;
  env.scroll(MAX / 2);
  expect(env.zone.style.top).toBe(atTop);
  expect(atTop).toBe(`${SCROLLER_TOP + STICK_PX + BAR_H + 8}px`);
});

test("the knob stays in the rail at both ends", async () => {
  await env.run();
  env.scroll(0.0001);
  expect(parseFloat(env.knob.style.top)).toBeGreaterThanOrEqual(0);
  env.scroll(MAX);
  expect(parseFloat(env.knob.style.top) + KNOB_H).toBeLessThanOrEqual(ZONE_H);
});

test("only the knob takes the finger", async () => {
  await env.run();
  expect(env.zone.on.has("pointerdown")).toBe(false);
  expect(env.knob.on.has("pointerdown")).toBe(true);
});

test("a grab taps and stops the note's own scrolling, without moving it", async () => {
  await env.run();
  env.scroll(1000);
  env.press(FINGER);
  expect(env.sent).toEqual(["grab"]);
  expect(env.scroller.attrs.has("data-scrubbing")).toBe(true);
  expect(env.scroller.scrollTop).toBe(1000);
});

test("dragging moves the note by as much as the finger moves", async () => {
  await env.run();
  env.scroll(1000);
  env.press(FINGER);
  env.drag(FINGER + 0.3 * TRAVEL);
  expect(env.scroller.scrollTop).toBeCloseTo(1000 + 0.3 * MAX);
  expect(env.labels.title.textContent).toBe("Budget");
});

test("dragged past the bottom it lands on the end", async () => {
  await env.run();
  env.scroll(1000);
  env.press(FINGER);
  env.drag(FINGER + ZONE_H);
  expect(env.scroller.scrollTop).toBe(MAX);
  expect(env.labels.n.textContent).toBe("↓");
  expect(env.labels.title.textContent).toBe("Fin de la note");
});

test("dragged past the top it lands on the start", async () => {
  await env.run();
  env.scroll(1000);
  env.press(FINGER);
  env.drag(FINGER - ZONE_H);
  expect(env.scroller.scrollTop).toBe(0);
  expect(env.labels.title.textContent).toBe("Début de la note");
});

test("close to a chapter mark it snaps onto that chapter", async () => {
  await env.run();
  env.scroll((mark(1) - 0.05) * MAX);
  env.press(FINGER);
  env.drag(FINGER + 0.04 * TRAVEL);
  expect(env.scroller.scrollTop).toBeCloseTo(OFFSETS[1] - BAR_H + 4);
  expect(env.labels.n.textContent).toBe("Chapitre 2/3");
  expect(env.labels.clock.textContent).toBe("25:00");
  expect(env.labels.title.textContent).toBe("Budget");
  expect(env.ticks[1].attrs.has("data-current")).toBe(true);
  expect(env.ticks[0].attrs.has("data-current")).toBe(false);
});

test("the rail marks each chapter where the knob would show it", async () => {
  await env.run();
  env.scroll(1000);
  env.press(FINGER);
  env.ticks.forEach((t, i) => {
    expect(parseFloat(t.style.top)).toBeCloseTo(KNOB_HALF + mark(i) * TRAVEL);
  });
});

test("letting go gives the note back its scrolling", async () => {
  await env.run();
  env.scroll(1000);
  env.press(FINGER);
  env.lift();
  expect(env.scroller.attrs.has("data-scrubbing")).toBe(false);
  expect(env.zone.attrs.has("data-drag")).toBe(false);
});

test("a remount drops the previous listeners", async () => {
  await env.run();
  const first = env.scroller.on.get("scroll");
  await env.run();
  expect(env.scroller.on.get("scroll")).not.toBe(first);
});
