//! Per CLAUDE.md: "Annotation tests must round-trip: create the annotation,
//! save, reopen, assert it is present with the same geometry."

mod common;

use app_lib::pdf::{annots, doc, save};
use pdfium_render::prelude::{PdfPageAnnotationCommon, PdfPageObjectCommon, PdfPageObjectsCommon};

#[test]
fn text_annotation_round_trips_through_save_and_reload() {
    let pdfium = common::load_pdfium();
    let mut document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
    let page_order: Vec<u16> = (0..document.pages().len()).collect();

    let x = 60.0;
    let y = 500.0;
    let font_size = 16.0;
    annots::add_text_annotation(&mut document, 0, x, y, "Round-trip text", font_size, "helvetica").unwrap();

    // "Same geometry" (CLAUDE.md) means matching the object's own ink bounds
    // before and after the round trip -- not the nominal (x, y) placement
    // request. A text object's actual bounding box legitimately differs from
    // its baseline-origin anchor by a point or two, due to font metrics
    // (left-side bearing, baseline overshoot on rounded glyphs); that's
    // normal typography, not a placement bug, so it's the wrong thing to
    // assert equal to the request. What must hold is that save+reload
    // doesn't move or resize the object.
    let bounds_before_save = {
        let page = document.pages().get(0).unwrap();
        let stamp = page.annotations().get(0).unwrap();
        let object = stamp.objects().get(0).unwrap();
        object.bounds().unwrap()
    };

    let dest = std::env::temp_dir().join("pdfapp_test_text_annotation_round_trip.pdf");
    let rebuilt = save::apply_page_order_and_flatten(pdfium, &document, &page_order, false).unwrap();
    save::save_atomic(&rebuilt, &dest).unwrap();

    let reloaded = pdfium.load_pdf_from_file(&dest, None).unwrap();
    let annotations = annots::list_text_annotations(&reloaded, 0).unwrap();
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].contents, "Round-trip text");

    let page = reloaded.pages().get(0).unwrap();
    let stamp = page.annotations().get(0).unwrap();
    let object = stamp.objects().get(0).unwrap();
    let bounds_after_reload = object.bounds().unwrap();

    assert!((bounds_after_reload.left().value - bounds_before_save.left().value).abs() < 0.5);
    assert!((bounds_after_reload.bottom().value - bounds_before_save.bottom().value).abs() < 0.5);
    assert!((bounds_after_reload.right().value - bounds_before_save.right().value).abs() < 0.5);
    assert!((bounds_after_reload.top().value - bounds_before_save.top().value).abs() < 0.5);

    // Sanity-check the placement request landed in the right ballpark too
    // (generous tolerance, since this is checking against the nominal
    // anchor, not exact ink bounds).
    assert!((bounds_before_save.left().value - x).abs() < 5.0);
    assert!((bounds_before_save.bottom().value - y).abs() < 5.0);

    let _ = std::fs::remove_file(&dest);
}

#[test]
fn signature_annotation_round_trips_through_save_and_reload() {
    let pdfium = common::load_pdfium();
    let mut document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
    let page_order: Vec<u16> = (0..document.pages().len()).collect();

    let png_bytes = std::fs::read(common::fixture("test-signature.png")).unwrap();
    let (natural_w, natural_h) = annots::signature_dimensions(&png_bytes).unwrap();
    assert_eq!((natural_w, natural_h), (300, 100));

    let x = 80.0;
    let y = 60.0;
    let width = 150.0;
    let height = 50.0;
    annots::add_signature_annotation(&mut document, 0, x, y, width, height, &png_bytes).unwrap();

    let dest = std::env::temp_dir().join("pdfapp_test_signature_annotation_round_trip.pdf");
    let rebuilt = save::apply_page_order_and_flatten(pdfium, &document, &page_order, false).unwrap();
    save::save_atomic(&rebuilt, &dest).unwrap();

    let reloaded = pdfium.load_pdf_from_file(&dest, None).unwrap();
    let annotations = annots::list_text_annotations(&reloaded, 0).unwrap();
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].contents, "Signature");

    let page = reloaded.pages().get(0).unwrap();
    let stamp = page.annotations().get(0).unwrap();
    let object = stamp.objects().get(0).unwrap();
    let bounds = object.bounds().unwrap();
    assert!((bounds.left().value - x).abs() < 1.0);
    assert!((bounds.bottom().value - y).abs() < 1.0);
    assert!((bounds.right().value - (x + width)).abs() < 1.0);
    assert!((bounds.top().value - (y + height)).abs() < 1.0);

    let _ = std::fs::remove_file(&dest);
}

#[test]
fn flattening_removes_annotation_as_a_discrete_object_but_keeps_it_visible() {
    let pdfium = common::load_pdfium();
    let mut document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
    let page_order: Vec<u16> = (0..document.pages().len()).collect();

    annots::add_text_annotation(&mut document, 0, 60.0, 500.0, "Flatten me", 16.0, "helvetica").unwrap();

    let before_pixels = {
        let before_page = document.pages().get(0).unwrap();
        let before_bitmap = before_page
            .render_with_config(&pdfium_render::prelude::PdfRenderConfig::new().set_target_width(400))
            .unwrap();
        before_bitmap.as_image().to_rgba8().into_raw()
    };

    let dest = std::env::temp_dir().join("pdfapp_test_flatten_round_trip.pdf");
    let flattened = save::apply_page_order_and_flatten(pdfium, &document, &page_order, true).unwrap();
    save::save_atomic(&flattened, &dest).unwrap();

    let reloaded = pdfium.load_pdf_from_file(&dest, None).unwrap();
    let annotations = annots::list_text_annotations(&reloaded, 0).unwrap();
    assert_eq!(annotations.len(), 0, "flattened annotation should no longer be a discrete object");

    let after_page = reloaded.pages().get(0).unwrap();
    let after_bitmap = after_page
        .render_with_config(&pdfium_render::prelude::PdfRenderConfig::new().set_target_width(400))
        .unwrap();
    let after_pixels = after_bitmap.as_image().to_rgba8().into_raw();

    assert_ne!(
        before_pixels, after_pixels,
        "flattened text should still be visible even though it's no longer an annotation"
    );

    let _ = std::fs::remove_file(&dest);
}

#[test]
fn every_available_font_can_be_used() {
    let pdfium = common::load_pdfium();

    for &font_name in annots::AVAILABLE_FONTS {
        let mut document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
        annots::add_text_annotation(&mut document, 0, 60.0, 500.0, "Font check", 14.0, font_name)
            .unwrap_or_else(|e| panic!("font \"{font_name}\" failed: {e}"));

        let annotations = annots::list_text_annotations(&document, 0).unwrap();
        assert_eq!(annotations.len(), 1, "font \"{font_name}\" did not create an annotation");
    }
}

#[test]
fn unknown_font_name_errors_instead_of_panicking() {
    let pdfium = common::load_pdfium();
    let mut document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
    let result = annots::add_text_annotation(&mut document, 0, 60.0, 500.0, "x", 14.0, "comic-sans");
    assert!(result.is_err());
}
