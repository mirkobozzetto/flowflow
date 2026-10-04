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
  const MENU_IDS = ["note-more", "chat-more", "note-plus", "chat-plus", "chats-plus"];
  w.__ffMenuOpen = (open) => {
    document.querySelectorAll('[data-glass$="-plus"]').forEach((a) => a.toggleAttribute("data-native-open", open));
  };
  w.__ffMenuPick = (id, context, action) => {
    if (!MENU_IDS.includes(id))
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
  const ICON_PT = 22;
  const ICON_SCALE = 3;
  const drawn = new Map;
  async function raster(holder) {
    const node = holder.firstElementChild;
    const size = ICON_PT * ICON_SCALE;
    let src;
    if (node instanceof HTMLImageElement) {
      src = node.src;
    } else if (node instanceof SVGSVGElement) {
      const svg = node.cloneNode(true);
      svg.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      svg.setAttribute("width", String(size));
      svg.setAttribute("height", String(size));
      svg.style.color = getComputedStyle(holder).color;
      src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(new XMLSerializer().serializeToString(svg));
    } else {
      return;
    }
    const disc = holder.dataset.nativeIcon === "disc";
    const key = (disc ? "disc|" : "") + src;
    const hit = drawn.get(key);
    if (hit)
      return hit;
    try {
      const img = new Image;
      img.src = src;
      await img.decode();
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = size;
      const ctx = canvas.getContext("2d");
      let box = size;
      if (disc) {
        ctx.fillStyle = "#1c1917";
        ctx.beginPath();
        ctx.arc(size / 2, size / 2, size / 2, 0, Math.PI * 2);
        ctx.fill();
        box = size * 0.5;
      }
      const w2 = img.naturalWidth || size;
      const h = img.naturalHeight || size;
      const k = box / Math.max(w2, h);
      ctx.drawImage(img, (size - w2 * k) / 2, (size - h * k) / 2, w2 * k, h * k);
      const url = canvas.toDataURL("image/png");
      drawn.set(key, url);
      return url;
    } catch {
      return;
    }
  }
  function items(el, icons) {
    const out = [];
    const iconOf = (item, owner) => {
      const holder = owner.querySelector(":scope > [data-native-icon]");
      if (holder)
        icons.push([item, holder]);
      return item;
    };
    for (const child of Array.from(el.children)) {
      if (child.dataset.nativeIcon !== undefined)
        continue;
      if (child.dataset.nativeAction !== undefined) {
        const button = child;
        out.push(iconOf({
          id: button.dataset.nativeAction,
          title: button.dataset.nativeTitle ?? button.textContent?.trim() ?? "",
          symbol: button.dataset.nativeSymbol ?? "ellipsis",
          subtitle: button.dataset.nativeSubtitle,
          disabled: button.disabled,
          destructive: button.hasAttribute("data-native-destructive"),
          checked: button.dataset.nativeChecked === "true"
        }, button));
      } else if (child.dataset.nativeSubmenu !== undefined) {
        out.push(iconOf({
          title: child.dataset.nativeTitle ?? "",
          symbol: child.dataset.nativeSymbol ?? "",
          inline: child.hasAttribute("data-native-inline"),
          children: items(child, icons)
        }, child));
      } else {
        out.push(...items(child, icons));
      }
    }
    return out;
  }
  function menuFor(id, anchor) {
    if (!MENU_IDS.includes(id))
      return null;
    const root = document.querySelector(`[data-native-menu="${id}"]`);
    const icons = [];
    const menu = {
      id,
      context: root?.dataset.nativeContext ?? "",
      label: anchor.getAttribute("aria-label") ?? "",
      items: root ? items(root, icons) : []
    };
    return { menu, icons };
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
      const found = menuFor(id, a);
      const menu = found?.menu;
      seen.add(id);
      const r = a.getBoundingClientRect();
      let visible = shown.get(id) ?? false;
      const under = !!a.closest(".sb-under");
      if (Math.abs(dx) < 0.5 || under && Math.abs(dx - travel()) < 0.5) {
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
      if (found && menu) {
        const json = JSON.stringify(menu);
        if (lastMenu.get(id) !== json) {
          lastMenu.set(id, json);
          Promise.all(found.icons.map(async ([item, holder]) => {
            item.image = await raster(holder);
          })).then(() => {
            if (lastMenu.get(id) === json)
              send("menu " + JSON.stringify(menu));
          });
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
    attributeFilter: ["class", "disabled", "hidden", "data-native-context", "data-native-checked", "aria-label"]
  });
  window.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("scroll", schedule);
  document.addEventListener("scroll", schedule, true);
  document.addEventListener("transitionend", schedule, true);
  watchCard();
  schedule();
})();
