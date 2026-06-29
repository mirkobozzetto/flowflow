use flowflow::ui::hooks::swipe::build_script;

const PLACEHOLDERS: [&str; 6] = [
    "__PANEL__",
    "__BACKDROP__",
    "__EDGE__",
    "__EDGE_PX__",
    "__OPEN_AT__",
    "__CLOSE_AT__",
];

#[test]
fn build_script_fills_every_placeholder() {
    let s = build_script("sb-panel", "sb-backdrop", "left", 30.0, 0.15, 0.4);
    for ph in PLACEHOLDERS {
        assert!(
            !s.contains(ph),
            "leftover placeholder {ph} in generated script"
        );
    }
    assert!(s.contains("\"sb-panel\""));
    assert!(s.contains("\"sb-backdrop\""));
    assert!(s.contains("\"left\""));
}

#[test]
fn build_script_numeric_config_is_not_nan() {
    // The embedded controller reads edgePx/openAt/closeAt as numbers. A bun
    // constant-fold of a string-coercion placeholder (`+"__EDGE_PX__"`) produced
    // NaN and silently killed the open swipe (clientX <= NaN is always false).
    let s = build_script("p", "b", "left", 30.0, 0.15, 0.4);
    assert!(
        !s.contains("NaN"),
        "NaN in generated script: numeric placeholder was folded"
    );
    assert!(s.contains("edgePx: 30"));
    assert!(s.contains("openAt: 0.15"));
    assert!(s.contains("closeAt: 0.4"));
}

#[test]
fn build_script_supports_right_edge() {
    let s = build_script("p", "b", "right", 24.0, 0.2, 0.5);
    assert!(s.contains("\"right\""));
    for ph in PLACEHOLDERS {
        assert!(!s.contains(ph));
    }
}
