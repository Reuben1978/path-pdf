use std::path::PathBuf;

use tauri::State;

use crate::error::AppError;
use crate::pdf::save;
use crate::state::AppState;

/// Overwrites the file the document was originally opened from.
#[tauri::command]
pub fn save_document(id: u32, flatten: bool, state: State<AppState>) -> Result<(), AppError> {
    let pdfium = state.pdfium();
    state.with_document(id, |document, page_order, path| {
        save_to(pdfium, document, page_order, path, flatten)
    })
}

#[tauri::command]
pub fn save_document_as(
    id: u32,
    dest_path: PathBuf,
    flatten: bool,
    state: State<AppState>,
) -> Result<(), AppError> {
    let pdfium = state.pdfium();
    state.with_document(id, |document, page_order, _original_path| {
        save_to(pdfium, document, page_order, &dest_path, flatten)
    })
}

fn save_to(
    pdfium: &'static pdfium_render::prelude::Pdfium,
    document: &pdfium_render::prelude::PdfDocument,
    page_order: &[pdfium_render::prelude::PdfPageIndex],
    dest_path: &std::path::Path,
    flatten: bool,
) -> Result<(), AppError> {
    if save::is_identity_order(page_order) && !flatten {
        return save::save_atomic(document, dest_path);
    }

    let rebuilt = save::apply_page_order_and_flatten(pdfium, document, page_order, flatten)?;
    save::save_atomic(&rebuilt, dest_path)
}
