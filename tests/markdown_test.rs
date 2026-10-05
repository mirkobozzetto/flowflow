use flowflow::ui::md_to_html;

#[test]
fn untrusted_markdown_cannot_inject_html_or_script_links() {
    let html = md_to_html(
        "<script>alert(1)</script>\n\nhi <img src=x onerror=alert(2)> \
         [a](javascript:alert(3)) [b](JaVaScRiPt:x) [c](https://ok.example) [d](/rel)",
    );
    assert!(!html.contains("<script"), "{html}");
    assert!(!html.contains("<img"), "{html}");
    assert!(!html.to_lowercase().contains("javascript:"), "{html}");
    assert!(html.contains(r#"href="https://ok.example""#), "{html}");
    assert!(html.contains(r#"href="/rel""#), "{html}");
}
