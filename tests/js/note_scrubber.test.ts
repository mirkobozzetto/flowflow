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
const CLIENT_H = 600;
const ZONE_TOP = 158;
const ZONE_H = 400;
const KNOB_H = 40;
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
  const bar = {
    offsetHeight: BAR_H,
    getBoundingClientRect: () => ({ bottom: SCROLLER_TOP + BAR_H }),
  };
  const knob = node({ offsetHeight: KNOB_H });
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
    getBoundingClientRect: () => ({ top: ZONE_TOP, height: ZONE_H }),
    setPointerCapture: () => {},
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
  // A finger at `r` of the zone's height.
  const at = (r: number) => ({ clientY: ZONE_TOP + r * ZONE_H, pointerId: 1 });
  const press = (r: number) => zone.on.get("pointerdown")!(at(r));
  const drag = (r: number) => zone.on.get("pointermove")!(at(r));
  const lift = () => zone.on.get("pointerup")!();
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
  expect(env.knob.style.top).toBe(`${ZONE_H / 2 - KNOB_H / 2}px`);
  expect(env.zone.style.top).toBe(`${SCROLLER_TOP + BAR_H + 8}px`);
});

test("a grab taps, dims the note, and near the bottom lands on the end", async () => {
  await env.run();
  env.scroll(1000);
  env.press(0.99);
  expect(env.sent).toEqual(["grab"]);
  expect(env.scroller.attrs.has("data-scrubbing")).toBe(true);
  expect(env.scroller.scrollTop).toBe(MAX);
  expect(env.labels.n.textContent).toBe("↓");
  expect(env.labels.title.textContent).toBe("Fin de la note");
});

test("near the top it lands on the start of the note", async () => {
  await env.run();
  env.scroll(1000);
  env.press(0.01);
  expect(env.scroller.scrollTop).toBe(0);
  expect(env.labels.title.textContent).toBe("Début de la note");
});

test("close to a chapter mark it snaps onto that chapter", async () => {
  await env.run();
  env.scroll(1000);
  env.press(mark(1) + 0.01);
  expect(env.scroller.scrollTop).toBeCloseTo(OFFSETS[1] - BAR_H + 4);
  expect(env.labels.n.textContent).toBe("Chapitre 2/3");
  expect(env.labels.clock.textContent).toBe("25:00");
  expect(env.labels.title.textContent).toBe("Budget");
  expect(env.ticks[1].attrs.has("data-current")).toBe(true);
  expect(env.ticks[0].attrs.has("data-current")).toBe(false);
});

test("the track marks each chapter where its jump lands", async () => {
  await env.run();
  env.scroll(1000);
  env.press(0.5);
  env.ticks.forEach((t, i) => {
    expect(parseFloat(t.style.top)).toBeCloseTo(mark(i) * ZONE_H);
  });
});

test("dragging follows the finger between marks", async () => {
  await env.run();
  env.scroll(1000);
  env.press(0.5);
  env.drag(0.8);
  expect(env.scroller.scrollTop).toBeCloseTo(0.8 * MAX);
  expect(env.labels.title.textContent).toBe("Next steps");
});

test("letting go undims the note", async () => {
  await env.run();
  env.scroll(1000);
  env.press(0.5);
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
