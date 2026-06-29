// src/ui/hooks/swipe_right_nav.ts
(function() {
  const w0 = window;
  if (w0.__swipeChat)
    return;
  w0.__swipeChat = true;
  let active = false;
  let engaged = false;
  let startX = 0;
  let startY = 0;
  function onDown(e) {
    if (e.pointerType === "mouse")
      return;
    if (e.clientX < window.innerWidth - __EDGE_PX__)
      return;
    active = true;
    engaged = false;
    startX = e.clientX;
    startY = e.clientY;
  }
  function onMove(e) {
    if (!active)
      return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    if (!engaged) {
      if (dx < -10 && Math.abs(dx) > Math.abs(dy))
        engaged = true;
      else if (Math.abs(dy) > 10) {
        active = false;
        return;
      } else
        return;
    }
  }
  function onUp(e) {
    if (!active)
      return;
    const dx = e.clientX - startX;
    const wasEngaged = engaged;
    active = false;
    engaged = false;
    if (wasEngaged && dx < -__THRESHOLD__)
      dioxus.send("chat");
  }
  document.addEventListener("pointerdown", onDown, true);
  document.addEventListener("pointermove", onMove, {
    passive: true,
    capture: true
  });
  document.addEventListener("pointerup", onUp, true);
  document.addEventListener("pointercancel", () => {
    active = false;
    engaged = false;
  }, true);
})();
