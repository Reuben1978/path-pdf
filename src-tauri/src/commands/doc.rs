use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, State};

use crate::commands::recents;
use crate::error::AppError;
use crate::pdf::{doc, render};
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInfo {
    pub id: u32,
    pub page_count: u32,
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSize {
    pub width_points: f32,
    pub height_points: f32,
}

/// Called once by the frontend on startup to check whether the OS launched
/// this process with a file to open (see lib.rs's `initial_file`). Returns
/// `None` on every subsequent call, or on a normal (no-file) launch.
#[tauri::command]
pub fn take_launch_file(state: State<AppState>) -> Option<String> {
    state.take_initial_file().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn open_document(path: PathBuf, app: AppHandle, state: State<AppState>) -> Result<DocumentInfo, AppError> {
    let document = doc::open(state.pdfium(), &path)?;
    let page_count = doc::page_count(&document);
    let path_string = path.to_string_lossy().into_owned();

    // Best-effort: a document that fails to open never reaches here, so this
    // only records genuinely successful opens. Failing to *record* it (e.g.
    // a disk error writing recents.json) shouldn't block the open itself --
    // it's a convenience feature, not core functionality.
    let _ = recents::record_opened(&app, &path);

    let id = state.insert(document, path);

    Ok(DocumentInfo { id, page_count, path: path_string })
}

/// Returns `tauri::ipc::Response` rather than a JSON-serialized struct to avoid
/// base64-encoding the bitmap (see CLAUDE.md's rendering-path note). Wire
/// format: u32 width, u32 height, f32 page_width_points, f32 page_height_points
/// (all little-endian), then raw RGBA8 bytes. Documented in src/lib/ipc.ts,
/// which decodes it on the frontend.
/// `page_index` is a *logical* index (post reorder/delete) -- see state.rs.
#[tauri::command]
pub fn render_page(
    id: u32,
    page_index: u32,
    target_width: u32,
    state: State<AppState>,
) -> Result<Response, AppError> {
    let rendered = state.with_document(id, |document, page_order, _path| {
        let page_count = page_order.len() as u32;
        let physical = *page_order
            .get(page_index as usize)
            .ok_or(AppError::PageOutOfRange { index: page_index, page_count })?;
        render::render_page(document, physical.into(), target_width)
    })?;

    let mut payload = Vec::with_capacity(16 + rendered.rgba.len());
    payload.extend_from_slice(&rendered.width.to_le_bytes());
    payload.extend_from_slice(&rendered.height.to_le_bytes());
    payload.extend_from_slice(&rendered.page_width_points.to_le_bytes());
    payload.extend_from_slice(&rendered.page_height_points.to_le_bytes());
    payload.extend_from_slice(&rendered.rgba);

    Ok(Response::new(payload))
}

/// Cheap (no rendering) dimensions for every page, in logical order -- used
/// to lay out the continuous-scroll viewer before each page's bitmap has
/// actually loaded.
#[tauri::command]
pub fn get_page_sizes(id: u32, state: State<AppState>) -> Result<Vec<PageSize>, AppError> {
    state.with_document(id, |document, page_order, _path| {
        page_order
            .iter()
            .map(|&physical| {
                let (width_points, height_points) = doc::page_size(document, physical)?;
                Ok(PageSize { width_points, height_points })
            })
            .collect()
    })
}
