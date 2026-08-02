use pdfium_render::prelude::{PdfDocument, PdfRenderConfig};

use crate::error::AppError;

pub struct RenderedPage {
    pub width: u32,
    pub height: u32,
    /// Page dimensions in PDF points (bottom-left origin) -- lets the
    /// frontend convert a click's canvas-pixel position into PDF page space
    /// without a second round trip. See ipc.ts, the one place that
    /// conversion is allowed to happen (CLAUDE.md's coordinate-system rule).
    pub page_width_points: f32,
    pub page_height_points: f32,
    pub rgba: Vec<u8>,
}

/// Rasterizes one page to an RGBA8 buffer, scaled to `target_width` with the
/// page's own aspect ratio preserved. Caller is responsible for picking a
/// sensible `target_width` (e.g. the visible canvas width) -- this never
/// renders at native document resolution up front.
pub fn render_page(
    document: &PdfDocument,
    page_index: u32,
    target_width: u32,
) -> Result<RenderedPage, AppError> {
    let page_count = document.pages().len() as u32;

    let page = document
        .pages()
        .iter()
        .nth(page_index as usize)
        .ok_or(AppError::PageOutOfRange { index: page_index, page_count })?;

    let page_width_points = page.width().value;
    let page_height_points = page.height().value;

    let config = PdfRenderConfig::new().set_target_width(target_width as i32);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| AppError::RenderFailed(e.to_string()))?;

    let image = bitmap.as_image().to_rgba8();

    let width = image.width();
    let height = image.height();
    let rgba = image.into_raw();

    Ok(RenderedPage { width, height, page_width_points, page_height_points, rgba })
}
