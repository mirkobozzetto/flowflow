// Runs the built chapter script (src/ui/hooks/chapter_scroll.js, filled the
// way build_chapter_scroll fills it) the way Dioxus evaluates it, as an async
// function body given a `dioxus` channel, against a minimal fake DOM: a
// scroller, a sticky bar and chapter headings laid out from their offsets.
import { beforeEach, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const SCRIPT = readFileSync(
  new URL("../../src/ui/hooks/chapter_scroll.js", import.meta.url),
  "utf8",
).replaceAll("__BAR__", "reading-bar");
const AsyncFunction = async function () {}.constructor as new (
  ...args: string[]
) => (...args: unknown[]) => Promise<unknown>;

const SCROLLER_TOP = 100;
const BAR_H = 50;
const OFFSETS = [0, 500, 1200];
const TITLES = ["Intro", "Budget", "Next steps"];
const CLOCKS = ["0:00", "8:20", "20:00"];

type Listener = () => void;

const text = (value: string) => ({ nodeType: 3, nodeValue: value });

function setup(idle = true) {
  const sent: unknown[] = [];
  const inbox: ((n: unknown) => void)[] = [];
  const frames: FrameRequestCallback[] = [];
  const listeners = new Set<Listener>();
  const label = {
    title: text(TITLES[0]),
    count: text("1/3"),
    clock: text("0:00"),
  };
  const scroller = {
    scrollTop: 0,
    getBoundingClientRect: () => ({ top: SCROLLER_TOP }),
    addEventListener: (_: string, f: Listener) => listeners.add(f),
    removeEventListener: (_: string, f: Listener) => listeners.delete(f),
    scrollTo(o: { top: number }) {
      this.scrollTop = o.top;
    },
    querySelectorAll: () => headings,
    querySelector: (sel: string) =>
      headings.find((h) => sel === `[data-chapter="${h.dataset.chapter}"]`) ??
      null,
  };
  // The bar is stuck at the top; the content flows right under it.
  const bar = {
    offsetHeight: BAR_H,
    dataset: { idle: String(idle) },
    getBoundingClientRect: () => ({ bottom: SCROLLER_TOP + BAR_H }),
    querySelector: (sel: string) => {
      const key = sel.match(/data-bar="(\w+)"/)?.[1] as keyof typeof label;
      return key ? { firstChild: label[key] } : null;
    },
  };
  const headings = OFFSETS.map((offset, i) => ({
    dataset: { chapter: String(i) },
    getBoundingClientRect: () => ({
      top: SCROLLER_TOP + BAR_H + offset - scroller.scrollTop,
    }),
    querySelector: (sel: string) =>
      sel.includes("title")
        ? { textContent: TITLES[i] }
        : sel.includes("clock")
          ? { textContent: CLOCKS[i] }
          : null,
  }));
  // The chapter sheet's list: 9 rows of 50 px in a 200 px window, row 6
  // current.
  const current = { offsetTop: 300, offsetHeight: 50 };
  const list = {
    scrollTop: 0,
    clientHeight: 200,
    querySelector: (sel: string) => (sel === "[data-current]" ? current : null),
  };
  const g = globalThis as Record<string, unknown>;
  g.window = g.window ?? {};
  g.document = {
    querySelector: (sel: string) => (sel === ".note-scroll" ? scroller : null),
    getElementById: (id: string) =>
      id === "reading-bar" ? bar : id === "chapter-list" ? list : null,
  };
  g.requestAnimationFrame = (f: FrameRequestCallback) => frames.push(f);
  g.cancelAnimationFrame = () => {};
  const dioxus = {
    send: (m: unknown) => sent.push(m),
    recv: () => new Promise((resolve) => inbox.push(resolve)),
  };
  let settled = false;
  const run = async () => {
    new AsyncFunction("dioxus", SCRIPT)(dioxus).finally(() => {
      settled = true;
    });
    await Promise.resolve();
  };
  const scroll = (top: number) => {
    scroller.scrollTop = top;
    listeners.forEach((f) => f());
    frames.splice(0).forEach((f) => f(0));
  };
  const jump = async (n: number | string) => {
    inbox.shift()!(n);
    await Promise.resolve();
    await Promise.resolve();
  };
  return {
    sent,
    list,
    label,
    scroller,
    listeners,
    run,
    scroll,
    jump,
    settled: () => settled,
  };
}

let env: ReturnType<typeof setup>;
beforeEach(() => {
  env = setup();
});

test("reports the first chapter on load", async () => {
  await env.run();
  expect(env.sent).toEqual([0]);
});

test("reports a chapter once its heading reaches the bar", async () => {
  await env.run();
  env.scroll(400);
  expect(env.sent).toEqual([0]);
  env.scroll(520);
  expect(env.sent).toEqual([0, 1]);
  env.scroll(600);
  expect(env.sent).toEqual([0, 1]);
  env.scroll(1300);
  expect(env.sent).toEqual([0, 1, 2]);
});

test("relabels the bar in the scroll's own frame", async () => {
  await env.run();
  env.scroll(520);
  expect(env.label.title.nodeValue).toBe("Budget");
  expect(env.label.count.nodeValue).toBe("2/3");
  expect(env.label.clock.nodeValue).toBe("8:20");
});

test("leaves the clock to the audio while a clip is loaded", async () => {
  env = setup(false);
  await env.run();
  env.scroll(520);
  expect(env.label.title.nodeValue).toBe("Budget");
  expect(env.label.clock.nodeValue).toBe("0:00");
});

test("stays running, so Rust's jumps still reach it", async () => {
  await env.run();
  await env.jump(1);
  expect(env.settled()).toBe(false);
});

test("a jump tucks the heading under the bar and reports it", async () => {
  await env.run();
  await env.jump(2);
  expect(env.scroller.scrollTop).toBe(1200 + 4);
  env.scroll(env.scroller.scrollTop);
  expect(env.sent).toEqual([0, 2]);
});

test("-1 goes back to the top of the note", async () => {
  await env.run();
  env.scroll(900);
  await env.jump(-1);
  expect(env.scroller.scrollTop).toBe(0);
});

test("a remount drops the previous scroll listener", async () => {
  await env.run();
  await env.run();
  expect(env.listeners.size).toBe(1);
});

test("an opened chapter list is centered on its current row", async () => {
  await env.run();
  await env.jump("chapter-list");
  expect(env.list.scrollTop).toBe(300 - 100 + 25);
  expect(env.scroller.scrollTop).toBe(0);
});
