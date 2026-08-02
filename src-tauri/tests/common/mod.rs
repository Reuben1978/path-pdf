use std::sync::OnceLock;

use pdfium_render::prelude::Pdfium;

/// `Pdfium` holds a `Box<dyn PdfiumLibraryBindings>` with no `Sync` marker
/// (same situation as `state.rs`'s `SyncPdfium` in the app itself). Sound
/// here for the same reason: the `thread_safe` feature serializes actual FFI
/// calls internally, so sharing one instance across threads is exactly what
/// it's designed to support.
struct SyncPdfium(Pdfium);
unsafe impl Send for SyncPdfium {}
unsafe impl Sync for SyncPdfium {}

static PDFIUM: OnceLock<SyncPdfium> = OnceLock::new();

/// Loads PDFium from the same dev vendor path lib.rs uses, once per test
/// binary process. `cargo test` runs `#[test]` functions within a binary
/// concurrently by default, and PDFium's own library init isn't reentrant --
/// each test calling `Pdfium::new`/`bind_to_library` independently caused a
/// real deadlock the first time this suite was run. A shared `OnceLock`
/// (mirroring how lib.rs itself initializes exactly once at app startup)
/// fixes that: only the first caller does the actual init, everyone else
/// just gets the already-initialized instance.
pub fn load_pdfium() -> &'static Pdfium {
    &PDFIUM
        .get_or_init(|| {
            let lib_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("vendor")
                .join("pdfium")
                .join("x86_64-unknown-linux-gnu")
                .join("lib");
            let bindings =
                Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_dir))
                    .expect("failed to load pdfium library for tests");
            SyncPdfium(Pdfium::new(bindings))
        })
        .0
}

pub fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("tests").join("fixtures").join(name)
}
