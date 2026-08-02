//! Opens every fixture in tests/fixtures/ and checks the app's PDF-handling
//! code does the right thing with each one -- including the "wrong" ones.
//! Per CLAUDE.md: every PDF is untrusted input, and a corrupt/encrypted/
//! truncated file must surface as an error, never a panic.

mod common;

use app_lib::pdf::doc;
use pdfium_render::prelude::PdfPageRenderRotation;

#[test]
fn opens_plain_text_document() {
    let pdfium = common::load_pdfium();
    let document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
    assert_eq!(doc::page_count(&document), 1);
}

#[test]
fn opens_500_page_document() {
    let pdfium = common::load_pdfium();
    let document = doc::open(pdfium, &common::fixture("500-page.pdf")).unwrap();
    assert_eq!(doc::page_count(&document), 500);
}

#[test]
fn opens_scanned_image_only_document() {
    let pdfium = common::load_pdfium();
    let document = doc::open(pdfium, &common::fixture("scanned-image-only.pdf")).unwrap();
    assert_eq!(doc::page_count(&document), 1);
}

#[test]
fn opens_rotated_pages_document_with_correct_rotations() {
    let pdfium = common::load_pdfium();
    let document = doc::open(pdfium, &common::fixture("rotated-pages.pdf")).unwrap();
    assert_eq!(doc::page_count(&document), 3);

    let expected = [
        PdfPageRenderRotation::Degrees90,
        PdfPageRenderRotation::Degrees180,
        PdfPageRenderRotation::Degrees270,
    ];
    for (index, expected_rotation) in expected.iter().enumerate() {
        let page = document.pages().get(index as u16).unwrap();
        assert_eq!(page.rotation().unwrap(), *expected_rotation);
    }
}

#[test]
fn password_protected_document_fails_gracefully_without_password() {
    let pdfium = common::load_pdfium();
    let result = doc::open(pdfium, &common::fixture("password-protected.pdf"));
    assert!(result.is_err(), "opening an encrypted PDF with no password must error, not panic or succeed");
}

#[test]
fn password_protected_document_opens_with_correct_password() {
    let pdfium = common::load_pdfium();
    let document = pdfium
        .load_pdf_from_file(&common::fixture("password-protected.pdf"), Some("user123"))
        .unwrap();
    assert_eq!(document.pages().len(), 1);
}

#[test]
fn truncated_document_fails_gracefully() {
    let pdfium = common::load_pdfium();
    let result = doc::open(pdfium, &common::fixture("truncated.pdf"));
    assert!(result.is_err(), "opening a truncated/corrupt PDF must error, not panic or succeed");
}
