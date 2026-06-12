pub fn copy_text(text: &str) {
    let eval = dioxus::document::eval(
        r#"
        const text = await dioxus.recv();
        try {
            await navigator.clipboard.writeText(text);
        } catch (e) {
            var ta = document.createElement('textarea');
            ta.value = text;
            ta.style.position = 'fixed';
            ta.style.opacity = '0';
            document.body.appendChild(ta);
            ta.focus();
            ta.select();
            document.execCommand('copy');
            document.body.removeChild(ta);
        }
        "#,
    );
    let _ = eval.send(text);
}
