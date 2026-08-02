use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::pdf::pages;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSummary {
    pub logical_index: u32,
    pub rotation_degrees: f32,
}

#[tauri::command]
pub fn list_pages(id: u32, state: State<AppState>) -> Result<Vec<PageSummary>, AppError> {
    state.with_document(id, |document, page_order, _path| {
        page_order
            .iter()
            .enumerate()
            .map(|(logical_index, &physical)| {
                let rotation_degrees = document
                    .pages()
                    .get(physical)
                    .map_err(|e| AppError::RenderFailed(e.to_string()))?
                    .rotation()
                    .unwrap_or(pdfium_render::prelude::PdfPageRenderRotation::None)
                    .as_degrees();

                Ok(PageSummary { logical_index: logical_index as u32, rotation_degrees })
            })
            .collect()
    })
}

#[tauri::command]
pub fn reorder_pages(id: u32, new_order: Vec<u32>, state: State<AppState>) -> Result<(), AppError> {
    state.with_document(id, |_document, page_order, _path| pages::reorder(page_order, &new_order))
}

#[tauri::command]
pub fn delete_pages(id: u32, logical_indices: Vec<u32>, state: State<AppState>) -> Result<(), AppError> {
    state.with_document(id, |_document, page_order, _path| pages::delete(page_order, &logical_indices))
}

#[tauri::command]
pub fn rotate_page(
    id: u32,
    logical_index: u32,
    clockwise: bool,
    state: State<AppState>,
) -> Result<(), AppError> {
    state.with_document(id, |document, page_order, _path| {
        pages::rotate(document, page_order, logical_index, clockwise)
    })
}

#[tauri::command]
pub fn extract_pages(
    id: u32,
    logical_indices: Vec<u32>,
    dest_path: PathBuf,
    state: State<AppState>,
) -> Result<(), AppError> {
    let pdfium = state.pdfium();
    state.with_document(id, |document, page_order, _path| {
        pages::extract(pdfium, document, page_order, &logical_indices, &dest_path)
    })
}
