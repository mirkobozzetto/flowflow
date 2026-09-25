// src/ui/app/glass_burger.ts
(function() {
  const w = window;
  w.__glassSend = (m) => dioxus.send(m);
  if (w.__glassInstalled)
    return;
  w.__glassInstalled = true;
  const send = (m) => w.__glassSend(m);
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
  const lastPlace = new Map;
  const shown = new Map;
  const lastMenu = new Map;
  w.__ffMenuPick = (id, context, action) => {
    if (id !== "note-more" && id !== "chat-more")
      return;
    const anchor = document.querySelector(`[data-glass="${id}"]`);
    const root = document.querySelector(`[data-native-menu="${id}"]`);
    if (!anchor || !root || root.dataset.nativeContext !== context)
      return;
    const r = anchor.getBoundingClientRect();
    const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    if (!hit || !anchor.contains(hit))
      return;
    const button = Array.from(root.querySelectorAll("[data-native-action]")).find((b) => b.dataset.nativeAction === action);
    if (button && !button.disabled)
      button.click();
  };
  function menuFor(id, anchor) {
    if (id !== "note-more" && id !== "chat-more")
      return null;
    const root = document.querySelector(`[data-native-menu="${id}"]`);
    return {
      id,
      context: root?.dataset.nativeContext ?? "",
      label: anchor.getAttribute("aria-label") ?? "",
      items: root ? Array.from(root.querySelectorAll("[data-native-action]")).map((button) => ({
        id: button.dataset.nativeAction,
        title: button.textContent?.trim() ?? "",
        symbol: button.dataset.nativeSymbol ?? "ellipsis",
        disabled: button.disabled,
        destructive: button.hasAttribute("data-native-destructive")
      })) : []
    };
  }
  const ready = () => {
    document.documentElement.dataset.glassReady = "1";
  };
  setTimeout(ready, 3000);
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
    const c = card();
    const dx = c ? shift(c) : 0;
    const vv = window.visualViewport;
    const seen = new Set;
    document.querySelectorAll("[data-glass]").forEach((a) => {
      const id = a.dataset.glass;
      const menu = menuFor(id, a);
      seen.add(id);
      const r = a.getBoundingClientRect();
      let visible = shown.get(id) ?? false;
      if (Math.abs(dx) < 0.5) {
        const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        visible = r.width > 0 && !!hit && (a.contains(hit) || !!hit.closest("[data-glass-pass]"));
      }
      if (menu && !menu.items.length)
        visible = false;
      shown.set(id, visible);
      a.toggleAttribute("data-glass-on", visible);
      if (visible)
        ready();
      const x = r.left - dx - (vv ? vv.offsetLeft : 0);
      const y = r.top - (vv ? vv.offsetTop : 0);
      const dot = a.querySelector("[data-badge]") ? 1 : 0;
      post(id, [x, y, r.width, r.height, travel(), visible ? 1 : 0, dot]);
      if (menu) {
        const json = JSON.stringify(menu);
        if (lastMenu.get(id) !== json) {
          lastMenu.set(id, json);
          send("menu " + json);
        }
      }
    });
    for (const id of lastPlace.keys()) {
      if (!seen.has(id)) {
        shown.set(id, false);
        post(id, [0, 0, 0, 0, travel(), 0, 0]);
        if (lastMenu.delete(id)) {
          send("menu " + JSON.stringify({ id, context: "", items: [] }));
        }
      }
    }
  }
  function post(id, v) {
    const msg = "place " + id + " " + v.join(" ");
    if (lastPlace.get(id) === msg)
      return;
    lastPlace.set(id, msg);
    send(msg);
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
    characterData: true,
    attributeFilter: ["class", "disabled", "hidden", "data-native-context", "aria-label"]
  });
  window.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("scroll", schedule);
  document.addEventListener("transitionend", schedule, true);
  watchCard();
  schedule();
})();
