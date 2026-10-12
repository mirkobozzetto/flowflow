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

#[test]
fn a_table_sits_in_its_own_scrolling_frame() {
    let html = md_to_html("| a | b |\n|---|---|\n| 1 | 2 |\n");
    assert!(
        html.starts_with(
            r#"<div class="table-frame"><div class="table-scroll"><table>"#
        ),
        "{html}"
    );
    assert!(html.trim_end().ends_with("</table></div></div>"), "{html}");
    assert!(!html.contains("style="), "{html}");
}

#[test]
fn table_alignment_reaches_the_cells() {
    let html = md_to_html("| name | cpu |\n|:--|--:|\n| web | 3 % |\n");
    assert!(
        html.contains(r#"<th style="text-align: right">cpu</th>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<td style="text-align: right">3 %</td>"#),
        "{html}"
    );
}

#[test]
fn a_bare_address_becomes_a_link_without_its_punctuation() {
    let html = md_to_html(
        "See https://example.com/a_b?x=1&y=2. And (https://ex.org/p).",
    );
    assert!(
        html.contains(
            r#"<a href="https://example.com/a_b?x=1&amp;y=2">https://example.com/a_b?x=1&amp;y=2</a>."#
        ),
        "{html}"
    );
    assert!(
        html.contains(r#"(<a href="https://ex.org/p">https://ex.org/p</a>)."#),
        "{html}"
    );
}

#[test]
fn addresses_in_code_and_links_are_left_alone() {
    let html = md_to_html(
        "`https://in.code`\n\n```\ncurl https://in.block\n```\n\n[x](https://linked.example) javascript:alert(1)",
    );
    assert!(html.contains("<code>https://in.code</code>"), "{html}");
    assert!(html.contains("curl https://in.block"), "{html}");
    assert_eq!(html.matches("<a ").count(), 1, "{html}");
    assert!(!html.contains(r#"href="javascript"#), "{html}");
}

#[test]
fn a_code_block_keeps_its_language() {
    let html = md_to_html("```bash\necho <hi>\n```\n");
    assert!(
        html.contains(r#"<pre><code class="language-bash">echo &lt;hi&gt;"#),
        "{html}"
    );
}
