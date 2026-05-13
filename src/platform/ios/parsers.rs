pub fn read_file_as_text(path: &std::path::Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "txt" | "md" | "csv" => std::fs::read_to_string(path)
            .map_err(|e| format!("Erreur lecture: {e}")),
        "pdf" => extract_pdf_text(path),
        "docx" => extract_docx_text(path),
        _ => Err(format!("Format non supporté: .{ext}")),
    }
}

fn extract_pdf_text(path: &std::path::Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("Erreur lecture PDF: {e}"))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Erreur extraction PDF: {e}"))
}

fn extract_docx_text(path: &std::path::Path) -> Result<String, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Erreur ouverture DOCX: {e}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Erreur archive DOCX: {e}"))?;
    let mut doc = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("Erreur document.xml: {e}"))?;
    let mut xml = String::new();
    use std::io::Read;
    doc.read_to_string(&mut xml)
        .map_err(|e| format!("Erreur lecture document.xml: {e}"))?;

    let mut reader = quick_xml::Reader::from_str(&xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut out = String::new();
    let mut in_text = false;
    let mut in_paragraph = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"w:t" {
                    in_text = true;
                } else if name_bytes == b"w:p" {
                    in_paragraph = true;
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"w:t" {
                    in_text = false;
                } else if name_bytes == b"w:p" {
                    if in_paragraph {
                        out.push('\n');
                    }
                    in_paragraph = false;
                }
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"w:br" || name_bytes == b"w:tab" {
                    out.push(if name_bytes == b"w:tab" { '\t' } else { '\n' });
                }
            }
            Ok(quick_xml::events::Event::Text(t)) => {
                if in_text {
                    let txt = t.unescape().map_err(|e| {
                        format!("Erreur décodage texte DOCX: {e}")
                    })?;
                    out.push_str(&txt);
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(format!("Erreur parsing DOCX: {e}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}
