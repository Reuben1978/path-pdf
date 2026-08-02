use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::pdf::annots;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnnotationInfo {
    pub annotation_index: u32,
    pub contents: String,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_text_annotation(
    id: u32,
    page_index: u32,
    x: f32,
    y: f32,
    text: String,
    font_size: f32,
    font_name: String,
    state: State<AppState>,
) -> Result<(), AppError> {
    state.with_document(id, |document, page_order, _path| {
        let page_count = page_order.len() as u32;
        let physical = *page_order
            .get(page_index as usize)
            .ok_or(AppError::PageOutOfRange { index: page_index, page_count })?;
        annots::add_text_annotation(document, physical, x, y, &text, font_size, &font_name)
    })
}

#[tauri::command]
pub fn list_available_fonts() -> Vec<&'static str> {
    annots::AVAILABLE_FONTS.to_vec()
}

#[tauri::command]
pub fn list_text_annotations(
    id: u32,
    page_index: u32,
    state: State<AppState>,
) -> Result<Vec<TextAnnotationInfo>, AppError> {
    state.with_document(id, |document, page_order, _path| {
        let page_count = page_order.len() as u32;
        let physical = *page_order
            .get(page_index as usize)
            .ok_or(AppError::PageOutOfRange { index: page_index, page_count })?;
        Ok(annots::list_text_annotations(document, physical)?
            .into_iter()
            .map(|s| TextAnnotationInfo { annotation_index: s.annotation_index, contents: s.contents })
            .collect())
    })
}

#[tauri::command]
pub fn delete_text_annotation(
    id: u32,
    page_index: u32,
    annotation_index: u32,
    state: State<AppState>,
) -> Result<(), AppError> {
    state.with_document(id, |document, page_order, _path| {
        let page_count = page_order.len() as u32;
        let physical = *page_order
            .get(page_index as usize)
            .ok_or(AppError::PageOutOfRange { index: page_index, page_count })?;
        annots::delete_text_annotation(document, physical, annotation_index)
    })
}
