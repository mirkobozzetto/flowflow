// Right-edge swipe to open a new chat (webview only). A leftward swipe that
// starts within EDGE_PX of the right edge opens a chat, from any page. Passive
// (no preventDefault: a horizontal swipe steals no vertical scroll), so it never
// blocks taps, buttons, or the main thread, and renders no element.
//
// Config placeholders are filled by the Rust hook (use_swipe_chat) pre-eval.
// Ambient globals (dioxus, __EDGE_PX__, __THRESHOLD__) live in globals.d.ts.

(function () {
  const w0 = window as unknown as Record<string, boolean>;
  if (w0.__swipeChat) return;
  w0.__swipeChat = true;

  let active = false;
  let engaged = false;
  let startX = 0;
  let startY = 0;

  function onDown(e: PointerEvent): void {
    if (e.pointerType === "mouse") return;
    // Right edge only. Re-init each pointerdown (a missed up/cancel during a nav
    // re-render must not kill later gestures).
    if (e.clientX < window.innerWidth - __EDGE_PX__) return;
    active = true;
    engaged = false;
    startX = e.clientX;
    startY = e.clientY;
  }
  function onMove(e: PointerEvent): void {
    if (!active) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    if (!engaged) {
      if (dx < -10 && Math.abs(dx) > Math.abs(dy)) engaged = true;
      else if (Math.abs(dy) > 10) {
        active = false;
        return;
      } else return;
    }
  }
  function onUp(e: PointerEvent): void {
    if (!active) return;
    const dx = e.clientX - startX;
    const wasEngaged = engaged;
    active = false;
    engaged = false;
    if (wasEngaged && dx < -__THRESHOLD__) dioxus.send("chat");
  }

  document.addEventListener("pointerdown", onDown, true);
  document.addEventListener("pointermove", onMove, {
    passive: true,
    capture: true,
  });
  document.addEventListener("pointerup", onUp, true);
  document.addEventListener(
    "pointercancel",
    () => {
      active = false;
      engaged = false;
    },
    true,
  );
})();
