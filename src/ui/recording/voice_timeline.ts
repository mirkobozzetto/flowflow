// Voice capsule timeline (webview only). Rust pushes one (rms, peak) slice
// every __TICK__ ms through dioxus.recv; this controller owns the DOM. A rAF
// loop slides the strip in from the right edge between two slices (sub-pixel,
// 60 fps) and lets the newest bar breathe (instant attack, ~120 ms release);
// older bars are frozen at their true value, so the timeline stays honest.
// Bars are symmetric around the axis, Voice Memos style. The strip survives a
// pause: reinstalling only replaces the loop, never the bars.

(async function () {
  const TICK = __TICK__;
  const PITCH = 4; // 2 px bar + 2 px gap
  const HALF = 13; // max half-height in px (track is 28 px tall)
  const RELEASE = 120;
  // The eval runs from the Rust effect before Dioxus has flushed the capsule
  // into the DOM: poll a few frames for the node instead of giving up.
  let host: HTMLElement | null = null;
  for (let i = 0; i < 30 && !host; i++) {
    host = document.querySelector<HTMLElement>(".voice-timeline");
    if (!host) await new Promise((r) => requestAnimationFrame(r));
  }
  const strip = host?.querySelector<HTMLElement>(".voice-bars");
  if (!host || !strip) return;

  const w = window as unknown as { __ffVoiceStop?: () => void };
  w.__ffVoiceStop?.();

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const cap = Math.ceil(Math.max(host.clientWidth, 200) / PITCH) + 4;
  const bars = Array.from(strip.children) as HTMLElement[];
  let live: HTMLElement | null = null;
  let target = 0;
  let shown = 0;
  let stamp = performance.now();
  let raf = 0;
  const px = (level: number) => `${Math.max(2, level * HALF * 2).toFixed(1)}px`;

  const push = (rms: number, peak: number) => {
    const level = Math.min(1, Math.max(rms, peak * 0.8));
    if (live) live.style.height = px(target);
    const bar = document.createElement("i");
    bar.style.height = px(level);
    strip.appendChild(bar);
    bars.push(bar);
    while (bars.length > cap) bars.shift()!.remove();
    live = bar;
    target = level;
    shown = Math.max(shown, level);
    stamp = performance.now();
  };

  const frame = (now: number) => {
    const t = Math.min(1, (now - stamp) / TICK);
    strip.style.transform = reduced ? "" : `translateX(${(PITCH * (1 - t)).toFixed(2)}px)`;
    if (live) {
      shown = target > shown ? target : shown + (target - shown) * Math.min(1, 16 / RELEASE);
      live.style.height = px(shown);
    }
    raf = requestAnimationFrame(frame);
  };
  raf = requestAnimationFrame(frame);
  w.__ffVoiceStop = () => cancelAnimationFrame(raf);

  (async () => {
    for (;;) {
      const slice = (await dioxus.recv()) as [number, number] | null;
      if (!slice || !host.isConnected) break;
      push(slice[0], slice[1]);
    }
    cancelAnimationFrame(raf);
  })();
})();
