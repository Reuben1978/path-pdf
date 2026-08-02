use std::fs;
use std::path::Path;

use pdfium_render::prelude::{Pdfium, PdfDocument, PdfPageIndex};

use crate::error::AppError;

/// True if `page_order` is still the identity mapping (no reorder/delete
/// pending) -- lets `save` skip an unnecessary document rebuild.
pub fn is_identity_order(page_order: &[PdfPageIndex]) -> bool {
    page_order.iter().enumerate().all(|(i, &physical)| i as PdfPageIndex == physical)
}

/// Applies the logical page order (see state.rs) to a real document by
/// building a new one and copying pages in over in that order, and
/// optionally flattens every page's annotations into its content stream.
/// Building an entirely new document is the only way pdfium-render exposes
/// to actually reorder/delete pages -- pdf::pages::reorder/delete stay
/// purely in-memory bookkeeping until this point. Annotations added via
/// pdf::annots are already part of each page's own PDF structure, so they
/// carry over automatically when pages are copied.
pub fn apply_page_order_and_flatten(
    pdfium: &'static Pdfium,
    document: &PdfDocument,
    page_order: &[PdfPageIndex],
    flatten: bool,
) -> Result<PdfDocument<'static>, AppError> {
    let mut new_document = pdfium.create_new_pdf().map_err(|e| AppError::SaveFailed(e.to_string()))?;

    let range = page_order
        .iter()
        .map(|physical| (physical + 1).to_string())
        .collect::<Vec<_>>()
        .join(",");

    new_document
        .pages_mut()
        .copy_pages_from_document(document, &range, 0)
        .map_err(|e| AppError::SaveFailed(e.to_string()))?;

    if flatten {
        flatten_all_pages(&new_document)?;
    }

    Ok(new_document)
}

fn flatten_all_pages(document: &PdfDocument) -> Result<(), AppError> {
    for index in document.pages().as_range() {
        let mut page = document.pages().get(index).map_err(|e| AppError::SaveFailed(e.to_string()))?;
        page.flatten().map_err(|e| AppError::SaveFailed(e.to_string()))?;
    }
    Ok(())
}

/// Writes atomically: serializes to bytes, writes a temp file in the same
/// directory as the destination, then renames over the destination -- so a
/// crash or power loss mid-write can never leave the user's file corrupted
/// or partially written. Note: pdfium-render 0.8's `save_to_*` functions
/// always perform a full rewrite (`FPDF_SaveAsCopy`); the crate doesn't
/// currently expose PDFium's `FPDF_INCREMENTAL` flag, so CLAUDE.md's
/// "incremental save when only annotations changed" isn't achievable with
/// this crate version -- every save is a full rewrite regardless.
pub fn save_atomic(document: &PdfDocument, dest_path: &Path) -> Result<(), AppError> {
    let dir = dest_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::SaveFailed("destination has no parent directory".into()))?;
    let file_name = dest_path
        .file_name()
        .ok_or_else(|| AppError::SaveFailed("destination has no file name".into()))?;
    let tmp_path = dir.join(format!(".{}.pdfapp-tmp", file_name.to_string_lossy()));

    let bytes = document.save_to_bytes().map_err(|e| AppError::SaveFailed(e.to_string()))?;
    fs::write(&tmp_path, &bytes).map_err(|e| AppError::SaveFailed(e.to_string()))?;
    fs::rename(&tmp_path, dest_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        AppError::SaveFailed(e.to_string())
    })
}
