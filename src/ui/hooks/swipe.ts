// Reusable edge-swipe drawer controller (webview only; no DOM => no dioxus-native).
//
// Native pointer listeners drive the panel's CSS `translate` directly (no
// per-frame round-trip through Dioxus), then send the committed open/closed
// state to Rust via dioxus.send. The CALLER renders three things: a panel
// (#<panelId>) positioned off-screen via a Tailwind translate class, a backdrop
// (#<backdropId>), and a thin edge element with `touch-action: none` over the
// edge zone (so the open swipe is not stolen by native scroll). The panel keeps
// a `data-open` attribute (0/1) that Dioxus syncs to the open state.
//
// Config placeholders are filled by the Rust hook (use_swipe_drawer) before eval.
// Source of truth: this .ts file; compile to swipe.js with `make js` (bun).

declare const dioxus: { send(msg: string): void };
// Numeric config placeholders are bare identifiers (declared, never defined) so
// the bun build cannot constant-fold them (e.g. `+"__EDGE_PX__"` would fold to
// NaN); the Rust hook replaces each identifier with a number literal pre-eval.
declare const __EDGE_PX__: number;
declare const __OPEN_AT__: number;
declare const __CLOSE_AT__: number;

(function () {
  const CFG = {
    panelId: "__PANEL__",
    backdropId: "__BACKDROP__",
    side: "__EDGE__", // "left" | "right"
    edgePx: __EDGE_PX__,
    openAt: __OPEN_AT__,
    closeAt: __CLOSE_AT__,
  };
  const FLICK = 0.4; // px/ms velocity that commits a flick
  // Closed position is sign*w (left: -w, right: +w); open is 0.
  const sign = CFG.side === "left" ? -1 : 1;

  const w0 = window as unknown as Record<string, boolean>;
  const guard = "__swipe_" + CFG.panelId;
  if (w0[guard]) return;
  w0[guard] = true;

  let panel: HTMLElement | null = null;
  let backdrop: HTMLElement | null = null;
  let w = 0;
  let active = false;
  let opening = false;
  let engaged = false;
  let startX = 0;
  let startY = 0;
  let lastX = 0;
  let lastT = 0;
  let vel = 0;
  let cur = 0;
  let raf = 0;

  function els(): boolean {
    panel = document.getElementById(CFG.panelId);
    backdrop = document.getElementById(CFG.backdropId);
    return !!panel && !!backdrop;
  }
  function apply(): void {
    raf = 0;
    if (!panel) return;
    // Tailwind v4 positions the panel with the CSS `translate` property, so we
    // must drive the same property to override it (an inline transform stacks).
    panel.style.transition = "none";
    panel.style.translate = cur + "px";
    if (backdrop) {
      const p = Math.max(0, Math.min(1, 1 - Math.abs(cur) / w));
      backdrop.style.transition = "none";
      backdrop.style.opacity = p.toFixed(3);
    }
  }
  function schedule(): void {
    if (!raf) raf = requestAnimationFrame(apply);
  }
  function settle(open: boolean): void {
    if (!panel) return;
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    panel.style.transition = "translate .25s cubic-bezier(.32,.72,0,1)";
    panel.style.translate = (open ? 0 : sign * w) + "px";
    if (backdrop) {
      backdrop.style.transition = "opacity .25s ease";
      backdrop.style.opacity = open ? "1" : "0";
    }
    dioxus.send(open ? "open" : "closed");
    // Hand control back to Dioxus only once it has applied the committed state
    // (data-open flips). On a busy page Rust may process the message after the
    // snap animation ends; clearing the inline style before that would let the
    // stale class position flash (reopen/close flicker).
    const want = open ? "1" : "0";
    let tries = 0;
    const tryClear = () => {
      if (!panel) return;
      if (panel.dataset.open === want || tries++ > 240) {
        panel.style.transition = "";
        panel.style.translate = "";
        if (backdrop) {
          backdrop.style.transition = "";
          backdrop.style.opacity = "";
        }
      } else {
        requestAnimationFrame(tryClear);
      }
    };
    setTimeout(() => requestAnimationFrame(tryClear), 260);
  }
  function onDown(e: PointerEvent): void {
    if (e.pointerType === "mouse" || active || !els() || !panel) return;
    w = panel.offsetWidth || 300;
    const open = panel.dataset.open === "1";
    if (!open) {
      const atEdge =
        CFG.side === "left"
          ? e.clientX <= CFG.edgePx
          : e.clientX >= window.innerWidth - CFG.edgePx;
      if (!atEdge) return;
      active = true;
      opening = true;
      engaged = true;
      cur = sign * w;
      schedule();
    } else {
      active = true;
      opening = false;
      engaged = false;
      cur = 0;
    }
    startX = lastX = e.clientX;
    startY = e.clientY;
    lastT = e.timeStamp;
    vel = 0;
  }
  function onMove(e: PointerEvent): void {
    if (!active || !panel) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    if (!engaged) {
      const towardClose = CFG.side === "left" ? dx < -8 : dx > 8;
      if (towardClose && Math.abs(dx) > Math.abs(dy)) {
        engaged = true;
      } else if (Math.abs(dy) > 8) {
        active = false;
        return;
      } else {
        return;
      }
    }
    const dt = e.timeStamp - lastT;
    if (dt > 0) vel = (e.clientX - lastX) / dt;
    lastX = e.clientX;
    lastT = e.timeStamp;
    const lo = Math.min(0, sign * w);
    const hi = Math.max(0, sign * w);
    cur = opening
      ? Math.max(lo, Math.min(hi, sign * w + dx))
      : Math.max(lo, Math.min(hi, dx));
    if (e.cancelable) e.preventDefault();
    schedule();
  }
  function onUp(): void {
    if (!active || !panel) return;
    const wasEngaged = engaged;
    active = false;
    engaged = false;
    if (!wasEngaged) return;
    // Swallow the click the browser synthesizes on the element the drag ended
    // over, so a swipe (e.g. over a list row) does not also fire its onclick.
    const swallow = (ev: Event) => {
      ev.stopPropagation();
      ev.preventDefault();
      document.removeEventListener("click", swallow, true);
    };
    document.addEventListener("click", swallow, true);
    setTimeout(() => document.removeEventListener("click", swallow, true), 350);
    const progress = 1 - Math.abs(cur) / w;
    const openByVel = CFG.side === "left" ? vel > 0 : vel < 0;
    const thresh = opening ? CFG.openAt : CFG.closeAt;
    const open = Math.abs(vel) > FLICK ? openByVel : progress > thresh;
    settle(open);
  }

  document.addEventListener("pointerdown", onDown, true);
  document.addEventListener("pointermove", onMove, {
    passive: false,
    capture: true,
  });
  document.addEventListener("pointerup", onUp, true);
  document.addEventListener("pointercancel", onUp, true);
})();
