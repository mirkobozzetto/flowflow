// Bottom-sheet drag-to-dismiss controller (webview only; no DOM => no dioxus-native).
//
// Same finger-follow mechanics as the edge drawer (native pointer listeners +
// rAF + driving CSS `translate` directly, never a per-frame Dioxus round-trip),
// but vertical and transient: the sheet is unmounted by Rust on dismiss, so
// there is no persistent open state / data-open handoff. The drag engages only
// when it starts on the grabber strip (top __GRAB_PX__ px of the sheet), so the
// menu rows below stay tappable/scrollable. On release past the threshold (or a
// downward flick) the sheet animates out and Rust unmounts it; otherwise it
// snaps back.
//
// The CALLER renders a sheet (#<sheetId>) pinned to the bottom and a backdrop
// (#<backdropId>). Config placeholders are filled by the Rust hook
// (use_sheet_dismiss) before eval. Source of truth: this .ts; `make js` -> .js.

(function () {
  const CFG = {
    sheetId: "__SHEET__",
    backdropId: "__BACKDROP__",
    grabPx: __GRAB_PX__,
    dismissAt: __DISMISS_AT__,
  };
  const FLICK = 0.5; // px/ms downward velocity that commits a dismiss

  // The sheet remounts on every menu open, so a one-shot window guard would
  // block re-attach. Instead tear down the previous controller's listeners
  // before installing fresh ones (no accumulation across opens).
  const w0 = window as unknown as Record<string, (() => void) | undefined>;
  const KEY = "__sheet_" + CFG.sheetId;
  const prev = w0[KEY];
  if (prev) prev();

  let sheet: HTMLElement | null = null;
  let backdrop: HTMLElement | null = null;
  let h = 0;
  let active = false;
  let startY = 0;
  let lastY = 0;
  let lastT = 0;
  let vel = 0;
  let cur = 0;
  let raf = 0;

  function apply(): void {
    raf = 0;
    if (!sheet) return;
    sheet.style.transition = "none";
    sheet.style.translate = "0 " + cur + "px";
    if (backdrop) {
      const p = Math.max(0, Math.min(1, 1 - cur / h));
      backdrop.style.transition = "none";
      backdrop.style.opacity = p.toFixed(3);
    }
  }
  function schedule(): void {
    if (!raf) raf = requestAnimationFrame(apply);
  }
  function dismiss(): void {
    if (!sheet) return;
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    sheet.style.transition = "translate .22s cubic-bezier(.32,.72,0,1)";
    sheet.style.translate = "0 " + h + "px";
    if (backdrop) {
      backdrop.style.transition = "opacity .22s ease";
      backdrop.style.opacity = "0";
    }
    // Let the slide-out play, then ask Rust to unmount (no flash: the element is
    // already off-screen when it disappears).
    setTimeout(() => dioxus.send("closed"), 210);
  }
  function snapBack(): void {
    if (!sheet) return;
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    sheet.style.transition = "translate .22s cubic-bezier(.32,.72,0,1)";
    sheet.style.translate = "0 0px";
    if (backdrop) {
      backdrop.style.transition = "opacity .22s ease";
      backdrop.style.opacity = "1";
    }
    setTimeout(() => {
      if (!sheet) return;
      sheet.style.transition = "";
      sheet.style.translate = "";
      if (backdrop) {
        backdrop.style.transition = "";
        backdrop.style.opacity = "";
      }
    }, 240);
  }
  function onDown(e: PointerEvent): void {
    if (e.pointerType === "mouse" || active) return;
    sheet = document.getElementById(CFG.sheetId);
    backdrop = document.getElementById(CFG.backdropId);
    if (!sheet) return;
    const rect = sheet.getBoundingClientRect();
    h = rect.height || 300;
    // Engage only on the grabber strip; below it the menu rows handle their taps.
    if (e.clientY > rect.top + CFG.grabPx) return;
    active = true;
    startY = lastY = e.clientY;
    lastT = e.timeStamp;
    vel = 0;
    cur = 0;
  }
  function onMove(e: PointerEvent): void {
    if (!active || !sheet) return;
    const dy = e.clientY - startY;
    if (dy <= 0) {
      cur = 0;
      schedule();
      return;
    }
    const dt = e.timeStamp - lastT;
    if (dt > 0) vel = (e.clientY - lastY) / dt;
    lastY = e.clientY;
    lastT = e.timeStamp;
    cur = Math.min(h, dy);
    if (e.cancelable) e.preventDefault();
    schedule();
  }
  function onUp(): void {
    if (!active) return;
    active = false;
    const progress = h > 0 ? cur / h : 0;
    if (vel > FLICK || progress > CFG.dismissAt) {
      dismiss();
    } else {
      snapBack();
    }
  }

  document.addEventListener("pointerdown", onDown, true);
  document.addEventListener("pointermove", onMove, {
    passive: false,
    capture: true,
  });
  document.addEventListener("pointerup", onUp, true);
  document.addEventListener("pointercancel", onUp, true);
  w0[KEY] = () => {
    document.removeEventListener("pointerdown", onDown, true);
    document.removeEventListener("pointermove", onMove, true);
    document.removeEventListener("pointerup", onUp, true);
    document.removeEventListener("pointercancel", onUp, true);
  };
})();
