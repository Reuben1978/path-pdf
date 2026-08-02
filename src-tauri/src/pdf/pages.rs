use std::path::Path;

use pdfium_render::prelude::{Pdfium, PdfDocument, PdfPageIndex, PdfPageRenderRotation};

use crate::error::AppError;

/// Permutes `page_order` (the logical-to-physical page mapping) according to
/// `new_order`, a permutation of logical indices `0..page_order.len()`. Never
/// touches the underlying document -- matches CLAUDE.md's "reordering mutates
/// an in-memory page order that is applied on save, not on each drag."
pub fn reorder(page_order: &mut [PdfPageIndex], new_order: &[u32]) -> Result<(), AppError> {
    let page_count = page_order.len() as u32;

    if new_order.len() != page_order.len() {
        return Err(AppError::InvalidPageOrder);
    }

    let mut reordered = Vec::with_capacity(page_order.len());
    for &logical in new_order {
        let physical = *page_order
            .get(logical as usize)
            .ok_or(AppError::PageOutOfRange { index: logical, page_count })?;
        reordered.push(physical);
    }

    page_order.copy_from_slice(&reordered);
    Ok(())
}

/// Removes the given logical indices from `page_order`. Same in-memory-only
/// semantics as `reorder`.
pub fn delete(page_order: &mut Vec<PdfPageIndex>, logical_indices: &[u32]) -> Result<(), AppError> {
    let page_count = page_order.len() as u32;

    let mut indices: Vec<usize> = logical_indices.iter().map(|&i| i as usize).collect();
    indices.sort_unstable();
    indices.dedup();

    for &index in indices.iter().rev() {
        if index >= page_order.len() {
            return Err(AppError::PageOutOfRange { index: index as u32, page_count });
        }
        page_order.remove(index);
    }

    Ok(())
}

fn rotate_clockwise(current: PdfPageRenderRotation) -> PdfPageRenderRotation {
    match current {
        PdfPageRenderRotation::None => PdfPageRenderRotation::Degrees90,
        PdfPageRenderRotation::Degrees90 => PdfPageRenderRotation::Degrees180,
        PdfPageRenderRotation::Degrees180 => PdfPageRenderRotation::Degrees270,
        PdfPageRenderRotation::Degrees270 => PdfPageRenderRotation::None,
    }
}

fn rotate_counter_clockwise(current: PdfPageRenderRotation) -> PdfPageRenderRotation {
    match current {
        PdfPageRenderRotation::None => PdfPageRenderRotation::Degrees270,
        PdfPageRenderRotation::Degrees90 => PdfPageRenderRotation::None,
        PdfPageRenderRotation::Degrees180 => PdfPageRenderRotation::Degrees90,
        PdfPageRenderRotation::Degrees270 => PdfPageRenderRotation::Degrees180,
    }
}

/// Rotates one page a quarter turn. This *does* mutate the live document (via
/// PDFium's own `/Rotate` page attribute) rather than staying purely
/// in-memory-only like reorder/delete: PDFium already treats rotation as a
/// cheap, reversible page attribute, so there's no reason to shadow it with a
/// second layer of bookkeeping. Renders and thumbnails pick it up for free
/// since PDFium bakes `/Rotate` into every render of that page.
pub fn rotate(
    document: &PdfDocument,
    page_order: &[PdfPageIndex],
    logical_index: u32,
    clockwise: bool,
) -> Result<(), AppError> {
    let page_count = page_order.len() as u32;
    let physical = *page_order
        .get(logical_index as usize)
        .ok_or(AppError::PageOutOfRange { index: logical_index, page_count })?;

    let mut page = document
        .pages()
        .get(physical)
        .map_err(|e| AppError::RenderFailed(e.to_string()))?;

    let current = page.rotation().unwrap_or(PdfPageRenderRotation::None);
    let next = if clockwise { rotate_clockwise(current) } else { rotate_counter_clockwise(current) };
    page.set_rotation(next);

    Ok(())
}

/// Extracts the given logical pages (in the given order) to a new PDF file at
/// `dest_path`. Rotation set via `rotate` above is preserved automatically,
/// since it's already part of each source page's own state.
pub fn extract(
    pdfium: &'static Pdfium,
    document: &PdfDocument,
    page_order: &[PdfPageIndex],
    logical_indices: &[u32],
    dest_path: &Path,
) -> Result<(), AppError> {
    let page_count = page_order.len() as u32;

    let mut physical_indices = Vec::with_capacity(logical_indices.len());
    for &logical in logical_indices {
        let physical = *page_order
            .get(logical as usize)
            .ok_or(AppError::PageOutOfRange { index: logical, page_count })?;
        physical_indices.push(physical);
    }

    // PDFium's page-range string is 1-indexed; a plain comma list (no "-"
    // shorthand) preserves an arbitrary, possibly non-contiguous, possibly
    // reordered sequence exactly as given.
    let range = physical_indices
        .iter()
        .map(|physical| (physical + 1).to_string())
        .collect::<Vec<_>>()
        .join(",");

    let mut new_document = crate::pdf::doc::create_blank(pdfium)?;

    new_document
        .pages_mut()
        .copy_pages_from_document(document, &range, 0)
        .map_err(|e| AppError::ExtractFailed(e.to_string()))?;

    new_document
        .save_to_file(dest_path)
        .map_err(|e| AppError::ExtractFailed(e.to_string()))
}
