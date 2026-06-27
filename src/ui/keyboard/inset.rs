use dioxus::prelude::*;

pub fn use_keyboard_inset() {
    use_effect(|| {
        dioxus::document::eval(
            r#"
            (function() {
                var cachedKeyboardH = 0;

                function applyOffset(offset) {
                    document.documentElement.style.setProperty('--keyboard-inset', offset + 'px');
                    var els = document.querySelectorAll('.keyboard-aware');
                    for (var i = 0; i < els.length; i++) {
                        els[i].style.bottom = offset + 'px';
                    }
                }

                function measureKeyboard() {
                    if (!window.visualViewport) return 0;
                    var vv = window.visualViewport;
                    return Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
                }

                if (window.visualViewport) {
                    var handler = function() {
                        var offset = measureKeyboard();
                        if (offset > 50) cachedKeyboardH = offset;
                        applyOffset(offset);
                    };
                    window.visualViewport.addEventListener('resize', handler);
                    window.visualViewport.addEventListener('scroll', handler);
                }

                document.addEventListener('focusin', function(e) {
                    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
                        if (cachedKeyboardH > 0) {
                            applyOffset(cachedKeyboardH);
                        }
                        setTimeout(function() {
                            var h = measureKeyboard();
                            if (h > 50) {
                                cachedKeyboardH = h;
                                applyOffset(h);
                            }
                            e.target.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
                        }, 400);
                    }
                });

                document.addEventListener('focusout', function() {
                    applyOffset(0);
                    requestAnimationFrame(function() {
                        window.scrollTo(0, window.scrollY);
                    });
                });
            })();
            "#,
        );
    });
}
