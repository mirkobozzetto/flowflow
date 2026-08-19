function initDashboard(): void {
  const items = document.querySelectorAll<HTMLButtonElement>(".b-item");
  const panes = document.querySelectorAll<HTMLElement>(".b-pane");

  for (const item of items) {
    item.addEventListener("click", () => {
      const key = item.dataset.pane;
      if (!key) return;
      for (const other of items) other.classList.toggle("on", other === item);
      for (const pane of panes) pane.classList.toggle("on", pane.dataset.pane === key);
    });
  }

  for (const toggle of document.querySelectorAll<HTMLElement>(".b-tog")) {
    toggle.addEventListener("click", () => toggle.classList.toggle("on"));
  }
}

if (document.readyState !== "loading") {
  initDashboard();
} else {
  addEventListener("DOMContentLoaded", initDashboard);
}

export {};
