use std::path::Path;

use pdfium_render::prelude::{Pdfium, PdfDocument, PdfPageIndex};

use crate::error::AppError;

/// Every PDF is untrusted input: malformed, truncated, encrypted, or hostile
/// files must surface as an `AppError`, never a panic.
pub fn open(pdfium: &'static Pdfium, path: &Path) -> Result<PdfDocument<'static>, AppError> {
    pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| AppError::OpenFailed(e.to_string()))
}

pub fn page_count(document: &PdfDocument) -> u32 {
    document.pages().len() as u32
}

/// A page's dimensions in PDF points, without rendering it -- cheap enough to
/// call for every page up front (used to lay out the continuous-scroll
/// viewer before each page's bitmap has actually loaded).
pub fn page_size(document: &PdfDocument, physical_index: PdfPageIndex) -> Result<(f32, f32), AppError> {
    let size = document
        .pages()
        .page_size(physical_index)
        .map_err(|e| AppError::RenderFailed(e.to_string()))?;
    Ok((size.width().value, size.height().value))
}

/// Creates a new, empty in-memory PDF document -- used by `pdf::pages::extract`
/// as the destination for copied pages.
pub fn create_blank(pdfium: &'static Pdfium) -> Result<PdfDocument<'static>, AppError> {
    pdfium
        .create_new_pdf()
        .map_err(|e| AppError::ExtractFailed(e.to_string()))
}
