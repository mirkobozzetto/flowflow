// src/ui/chat/tools_popover.ts
(() => {
  const menu = document.querySelector(".tools-popover");
  const trigger = menu?.parentElement?.querySelector(":scope > button");
  const root = menu?.querySelector(".tools-root");
  const sub = menu?.querySelector(".tools-sub");
  if (!menu || !trigger || !root || !sub)
    return;
  let returnTo = root.querySelector("button");
  const focus = (element) => element?.focus({ preventScroll: true });
  const fit = () => {
    const top = window.visualViewport?.offsetTop ?? 0;
    menu.style.setProperty("--tools-space", `${Math.max(0, trigger.getBoundingClientRect().top - top - 16)}px`);
  };
  const remember = (event) => {
    const row = event.target.closest("[data-pane]");
    if (row)
      returnTo = row;
  };
  root.addEventListener("click", remember);
  const panes = new MutationObserver(() => {
    focus(root.getAttribute("data-away") === "true" ? sub.querySelector("button") : returnTo);
  });
  panes.observe(root, { attributes: true, attributeFilter: ["data-away"] });
  window.visualViewport?.addEventListener("resize", fit);
  window.visualViewport?.addEventListener("scroll", fit);
  window.addEventListener("resize", fit);
  fit();
  focus(returnTo);
  const removal = new MutationObserver(() => {
    if (menu.isConnected)
      return;
    panes.disconnect();
    removal.disconnect();
    root.removeEventListener("click", remember);
    window.visualViewport?.removeEventListener("resize", fit);
    window.visualViewport?.removeEventListener("scroll", fit);
    window.removeEventListener("resize", fit);
    if (trigger.isConnected && document.activeElement === document.body)
      focus(trigger);
  });
  removal.observe(document.body, { childList: true, subtree: true });
})();
