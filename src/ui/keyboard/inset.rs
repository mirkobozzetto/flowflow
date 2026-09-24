use dioxus::prelude::*;

// Lifts `.keyboard-aware` bars above the keyboard. On iOS the height comes
// from UIKit (`observe_keyboard` calls `window.__ffKeyboard`), exact for every
// keyboard including Gboard; `visualViewport` is only the fallback until the
// first native report. WebKit pans the page to reveal a focused field sitting
// under the keys; for a bar that we lift ourselves that pan is pure offset, so
// the page is pinned back to the top while the keyboard settles.
pub fn use_keyboard_inset() {
    use_effect(|| {
        dioxus::document::eval(
            r#"
            (function() {
                var cachedKeyboardH = 0;
                var native = false;

                function applyOffset(offset) {
                    document.documentElement.style.setProperty('--keyboard-inset', offset + 'px');
                    document.documentElement.dataset.keyboard = offset > 50 ? '1' : '0';
                    var els = document.querySelectorAll('.keyboard-aware');
                    for (var i = 0; i < els.length; i++) {
                        els[i].style.bottom = offset + 'px';
                    }
                }

                function inBar(el) {
                    return !!(el && el.closest && el.closest('.keyboard-aware'));
                }

                function pinTop() {
                    if (!inBar(document.activeElement)) return;
                    var start = performance.now();
                    var step = function(now) {
                        if (window.scrollY !== 0) window.scrollTo(0, 0);
                        if (now - start < 700) requestAnimationFrame(step);
                    };
                    requestAnimationFrame(step);
                }

                window.__ffKeyboard = function(h) {
                    native = true;
                    if (h > 50) cachedKeyboardH = h;
                    applyOffset(h);
                    if (h > 0) pinTop();
                };

                function measureKeyboard() {
                    if (!window.visualViewport) return 0;
                    var vv = window.visualViewport;
                    return Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
                }

                if (window.visualViewport) {
                    var handler = function() {
                        if (native) return;
                        var offset = measureKeyboard();
                        if (offset > 50) cachedKeyboardH = offset;
                        applyOffset(offset);
                    };
                    window.visualViewport.addEventListener('resize', handler);
                    window.visualViewport.addEventListener('scroll', handler);
                }

                document.addEventListener('focusin', function(e) {
                    if (e.target.tagName !== 'INPUT' && e.target.tagName !== 'TEXTAREA') return;
                    if (cachedKeyboardH > 0) applyOffset(cachedKeyboardH);
                    if (inBar(e.target)) {
                        pinTop();
                        return;
                    }
                    setTimeout(function() {
                        e.target.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
                    }, 400);
                });

                document.addEventListener('focusout', function() {
                    // Natively the hide notification resets the offset; a
                    // focus move between two fields keeps the keyboard up.
                    if (!native) applyOffset(0);
                });
            })();
            "#,
        );
    });
}
