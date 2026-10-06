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
      $$(".mk-menu-layer, .mk-alert-layer").forEach((l) => l.remove());
      $$("[data-open='true']").forEach((a) => a.removeAttribute("data-open"));
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

    // A native iOS menu anchored to `anchor`. Items:
    //   { icon, title, sub, danger, checked, run() }   an action
    //   { icon, title, sub, items: [...] }             a submenu, opened in place
    //   { head: "Title" } / "sep"                      a section title / a line
    menu(anchor, items, { above = true, right = false } = {}) {
      const phone = anchor.closest(".mk-phone");
      const layer = Mock.html(`<div class="mk-menu-layer"><div class="mk-glass-menu"></div></div>`);
      const box = layer.firstElementChild;
      const close = () => {
        layer.remove();
        anchor.removeAttribute("data-open");
      };
      const row = (it) => {
        if (it === "sep") return `<div class="mk-sep"></div>`;
        if (it.head) return `<div class="mk-head">${it.head}</div>`;
        const icon = it.checked ? "IconCheck" : it.icon;
        const lead = it.img
          ? `<img src="${it.img}" width="22" height="22" class="rounded-full object-cover">`
          : icon ? `<i data-i="${icon}" data-s="22"></i>` : "";
        const sub = it.sub ? `<span class="mk-sub">${it.sub}</span>` : "";
        const chev = it.items ? `<span class="mk-chev"><i data-i="IconCaretRight" data-s="16"></i></span>` : "";
        return `<button class="mk-row ${it.danger ? "is-danger" : ""}"><span class="mk-ic">${lead}</span><span class="mk-txt">${it.title}${sub}</span>${chev}</button>`;
      };
      const show = (list, parent) => {
        box.innerHTML = "";
        if (parent) {
          const back = Mock.html(`<button class="mk-row mk-back"><span class="mk-ic"><i data-i="IconArrowLeft" data-s="18"></i></span><span class="mk-txt">${parent.title}</span></button>`);
          back.onclick = () => show(parent.from, null);
          box.appendChild(back);
          box.appendChild(Mock.html(`<div class="mk-sep"></div>`));
        }
        list.forEach((it) => {
          const el = Mock.html(row(it));
          if (it.items) el.onclick = () => show(it.items, { title: it.title, from: list });
          else if (it.title) el.onclick = () => { close(); it.run && it.run(); };
          box.appendChild(el);
        });
        box.scrollTop = 0;
      };
      layer.addEventListener("click", (e) => { if (e.target === layer) close(); });
      phone.appendChild(layer);
      show(items, null);
      const p = phone.getBoundingClientRect();
      const a = anchor.getBoundingClientRect();
      const border = parseFloat(getComputedStyle(phone).borderTopWidth) || 0;
      if (right) box.style.right = `${p.right - a.right - border}px`;
      else box.style.left = `${a.left - p.left - border}px`;
      if (above) box.style.bottom = `${p.bottom - a.top - border + 8}px`;
      else box.style.top = `${a.bottom - p.top - border + 8}px`;
      box.style.transformOrigin = `${above ? "bottom" : "top"} ${right ? "right" : "left"}`;
      anchor.setAttribute("data-open", "true");
      return close;
    },

    // A native iOS alert with one text field; resolves with the text or null.
    prompt(phoneChild, title, message, value, ok = "OK") {
      const phone = phoneChild.closest(".mk-phone");
      const layer = Mock.html(`<div class="mk-alert-layer"><div class="mk-alert"><h4>${title}</h4><p>${message}</p>
        <input value="${value || ""}"><div class="mk-btns"><button>Annuler</button><button class="is-main">${ok}</button></div></div></div>`);
      phone.appendChild(layer);
      const input = layer.querySelector("input");
      input.focus();
      input.select();
      return new Promise((done) => {
        const [no, yes] = layer.querySelectorAll("button");
        no.onclick = () => { layer.remove(); done(null); };
        yes.onclick = () => { layer.remove(); done(input.value.trim() || null); };
      });
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
