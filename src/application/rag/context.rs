use crate::infrastructure::vectordb::SearchResult;

pub fn build_context(results: &[SearchResult]) -> String {
    let mut ctx = String::from("--- User notes ---\n\n");
    for (i, r) in results.iter().enumerate() {
        ctx.push_str(&format!(
            "[Source {}] Note: \"{}\"\n{}\n\n",
            i + 1,
            r.title,
            r.chunk_text
        ));
    }
    ctx
}
