use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("no document open with id {0}")]
    UnknownDocument(u32),

    #[error("page index {index} out of range (document has {page_count} pages)")]
    PageOutOfRange { index: u32, page_count: u32 },

    #[error("new page order must be a permutation of the current pages")]
    InvalidPageOrder,

    #[error("failed to open PDF: {0}")]
    OpenFailed(String),

    #[error("failed to render page: {0}")]
    RenderFailed(String),

    #[error("failed to extract pages: {0}")]
    ExtractFailed(String),

    #[error("failed to add or remove annotation: {0}")]
    AnnotationFailed(String),

    #[error("failed to save document: {0}")]
    SaveFailed(String),

    #[error("failed to read or write recent files list: {0}")]
    RecentsFailed(String),
}

// Serialized as the plain Display-formatted message, not the derived
// {kind, message} shape -- Tauri sends this straight to the frontend as
// the rejected value of a failed invoke(), and nothing there does more
// than String(e) on it. A structured object shows up as "[object Object]"
// there; a plain string round-trips correctly with no frontend changes.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
