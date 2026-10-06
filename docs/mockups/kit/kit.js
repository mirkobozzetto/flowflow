// Tiny runtime for clickable mockups. Markup drives it:
//   <i data-i="IconPlus" data-s="24">     an icon from icons.js
//   data-go="screen-id"                    show that screen
//   data-open="layer-id" / data-close      open or close a menu or sheet
//   data-act="name"                        call Mock.act[name](el, event)
// A page adds its own behaviour in Mock.act and calls Mock.start().
(function () {
  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

  const Mock = {
    act: {},
    $,
    $$,

    icons(root = document) {
      $$("i[data-i]", root).forEach((el) => {
        const svg = window.ICONS[el.dataset.i];
        if (svg) el.innerHTML = svg.replace(/SIZE/g, el.dataset.s || "20");
      });
    },

    go(id) {
      $$(".mk-layer.is-on").forEach((l) => l.classList.remove("is-on"));
      $$(".mk-screen").forEach((s) => s.classList.toggle("is-on", s.id === id));
      $$(".mk-bar button").forEach((b) => b.setAttribute("aria-pressed", b.dataset.go === id));
    },

    open(id) {
      $("#" + id).classList.add("is-on");
    },

    close(el) {
      const layer = el.closest(".mk-layer") || $(".mk-layer.is-on:last-of-type");
      if (layer) layer.classList.remove("is-on");
    },

    toast(text) {
      const t = $(".mk-toast");
      t.textContent = text;
      t.classList.add("is-on");
      clearTimeout(t._timer);
      t._timer = setTimeout(() => t.classList.remove("is-on"), 1600);
    },

    // Builds markup from a template string and fills its icons.
    html(markup) {
      const box = document.createElement("div");
      box.innerHTML = markup.trim();
      const el = box.firstElementChild;
      Mock.icons(el);
      return el;
    },

    start(first) {
      Mock.icons();
      document.addEventListener("click", (e) => {
        const go = e.target.closest("[data-go]");
        if (go) return Mock.go(go.dataset.go);
        const open = e.target.closest("[data-open]");
        if (open) return Mock.open(open.dataset.open);
        const close = e.target.closest("[data-close]");
        if (close) return Mock.close(close);
        const act = e.target.closest("[data-act]");
        if (act && Mock.act[act.dataset.act]) Mock.act[act.dataset.act](act, e);
      });
      Mock.go(first);
    },
  };

  window.Mock = Mock;
})();
