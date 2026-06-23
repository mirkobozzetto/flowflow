use objc2_foundation::{
    MainThreadMarker, NSDictionary, NSNumber, NSString, NSURL,
};
use objc2_pdf_kit::{PDFDocument, PDFDocumentSaveTextFromOCROption};

pub fn extract(path: &std::path::Path) -> Result<String, String> {
    let mtm =
        MainThreadMarker::new().ok_or("PDF extraction requires main thread")?;
    unsafe {
        let ns_path = NSString::from_str(&path.to_string_lossy());
        let url = NSURL::fileURLWithPath(&ns_path);
        let Some(doc) =
            PDFDocument::initWithURL(mtm.alloc::<PDFDocument>(), &url)
        else {
            return Err("Impossible d'ouvrir le PDF".to_string());
        };
        if let Some(text) = doc.string() {
            let s = text.to_string();
            if !s.trim().is_empty() {
                return Ok(s);
            }
        }
        extract_with_ocr(mtm, &doc)
    }
}

unsafe fn extract_with_ocr(
    mtm: MainThreadMarker,
    doc: &PDFDocument,
) -> Result<String, String> {
    let key = PDFDocumentSaveTextFromOCROption;
    let val = NSNumber::new_bool(true);
    let opts: objc2::rc::Retained<NSDictionary> = objc2::msg_send![
        objc2::runtime::AnyClass::get(c"NSDictionary").unwrap(),
        dictionaryWithObject: &*val,
        forKey: key,
    ];
    let Some(data) = doc.dataRepresentationWithOptions(&opts) else {
        return Err("OCR : impossible de traiter le PDF".to_string());
    };
    let Some(ocr_doc) =
        PDFDocument::initWithData(mtm.alloc::<PDFDocument>(), &data)
    else {
        return Err("OCR : impossible de relire le PDF".to_string());
    };
    match ocr_doc.string() {
        Some(text) => {
            let s = text.to_string();
            if s.trim().is_empty() {
                Err("Aucun texte extrait (PDF image sans texte lisible)"
                    .to_string())
            } else {
                Ok(s)
            }
        }
        None => Err("Aucun texte extrait du PDF".to_string()),
    }
}
