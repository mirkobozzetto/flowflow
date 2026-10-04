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

  const MENU_IDS = ["note-more", "chat-more", "note-plus", "chat-plus", "chats-plus"];

  // The native menu opened or closed: the composer's "+" turns into a cross.
  w.__ffMenuOpen = (open: boolean) => {
    document.querySelectorAll<HTMLElement>('[data-glass$="-plus"]')
      .forEach((a) => a.toggleAttribute("data-native-open", open));
  };

  // Reuse the existing, mounted web actions. The per-mount context prevents a
  // stale UIKit menu from targeting another note/chat after navigation.
  w.__ffMenuPick = (id: string, context: string, action: string) => {
    if (!MENU_IDS.includes(id)) return;
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

  type NativeItem = {
    id?: string;
    title: string;
    symbol: string;
    subtitle?: string;
    disabled?: boolean;
    destructive?: boolean;
    checked?: boolean;
    inline?: boolean;
    image?: string;
    children?: NativeItem[];
  };
  type Icon = [NativeItem, HTMLElement];

  // The app's own icon for an entry ([data-native-icon] holder), drawn to a
  // 3x PNG the native menu shows with its colours. "disc" puts it on a dark
  // round tile, as the web menu does for the Exa mark.
  const ICON_PT = 22;
  const ICON_SCALE = 3;
  const drawn = new Map<string, string>();
  async function raster(holder: HTMLElement): Promise<string | undefined> {
    const node = holder.firstElementChild;
    const size = ICON_PT * ICON_SCALE;
    let src: string;
    if (node instanceof HTMLImageElement) {
      src = node.src;
    } else if (node instanceof SVGSVGElement) {
      const svg = node.cloneNode(true) as SVGSVGElement;
      svg.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      svg.setAttribute("width", String(size));
      svg.setAttribute("height", String(size));
      svg.style.color = getComputedStyle(holder).color;
      src = "data:image/svg+xml;charset=utf-8," +
        encodeURIComponent(new XMLSerializer().serializeToString(svg));
    } else {
      return undefined;
    }
    const disc = holder.dataset.nativeIcon === "disc";
    const key = (disc ? "disc|" : "") + src;
    const hit = drawn.get(key);
    if (hit) return hit;
    try {
      const img = new Image();
      img.src = src;
      await img.decode();
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = size;
      const ctx = canvas.getContext("2d")!;
      let box = size;
      if (disc) {
        ctx.fillStyle = "#1c1917";
        ctx.beginPath();
        ctx.arc(size / 2, size / 2, size / 2, 0, Math.PI * 2);
        ctx.fill();
        box = size * 0.5;
      }
      const w = img.naturalWidth || size;
      const h = img.naturalHeight || size;
      const k = box / Math.max(w, h);
      ctx.drawImage(img, (size - w * k) / 2, (size - h * k) / 2, w * k, h * k);
      const url = canvas.toDataURL("image/png");
      drawn.set(key, url);
      return url;
    } catch {
      // No bitmap: the native menu falls back to the entry's symbol.
      return undefined;
    }
  }

  // Walks the hidden DOM menu in order: a [data-native-action] button is an
  // action, a [data-native-submenu] element a submenu (a separated group when
  // data-native-inline), anything else is looked through.
  function items(el: Element, icons: Icon[]): NativeItem[] {
    const out: NativeItem[] = [];
    const iconOf = (item: NativeItem, owner: HTMLElement) => {
      const holder = owner.querySelector<HTMLElement>(":scope > [data-native-icon]");
      if (holder) icons.push([item, holder]);
      return item;
    };
    for (const child of Array.from(el.children) as HTMLElement[]) {
      if (child.dataset.nativeIcon !== undefined) continue;
      if (child.dataset.nativeAction !== undefined) {
        const button = child as HTMLButtonElement;
        out.push(iconOf({
          id: button.dataset.nativeAction,
          title: button.dataset.nativeTitle ?? button.textContent?.trim() ?? "",
          symbol: button.dataset.nativeSymbol ?? "ellipsis",
          subtitle: button.dataset.nativeSubtitle,
          disabled: button.disabled,
          destructive: button.hasAttribute("data-native-destructive"),
          checked: button.dataset.nativeChecked === "true",
        }, button));
      } else if (child.dataset.nativeSubmenu !== undefined) {
        out.push(iconOf({
          title: child.dataset.nativeTitle ?? "",
          symbol: child.dataset.nativeSymbol ?? "",
          inline: child.hasAttribute("data-native-inline"),
          children: items(child, icons),
        }, child));
      } else {
        out.push(...items(child, icons));
      }
    }
    return out;
  }

  function menuFor(id: string, anchor: HTMLElement) {
    if (!MENU_IDS.includes(id)) return null;
    const root = document.querySelector<HTMLElement>(`[data-native-menu="${id}"]`);
    const icons: Icon[] = [];
    const menu = {
      id,
      context: root?.dataset.nativeContext ?? "",
      label: anchor.getAttribute("aria-label") ?? "",
      items: root ? items(root, icons) : [],
    };
    return { menu, icons };
  }
  const ready = () => {
    document.documentElement.dataset.glassReady = "1";
  };
  // Never keep the anchors hidden if no native button ever shows.
  setTimeout(ready, 3000);
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
      const found = menuFor(id, a);
      const menu = found?.menu;
      seen.add(id);
      const r = a.getBoundingClientRect();
      // Visibility is only re-judged with the card at rest: mid-slide an
      // anchor can leave the screen and must not flip look on the way.
      let visible = shown.get(id) ?? false;
      // An anchor in the menu under the card is reachable only with the
      // card fully aside: judge it at that rest too.
      const under = !!a.closest(".sb-under");
      if (Math.abs(dx) < 0.5 || (under && Math.abs(dx - travel()) < 0.5)) {
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
      // A native button is on screen: the anchors held transparent at
      // launch (router.rs, data-glass-wait) now follow data-glass-on alone.
      if (visible) ready();
      const x = r.left - dx - (vv ? vv.offsetLeft : 0);
      const y = r.top - (vv ? vv.offsetTop : 0);
      const dot = a.querySelector("[data-badge]") ? 1 : 0;
      post(id, [x, y, r.width, r.height, travel(), visible ? 1 : 0, dot]);
      if (found && menu) {
        // Compare the menu without its bitmaps; send it once they are drawn,
        // unless a newer menu for this anchor replaced it meanwhile.
        const json = JSON.stringify(menu);
        if (lastMenu.get(id) !== json) {
          lastMenu.set(id, json);
          Promise.all(found.icons.map(async ([item, holder]) => {
            item.image = await raster(holder);
          })).then(() => {
            if (lastMenu.get(id) === json) send("menu " + JSON.stringify(menu));
          });
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
    attributeFilter: ["class", "disabled", "hidden", "data-native-context", "data-native-checked", "aria-label"],
  });
  window.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("resize", schedule);
  window.visualViewport?.addEventListener("scroll", schedule);
  // The menu's lists scroll their anchors without any DOM mutation.
  document.addEventListener("scroll", schedule, true);
  document.addEventListener("transitionend", schedule, true);
  watchCard();
  schedule();
})();
