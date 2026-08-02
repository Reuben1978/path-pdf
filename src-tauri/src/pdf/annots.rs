use pdfium_render::prelude::{
    PdfDocument, PdfFonts, PdfPageAnnotationCommon, PdfPageIndex, PdfPageObjectsCommon, PdfFontToken,
    PdfPoints, PdfRect,
};

use crate::error::AppError;

/// A small, curated set of PDFium's built-in "standard 14" fonts -- see
/// CLAUDE.md: embedding arbitrary system fonts is explicitly out of scope
/// for now ("Font embedding is the hard part... treat custom font embedding
/// as a later, separate task"), so the typewriter tool only ever offers
/// fonts PDFium can draw without embedding anything.
pub const AVAILABLE_FONTS: &[&str] = &["helvetica", "helvetica-bold", "times-roman", "times-bold", "courier"];

fn resolve_font(fonts: &mut PdfFonts, name: &str) -> Result<PdfFontToken, AppError> {
    match name {
        "helvetica" => Ok(fonts.helvetica()),
        "helvetica-bold" => Ok(fonts.helvetica_bold()),
        "times-roman" => Ok(fonts.times_roman()),
        "times-bold" => Ok(fonts.times_bold()),
        "courier" => Ok(fonts.courier()),
        other => Err(AppError::AnnotationFailed(format!("unknown font \"{other}\""))),
    }
}

/// Reads a PNG signature image's pixel dimensions, used by the frontend to
/// lock the placement rectangle's aspect ratio while dragging.
pub fn signature_dimensions(png_bytes: &[u8]) -> Result<(u32, u32), AppError> {
    let image = image::load_from_memory(png_bytes)
        .map_err(|e| AppError::AnnotationFailed(format!("invalid signature image: {e}")))?;
    Ok((image.width(), image.height()))
}

/// Places a signature image as a Stamp annotation, same rationale as the
/// typewriter tool for using Stamp rather than a more specific annotation
/// type. `png_bytes` should have a transparent background with straight
/// (non-premultiplied) alpha -- standard for PNGs exported from a canvas or
/// most image editors; PDFium derives a soft mask from that alpha channel
/// internally when the image object is created from a `DynamicImage`,
/// so no manual alpha stripping is needed on this side.
pub fn add_signature_annotation(
    document: &mut PdfDocument,
    physical_page_index: PdfPageIndex,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    png_bytes: &[u8],
) -> Result<(), AppError> {
    let image = image::load_from_memory(png_bytes)
        .map_err(|e| AppError::AnnotationFailed(format!("invalid signature image: {e}")))?;

    let mut page = document
        .pages()
        .get(physical_page_index)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    let mut stamp = page
        .annotations_mut()
        .create_stamp_annotation()
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    stamp
        .set_bounds(PdfRect::new(
            PdfPoints::new(y),
            PdfPoints::new(x),
            PdfPoints::new(y + height),
            PdfPoints::new(x + width),
        ))
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    stamp
        .objects_mut()
        .create_image_object(
            PdfPoints::new(x),
            PdfPoints::new(y),
            &image,
            Some(PdfPoints::new(width)),
            Some(PdfPoints::new(height)),
        )
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    stamp
        .set_contents("Signature")
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    Ok(())
}

pub struct TextAnnotationSummary {
    pub annotation_index: u32,
    pub contents: String,
}

/// Typewriter text is stored as a Stamp annotation containing a single text
/// object, not a FreeText annotation: PDFium's public FreeText appearance API
/// (in this crate version) offers no font-size/color control, while Stamp
/// annotations support adding arbitrary page objects with full control --
/// which is what CLAUDE.md's "font size, family, and color are user-
/// adjustable" requirement actually needs. It's still a real, non-destructive,
/// individually selectable/deletable annotation, just a different subtype.
///
/// `x`/`y` are in PDF page space (bottom-left origin, Y up) -- the frontend
/// converts from canvas pixel space at the ipc.ts boundary, per CLAUDE.md's
/// coordinate-system rule.
pub fn add_text_annotation(
    document: &mut PdfDocument,
    physical_page_index: PdfPageIndex,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    font_name: &str,
) -> Result<(), AppError> {
    let mut page = document
        .pages()
        .get(physical_page_index)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    let font = resolve_font(document.fonts_mut(), font_name)?;

    // Rough width estimate for the bounding box only; Stamp bounds don't clip
    // contained objects; the box just needs to comfortably contain the text.
    let width = (font_size * 0.6 * text.chars().count().max(1) as f32 + 20.0).max(40.0);
    let height = font_size * 1.8;

    let mut stamp = page
        .annotations_mut()
        .create_stamp_annotation()
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    stamp
        .set_bounds(PdfRect::new(
            PdfPoints::new(y),
            PdfPoints::new(x),
            PdfPoints::new(y + height),
            PdfPoints::new(x + width),
        ))
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    stamp
        .objects_mut()
        .create_text_object(PdfPoints::new(x), PdfPoints::new(y), text, font, PdfPoints::new(font_size))
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    // Populated so list_text_annotations (and other PDF viewers, for
    // accessibility) can identify/describe this annotation -- a Stamp
    // annotation's /Contents isn't otherwise set just by adding a text object.
    // (Deliberately not calling set_fill_color here -- PDFium segfaults on
    // FPDFAnnot_SetColor for Stamp annotations, which don't have a fill
    // concept in the first place since nothing paints a filled shape.)
    stamp
        .set_contents(text)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    Ok(())
}

pub fn list_text_annotations(
    document: &PdfDocument,
    physical_page_index: PdfPageIndex,
) -> Result<Vec<TextAnnotationSummary>, AppError> {
    let page = document
        .pages()
        .get(physical_page_index)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    Ok(page
        .annotations()
        .iter()
        .enumerate()
        .filter_map(|(index, annotation)| {
            annotation.contents().map(|contents| TextAnnotationSummary {
                annotation_index: index as u32,
                contents,
            })
        })
        .collect())
}

pub fn delete_text_annotation(
    document: &PdfDocument,
    physical_page_index: PdfPageIndex,
    annotation_index: u32,
) -> Result<(), AppError> {
    let mut page = document
        .pages()
        .get(physical_page_index)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    let annotation = page
        .annotations()
        .get(annotation_index as usize)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))?;

    page.annotations_mut()
        .delete_annotation(annotation)
        .map_err(|e| AppError::AnnotationFailed(e.to_string()))
}
