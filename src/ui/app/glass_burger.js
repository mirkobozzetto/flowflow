// src/ui/app/glass_burger.ts
(function() {
  const w = window;
  w.__glassSend = (m) => dioxus.send(m);
  if (w.__glassInstalled)
    return;
  w.__glassInstalled = true;
  const send = (m) => w.__glassSend(m);
  const root = document.documentElement;
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
  let lastPlace = "";
  let lastTarget = -1;
  let raf = 0;
  const card = () => document.getElementById("main-card");
  const travel = () => window.innerWidth * 0.82;
  const px = (t) => {
    const m = /translateX\((-?[\d.]+)px\)/.exec(t);
    return m ? parseFloat(m[1]) : 0;
  };
  function shift(c) {
    const t = getComputedStyle(c).transform;
    return t && t !== "none" ? new DOMMatrix(t).m41 : 0;
  }
  function measure() {
    raf = 0;
    const b = document.getElementById("burger");
    const c = card();
    let msg = "place 0 0 0 0 0 0";
    let visible = false;
    if (b && c) {
      const r = b.getBoundingClientRect();
      const vv = window.visualViewport;
      const x = r.left - shift(c) - (vv ? vv.offsetLeft : 0);
      const y = r.top - (vv ? vv.offsetTop : 0);
      const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      visible = r.width > 0 && !!hit && (b.contains(hit) || !!hit.closest("[data-glass-pass]"));
      const dot = b.querySelector("[data-badge]") ? 1 : 0;
      msg = ["place", x, y, r.width, travel(), visible ? 1 : 0, dot].join(" ");
    }
    root.classList.toggle("glass-burger", visible);
    if (msg !== lastPlace) {
      lastPlace = msg;
      send(msg);
    }
  }
  function schedule() {
    if (!raf)
      raf = requestAnimationFrame(measure);
  }
  function onCard() {
    const c = card();
    if (!c)
      return;
    const inline = c.style.transform;
    if (inline && c.style.transition === "none") {
      lastTarget = -1;
      send("drag " + (px(inline) / travel()).toFixed(4));
      return;
    }
    const target = inline ? px(inline) > 1 ? 1 : 0 : c.dataset.open === "1" ? 1 : 0;
    if (target === lastTarget)
      return;
    lastTarget = target;
    send((reduced.matches ? "drag " : "settle ") + target);
  }
  let watched = null;
  const cardObserver = new MutationObserver(onCard);
  function watchCard() {
    const c = card();
    if (c === watched)
      return;
    cardObserver.disconnect();
    watched = c;
    if (c) {
      cardObserver.observe(c, {
        attributes: true,
        attributeFilter: ["style", "data-open"]
      });
      onCard();
    }
  }
  new MutationObserver(() => {
    watchCard();
    schedule();
  }).observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["class"]
  });
  window.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("scroll", schedule);
  document.addEventListener("transitionend", schedule, true);
  watchCard();
  schedule();
})();
