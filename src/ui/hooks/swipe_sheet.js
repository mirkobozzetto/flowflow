// src/ui/hooks/swipe_sheet.ts
(function() {
  const CFG = {
    sheetId: "__SHEET__",
    backdropId: "__BACKDROP__",
    grabPx: __GRAB_PX__,
    dismissAt: __DISMISS_AT__
  };
  const FLICK = 0.5;
  const w0 = window;
  const KEY = "__sheet_" + CFG.sheetId;
  const prev = w0[KEY];
  if (prev)
    prev();
  let sheet = null;
  let backdrop = null;
  let h = 0;
  let active = false;
  let startY = 0;
  let lastY = 0;
  let lastT = 0;
  let vel = 0;
  let cur = 0;
  let raf = 0;
  function apply() {
    raf = 0;
    if (!sheet)
      return;
    sheet.style.transition = "none";
    sheet.style.translate = "0 " + cur + "px";
    if (backdrop) {
      const p = Math.max(0, Math.min(1, 1 - cur / h));
      backdrop.style.transition = "none";
      backdrop.style.opacity = p.toFixed(3);
    }
  }
  function schedule() {
    if (!raf)
      raf = requestAnimationFrame(apply);
  }
  function dismiss() {
    if (!sheet)
      return;
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
    setTimeout(() => dioxus.send("closed"), 210);
  }
  function snapBack() {
    if (!sheet)
      return;
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
      if (!sheet)
        return;
      sheet.style.transition = "";
      sheet.style.translate = "";
      if (backdrop) {
        backdrop.style.transition = "";
        backdrop.style.opacity = "";
      }
    }, 240);
  }
  function onDown(e) {
    if (e.pointerType === "mouse" || active)
      return;
    sheet = document.getElementById(CFG.sheetId);
    backdrop = document.getElementById(CFG.backdropId);
    if (!sheet)
      return;
    const rect = sheet.getBoundingClientRect();
    h = rect.height || 300;
    if (e.clientY > rect.top + CFG.grabPx)
      return;
    active = true;
    startY = lastY = e.clientY;
    lastT = e.timeStamp;
    vel = 0;
    cur = 0;
  }
  function onMove(e) {
    if (!active || !sheet)
      return;
    const dy = e.clientY - startY;
    if (dy <= 0) {
      cur = 0;
      schedule();
      return;
    }
    const dt = e.timeStamp - lastT;
    if (dt > 0)
      vel = (e.clientY - lastY) / dt;
    lastY = e.clientY;
    lastT = e.timeStamp;
    cur = Math.min(h, dy);
    if (e.cancelable)
      e.preventDefault();
    schedule();
  }
  function onUp() {
    if (!active)
      return;
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
    capture: true
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
