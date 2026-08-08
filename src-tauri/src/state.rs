use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use pdfium_render::prelude::{Pdfium, PdfDocument, PdfPageIndex};

use crate::error::AppError;

/// One open document plus its logical page order (see `pdf::pages::reorder`
/// and `pdf::pages::delete` -- reordering/deletion live here, not in the
/// underlying PDFium document, until the user saves or extracts) and the
/// path it was opened from, which "Save" (as opposed to "Save As") writes
/// back to by default.
struct OpenDocument {
    document: PdfDocument<'static>,
    page_order: Vec<PdfPageIndex>,
    path: PathBuf,
}

/// `PdfDocument` (and `Pdfium` itself, see `SyncPdfium` below) hold raw FFI
/// pointers and aren't `Send`, but Tauri's managed `State` requires it. This
/// is sound here because pdfium-render's `thread_safe` feature (a default
/// feature) serializes every actual call into the underlying PDFium library
/// behind its own internal mutex, and every access to a stored document
/// additionally goes through this module's own `Mutex<HashMap<..>>` below
/// (see `with_document`) -- so a document is never touched concurrently even
/// though it can cross threads.
struct SendOpenDocument(OpenDocument);

unsafe impl Send for SendOpenDocument {}

struct SyncPdfium(&'static Pdfium);

unsafe impl Send for SyncPdfium {}
unsafe impl Sync for SyncPdfium {}

/// Open-document registry. `pdfium` is `&'static` because `PdfDocument` borrows
/// from the `Pdfium` instance that loaded it; a self-referential struct isn't
/// expressible in safe Rust, so the instance is deliberately leaked once at
/// startup (see `lib.rs::run`) rather than owned here.
pub struct AppState {
    pdfium: SyncPdfium,
    documents: Mutex<HashMap<u32, SendOpenDocument>>,
    next_id: AtomicU32,
    /// A file path passed on the command line at launch (the OS does this
    /// when the app is invoked via a file association, e.g. double-clicking
    /// a .pdf with this app set as the default handler). Taken exactly once
    /// by the frontend on startup -- see commands::doc::take_launch_file.
    initial_file: Mutex<Option<PathBuf>>,
}

impl AppState {
    pub fn new(pdfium: &'static Pdfium, initial_file: Option<PathBuf>) -> Self {
        Self {
            pdfium: SyncPdfium(pdfium),
            documents: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
            initial_file: Mutex::new(initial_file),
        }
    }

    pub fn pdfium(&self) -> &'static Pdfium {
        self.pdfium.0
    }

    pub fn take_initial_file(&self) -> Option<PathBuf> {
        self.initial_file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    pub fn insert(&self, document: PdfDocument<'static>, path: PathBuf) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let page_order: Vec<PdfPageIndex> = (0..document.pages().len()).collect();

        self.documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, SendOpenDocument(OpenDocument { document, page_order, path }));
        id
    }

    /// Drops the document (freeing its PDFium handle), called when a tab
    /// closes. Closing an id that's already gone (or never existed) is a
    /// no-op -- the frontend doesn't need to special-case double-closes.
    pub fn remove(&self, id: u32) {
        self.documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&id);
    }

    /// `f` gets mutable access to both the document and its logical page
    /// order, plus the path it was opened from. Most operations (render,
    /// rotate, reorder, delete) only need an immutable borrow at the Rust
    /// level -- PDFium mutates through the page's own FFI handle, not
    /// through `&mut PdfDocument` -- but a few (e.g. loading a standard-14
    /// font for a new annotation) genuinely need `&mut PdfDocument`, so this
    /// hands out `&mut` and lets callers reborrow as `&` where that's all
    /// they need.
    pub fn with_document<T>(
        &self,
        id: u32,
        f: impl FnOnce(&mut PdfDocument<'static>, &mut Vec<PdfPageIndex>, &PathBuf) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut documents = self
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let open_document = documents.get_mut(&id).ok_or(AppError::UnknownDocument(id))?;
        f(&mut open_document.0.document, &mut open_document.0.page_order, &open_document.0.path)
    }
}
