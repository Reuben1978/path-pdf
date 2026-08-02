//! Reorder/delete/rotate/extract against a real multi-page fixture.

mod common;

use app_lib::pdf::{doc, pages};

#[test]
fn reorder_delete_rotate_extract_round_trip() {
    let pdfium = common::load_pdfium();
    let document = doc::open(pdfium, &common::fixture("rotated-pages.pdf")).unwrap();
    assert_eq!(document.pages().len(), 3);

    let mut page_order: Vec<u16> = (0..document.pages().len()).collect();

    pages::reorder(&mut page_order, &[2, 0, 1]).unwrap();
    assert_eq!(page_order, vec![2, 0, 1]);

    pages::delete(&mut page_order, &[0]).unwrap();
    assert_eq!(page_order, vec![0, 1]);

    pages::rotate(&document, &page_order, 0, true).unwrap();

    let dest = std::env::temp_dir().join("pdfapp_test_extract.pdf");
    pages::extract(pdfium, &document, &page_order, &[0, 1], &dest).unwrap();

    let extracted = pdfium.load_pdf_from_file(&dest, None).unwrap();
    assert_eq!(extracted.pages().len(), 2);

    let _ = std::fs::remove_file(&dest);
}

#[test]
fn out_of_range_page_operations_error_instead_of_panicking() {
    let pdfium = common::load_pdfium();
    let document = doc::open(pdfium, &common::fixture("plain-text.pdf")).unwrap();
    let mut page_order: Vec<u16> = (0..document.pages().len()).collect();

    assert!(pages::delete(&mut page_order, &[99]).is_err());
    assert!(pages::rotate(&document, &page_order, 99, true).is_err());
}
