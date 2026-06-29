// src/ui/hooks/swipe.ts
(function() {
  const CFG = {
    panelId: "__PANEL__",
    backdropId: "__BACKDROP__",
    side: "__EDGE__",
    edgePx: __EDGE_PX__,
    openAt: __OPEN_AT__,
    closeAt: __CLOSE_AT__
  };
  const FLICK = 0.4;
  const sign = CFG.side === "left" ? -1 : 1;
  const w0 = window;
  const guard = "__swipe_" + CFG.panelId;
  if (w0[guard])
    return;
  w0[guard] = true;
  let panel = null;
  let backdrop = null;
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
  function els() {
    panel = document.getElementById(CFG.panelId);
    backdrop = document.getElementById(CFG.backdropId);
    return !!panel && !!backdrop;
  }
  function apply() {
    raf = 0;
    if (!panel)
      return;
    panel.style.transition = "none";
    panel.style.translate = cur + "px";
    if (backdrop) {
      const p = Math.max(0, Math.min(1, 1 - Math.abs(cur) / w));
      backdrop.style.transition = "none";
      backdrop.style.opacity = p.toFixed(3);
    }
  }
  function schedule() {
    if (!raf)
      raf = requestAnimationFrame(apply);
  }
  function settle(open) {
    if (!panel)
      return;
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
    const want = open ? "1" : "0";
    let tries = 0;
    const tryClear = () => {
      if (!panel)
        return;
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
  function onDown(e) {
    if (e.pointerType === "mouse" || active || !els() || !panel)
      return;
    w = panel.offsetWidth || 300;
    const open = panel.dataset.open === "1";
    if (!open) {
      const atEdge = CFG.side === "left" ? e.clientX <= CFG.edgePx : e.clientX >= window.innerWidth - CFG.edgePx;
      if (!atEdge)
        return;
      active = true;
      opening = true;
      engaged = false;
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
  function onMove(e) {
    if (!active || !panel)
      return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    if (!engaged) {
      let toward;
      if (opening)
        toward = CFG.side === "left" ? dx > 8 : dx < -8;
      else
        toward = CFG.side === "left" ? dx < -8 : dx > 8;
      if (toward && Math.abs(dx) > Math.abs(dy)) {
        engaged = true;
      } else if (Math.abs(dy) > 8) {
        active = false;
        return;
      } else {
        return;
      }
    }
    const dt = e.timeStamp - lastT;
    if (dt > 0)
      vel = (e.clientX - lastX) / dt;
    lastX = e.clientX;
    lastT = e.timeStamp;
    const lo = Math.min(0, sign * w);
    const hi = Math.max(0, sign * w);
    cur = opening ? Math.max(lo, Math.min(hi, sign * w + dx)) : Math.max(lo, Math.min(hi, dx));
    if (e.cancelable)
      e.preventDefault();
    schedule();
  }
  function onUp() {
    if (!active || !panel)
      return;
    const wasEngaged = engaged;
    active = false;
    engaged = false;
    if (!wasEngaged)
      return;
    const swallow = (ev) => {
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
    capture: true
  });
  document.addEventListener("pointerup", onUp, true);
  document.addEventListener("pointercancel", onUp, true);
})();
