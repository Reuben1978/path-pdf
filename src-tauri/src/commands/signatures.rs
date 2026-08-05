use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::pdf::annots;
use crate::state::AppState;

fn signatures_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?
        .join("signatures");
    fs::create_dir_all(&dir).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
    Ok(dir)
}

/// Timestamp-based filename, unique enough for this single-user, low-frequency
/// use case without pulling in a UUID dependency just for this.
fn unique_filename(extension: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("sig_{nanos}.{extension}")
}

fn store_png(dir: &Path, bytes: &[u8]) -> Result<String, AppError> {
    // Validate it's actually a decodable image before writing it to the
    // library -- every file on disk here is later fed straight into PDFium.
    annots::signature_dimensions(bytes)?;

    // Imported signatures may be PNG or JPEG (see Cargo.toml's image crate
    // features) -- name the file to match what's actually in it, since the
    // frontend picks a Blob MIME type off this extension when rendering
    // thumbnails.
    let extension = match image::guess_format(bytes) {
        Ok(image::ImageFormat::Jpeg) => "jpg",
        _ => "png",
    };
    let filename = unique_filename(extension);
    fs::write(dir.join(&filename), bytes).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
    Ok(filename)
}

#[tauri::command]
pub fn import_signature(source_path: PathBuf, app: AppHandle) -> Result<String, AppError> {
    let bytes = fs::read(&source_path).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
    store_png(&signatures_dir(&app)?, &bytes)
}

/// Used by the "draw a signature" flow: the frontend exports the drawing
/// canvas to a PNG blob and sends the raw bytes here.
#[tauri::command]
pub fn save_drawn_signature(png_bytes: Vec<u8>, app: AppHandle) -> Result<String, AppError> {
    store_png(&signatures_dir(&app)?, &png_bytes)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureInfo {
    pub filename: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn list_signatures(app: AppHandle) -> Result<Vec<SignatureInfo>, AppError> {
    let dir = signatures_dir(&app)?;
    let mut entries = Vec::new();

    let read_dir = fs::read_dir(&dir).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
    for entry in read_dir {
        let entry = entry.map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
        let filename = entry.file_name().to_string_lossy().into_owned();
        let bytes = fs::read(entry.path()).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
        let (width, height) = annots::signature_dimensions(&bytes)?;
        entries.push(SignatureInfo { filename, width, height });
    }

    entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(entries)
}

#[tauri::command]
pub fn delete_signature(filename: String, app: AppHandle) -> Result<(), AppError> {
    let path = signatures_dir(&app)?.join(&filename);
    fs::remove_file(path).map_err(|e| AppError::AnnotationFailed(e.to_string()))
}

#[tauri::command]
pub fn get_signature_bytes(filename: String, app: AppHandle) -> Result<Response, AppError> {
    let path = signatures_dir(&app)?.join(&filename);
    let bytes = fs::read(path).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;
    Ok(Response::new(bytes))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn place_signature(
    id: u32,
    page_index: u32,
    filename: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), AppError> {
    let path = signatures_dir(&app)?.join(&filename);
    let bytes = fs::read(path).map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    state.with_document(id, |document, page_order, _path| {
        let page_count = page_order.len() as u32;
        let physical = *page_order
            .get(page_index as usize)
            .ok_or(AppError::PageOutOfRange { index: page_index, page_count })?;
        annots::add_signature_annotation(document, physical, x, y, width, height, &bytes)
    })
}
