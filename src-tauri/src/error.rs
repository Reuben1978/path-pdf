use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
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
