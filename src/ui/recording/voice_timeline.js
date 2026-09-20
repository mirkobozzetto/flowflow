// Voice capsule timeline. Hand-written JS, NOT built from a .ts: Dioxus runs
// this text as the body of one async function, so `await dioxus.recv()` sits
// at top level, exactly like packages/document/docs/eval.md. The channel
// stays open as long as this body has not finished, i.e. for the whole take.
//
// Rust pushes one [rms, peak] slice every __TICK__ ms and `null` at the end.
// A rAF loop slides the strip in from the right between two slices (sub-pixel,
// 60 fps) and lets the newest bar breathe (instant attack, ~120 ms release);
// older bars are frozen at their true value. Bars are styled by tailwind.css
// (.voice-bars i).

const TICK = __TICK__;
const PITCH = 4; // 2 px bar + 2 px gap
const HALF = 13; // max half-height in px (track is 28 px tall)
const RELEASE = 120;

// The eval runs from the Rust effect before Dioxus has flushed the capsule
// into the DOM: poll a few frames for the node instead of giving up.
let host = null;
for (let i = 0; i < 30 && !host; i++) {
  host = document.querySelector(".voice-timeline");
  if (!host) await new Promise((r) => requestAnimationFrame(r));
}
const strip = host && host.querySelector(".voice-bars");
if (!host || !strip) {
  dioxus.send("timeline:missing");
  return;
}
if (window.__ffVoiceStop) window.__ffVoiceStop();

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const cap = Math.ceil(Math.max(host.clientWidth, 200) / PITCH) + 4;
const bars = Array.from(strip.children);
let live = null;
let target = 0;
let shown = 0;
let stamp = performance.now();
let raf = 0;
const px = (level) => Math.max(2, level * HALF * 2).toFixed(1) + "px";

const push = (rms, peak) => {
  const level = Math.min(1, Math.max(rms, peak * 0.8));
  if (live) live.style.height = px(target);
  const bar = document.createElement("i");
  bar.style.height = px(level);
  strip.appendChild(bar);
  bars.push(bar);
  while (bars.length > cap) bars.shift().remove();
  live = bar;
  target = level;
  shown = Math.max(shown, level);
  stamp = performance.now();
};

const frame = (now) => {
  const t = Math.min(1, (now - stamp) / TICK);
  strip.style.transform = reduced ? "" : "translateX(" + (PITCH * (1 - t)).toFixed(2) + "px)";
  if (live) {
    shown = target > shown ? target : shown + (target - shown) * Math.min(1, 16 / RELEASE);
    live.style.height = px(shown);
  }
  raf = requestAnimationFrame(frame);
};
raf = requestAnimationFrame(frame);
window.__ffVoiceStop = () => cancelAnimationFrame(raf);

for (;;) {
  const slice = await dioxus.recv();
  if (!slice || !host.isConnected) break;
  push(slice[0], slice[1]);
  // Proof line for the Rust log: bars really are in the DOM with a size.
  if (bars.length === 40) {
    // Proof line for the Rust log (FLOWFLOW_VOICE_PROBE=1, debug builds):
    // bars are in the DOM, 2 px wide, reacting to sound. CSS transitions
    // cannot be judged from that run: a webview launched from a shell is
    // `visibilityState: hidden` and WebKit freezes its animation clock.
    const r = bars[39].getBoundingClientRect();
    const tall = bars.filter((b) => b.getBoundingClientRect().height > 4).length;
    const layer = host.closest(".voice-layer");
    dioxus.send("timeline:bars=" + bars.length + " w=" + r.width + " tall=" + tall
      + " layer_in=" + (layer && layer.getAttribute("data-in"))
      + " visibility=" + document.visibilityState);
  }
}
cancelAnimationFrame(raf);
