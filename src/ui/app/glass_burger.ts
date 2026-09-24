// #177: reports the web anchors ([data-glass="<id>"]: burger, chat pill,
// new-note button) to the native iOS 26 Liquid Glass buttons laid over the
// web view (src/infrastructure/platform/ios/glass_burger.rs). Each anchor
// keeps its place, its click handler and, whenever its native button cannot
// show, its own look. Messages:
//   "place id x y w h travel visible dot"  resting place (card closed)
//   "drag p"                                the card follows the finger
//   "settle p"                              the card glides to 0 or 1
// Source of truth: this .ts file; compile to glass_burger.js with `make js`.

(function () {
  const w = window as unknown as Record<string, unknown>;
  // A re-eval takes over the channel; listeners are installed once.
  w.__glassSend = (m: string) => dioxus.send(m);
  if (w.__glassInstalled) return;
  w.__glassInstalled = true;
  const send = (m: string) => (w.__glassSend as (m: string) => void)(m);

  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
  const lastPlace = new Map<string, string>();
  const shown = new Map<string, boolean>();
  const lastMenu = new Map<string, string>();

  // Reuse the existing, mounted web actions. The per-mount context prevents a
  // stale UIKit menu from targeting another note/chat after navigation.
  w.__ffMenuPick = (id: string, context: string, action: string) => {
    if (id !== "note-more" && id !== "chat-more") return;
    const anchor = document.querySelector<HTMLElement>(`[data-glass="${id}"]`);
    const root = document.querySelector<HTMLElement>(`[data-native-menu="${id}"]`);
    if (!anchor || !root || root.dataset.nativeContext !== context) return;
    const r = anchor.getBoundingClientRect();
    const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    if (!hit || !anchor.contains(hit)) return;
    const button = Array.from(root.querySelectorAll<HTMLButtonElement>("[data-native-action]"))
      .find((b) => b.dataset.nativeAction === action);
    if (button && !button.disabled) button.click();
  };

  function menuFor(id: string, anchor: HTMLElement) {
    if (id !== "note-more" && id !== "chat-more") return null;
    const root = document.querySelector<HTMLElement>(`[data-native-menu="${id}"]`);
    return {
      id,
      context: root?.dataset.nativeContext ?? "",
      label: anchor.getAttribute("aria-label") ?? "",
      items: root ? Array.from(root.querySelectorAll<HTMLButtonElement>("[data-native-action]"))
        .map((button) => ({
          id: button.dataset.nativeAction!,
          title: button.textContent?.trim() ?? "",
          symbol: button.dataset.nativeSymbol ?? "ellipsis",
          disabled: button.disabled,
          destructive: button.hasAttribute("data-native-destructive"),
        })) : [],
    };
  }
  let lastTarget = -1;
  let raf = 0;

  const card = () => document.getElementById("main-card");
  const travel = () => window.innerWidth * 0.82;
  const px = (t: string) => {
    const m = /translateX\((-?[\d.]+)px\)/.exec(t);
    return m ? parseFloat(m[1]) : 0;
  };
  function shift(c: HTMLElement): number {
    const t = getComputedStyle(c).transform;
    return t && t !== "none" ? new DOMMatrix(t).m41 : 0;
  }

  function measure(): void {
    raf = 0;
    const c = card();
    const dx = c ? shift(c) : 0;
    const vv = window.visualViewport;
    const seen = new Set<string>();
    document.querySelectorAll<HTMLElement>("[data-glass]").forEach((a) => {
      const id = a.dataset.glass as string;
      const menu = menuFor(id, a);
      seen.add(id);
      const r = a.getBoundingClientRect();
      // Visibility is only re-judged with the card at rest: mid-slide an
      // anchor can leave the screen and must not flip look on the way.
      let visible = shown.get(id) ?? false;
      if (Math.abs(dx) < 0.5) {
        // Shown only when the anchor is what the finger would hit: any
        // overlay (sheet, modal, splash) hands the look back to the web one.
        const hit = document.elementFromPoint(
          r.left + r.width / 2,
          r.top + r.height / 2,
        );
        visible =
          r.width > 0 &&
          !!hit &&
          (a.contains(hit) || !!hit.closest("[data-glass-pass]"));
      }
      if (menu && !menu.items.length) visible = false;
      shown.set(id, visible);
      a.toggleAttribute("data-glass-on", visible);
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
    // An anchor gone from the page (inner view) hides its button.
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
  function post(id: string, v: number[]): void {
    const msg = "place " + id + " " + v.join(" ");
    if (lastPlace.get(id) === msg) return;
    lastPlace.set(id, msg);
    send(msg);
  }
  function schedule(): void {
    if (!raf) raf = requestAnimationFrame(measure);
  }

  // Card motion, read from what swipe.ts and the .sb-card class write.
  function onCard(): void {
    const c = card();
    if (!c) return;
    const inline = c.style.transform;
    if (inline && c.style.transition === "none") {
      lastTarget = -1;
      send("drag " + (px(inline) / travel()).toFixed(4));
      return;
    }
    const target = inline
      ? px(inline) > 1
        ? 1
        : 0
      : c.dataset.open === "1"
        ? 1
        : 0;
    if (target === lastTarget) return;
    lastTarget = target;
    send((reduced.matches ? "drag " : "settle ") + target);
  }

  let watched: HTMLElement | null = null;
  const cardObserver = new MutationObserver(onCard);
  function watchCard(): void {
    const c = card();
    if (c === watched) return;
    cardObserver.disconnect();
    watched = c;
    if (c) {
      cardObserver.observe(c, {
        attributes: true,
        attributeFilter: ["style", "data-open"],
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
    attributeFilter: ["class", "disabled", "hidden", "data-native-context", "aria-label"],
  });
  window.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("scroll", schedule);
  document.addEventListener("transitionend", schedule, true);
  watchCard();
  schedule();
})();
