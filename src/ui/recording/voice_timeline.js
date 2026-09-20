// src/ui/recording/voice_timeline.ts
(function() {
  const TICK = __TICK__;
  const PITCH = 4;
  const HALF = 13;
  const RELEASE = 120;
  const host = document.querySelector(".voice-timeline");
  const strip = host?.querySelector(".voice-bars");
  if (!host || !strip)
    return;
  const w = window;
  w.__ffVoiceStop?.();
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const cap = Math.ceil(host.clientWidth / PITCH) + 4;
  const bars = Array.from(strip.children);
  let live = null;
  let target = 0;
  let shown = 0;
  let stamp = performance.now();
  let raf = 0;
  const px = (level) => `${Math.max(2, level * HALF * 2).toFixed(1)}px`;
  const push = (rms, peak) => {
    const level = Math.min(1, Math.max(rms, peak * 0.8));
    if (live)
      live.style.height = px(target);
    const bar = document.createElement("i");
    bar.style.height = px(level);
    strip.appendChild(bar);
    bars.push(bar);
    while (bars.length > cap)
      bars.shift().remove();
    live = bar;
    target = level;
    shown = Math.max(shown, level);
    stamp = performance.now();
  };
  const frame = (now) => {
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
    for (;; ) {
      const slice = await dioxus.recv();
      if (!slice || !host.isConnected)
        break;
      push(slice[0], slice[1]);
    }
    cancelAnimationFrame(raf);
  })();
})();
