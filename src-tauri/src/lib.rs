use std::path::PathBuf;
use std::time::Duration;

use pdfium_render::prelude::Pdfium;
use tauri::Manager;

mod commands;
mod state;

// Exposed (not just `mod`) so integration tests under tests/ can exercise
// the PDF logic directly against tests/fixtures/, per CLAUDE.md's testing
// section, without going through the Tauri command/IPC layer.
pub mod error;
pub mod pdf;

use state::AppState;

#[cfg(target_os = "windows")]
const PDFIUM_TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";

#[cfg(not(target_os = "windows"))]
const PDFIUM_TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";

/// Locates the PDFium dynamic library, preferring Tauri's bundled-resource
/// directory (see `tauri.linux.conf.json` / `tauri.windows.conf.json`,
/// which bundle PDFium under a `pdfium/` resource) and falling back to the
/// `vendor/pdfium/<triple>/` layout fetched by `scripts/fetch-pdfium.sh`,
/// resolved relative to the source tree at compile time.
///
/// The fallback matters even in release builds: `tauri build` only copies
/// `resources` into the packaged installers (.deb/.AppImage/.msi), not
/// loosely next to the raw binary in `target/release/`. Tauri's
/// `resource_dir()` still resolves to `target/release/` itself when run
/// from there (it detects the Cargo output directory), so a release binary
/// run directly out of a dev checkout -- as opposed to a real install --
/// would otherwise find no `pdfium/` resource at all. Probing for the
/// bundled file's actual existence, rather than trusting whichever path
/// resolved, covers both cases without needing a debug/release split.
fn pdfium_library_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vendor")
        .join("pdfium")
        .join(PDFIUM_TARGET_TRIPLE)
        .join(if cfg!(target_os = "windows") { "bin" } else { "lib" });

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let bundled_dir = resource_dir.join("pdfium");
        if Pdfium::pdfium_platform_library_name_at_path(&bundled_dir).exists() {
            return bundled_dir;
        }
    }

    dev_dir
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The event loop doesn't start pumping/painting windows until this
            // closure returns, so nothing blocking (PDFium's dlopen, disk
            // access) can happen here directly -- it would delay the splash
            // window's very first paint. Do it on a background thread instead
            // and let setup() return immediately.
            //
            // The splash window is built here (rather than declared in
            // tauri.conf.json) so on_page_load can be attached before the
            // webview navigates -- that's only available on the builder, not
            // on an already-created window. It starts hidden and is only
            // shown once its content has actually finished loading, so the
            // OS never displays an empty/white frame while the webview is
            // still painting its first frame.
            let splash = tauri::WebviewWindowBuilder::new(
                app,
                "splashscreen",
                tauri::WebviewUrl::App("splashscreen.html".into()),
            )
            .inner_size(720.0, 720.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .center()
            .skip_taskbar(true)
            .visible(false)
            .on_page_load(|window, payload| {
                if payload.event() == tauri::webview::PageLoadEvent::Finished {
                    let _ = window.show();
                }
            })
            .build()
            .ok();
            let main = app.get_webview_window("main");
            let app_handle = app.handle().clone();

            std::thread::spawn(move || {
                let start = std::time::Instant::now();

                let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(
                    &pdfium_library_dir(&app_handle),
                ))
                .expect("failed to load PDFium library");
                // Leaked deliberately: PdfDocument borrows from the Pdfium
                // instance that opened it, and this instance needs to outlive
                // every document opened for the lifetime of the process. See
                // state.rs.
                let pdfium: &'static Pdfium = Box::leak(Box::new(Pdfium::new(bindings)));

                // When launched via a file association (e.g. double-clicking a
                // .pdf with this app set as the default handler), the OS
                // passes the file path as the first CLI argument. `.exists()`
                // guards against treating some unrelated flag as a path.
                let initial_file = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.exists());

                app_handle.manage(AppState::new(pdfium, initial_file));

                // Splash stays up for a fixed minimum so the branding is
                // visible even though setup above usually finishes well
                // under that. WebviewWindow methods dispatch to the event
                // loop internally, so it's safe to call show()/close() from
                // this plain OS thread.
                let min_splash = Duration::from_millis(1500);
                if let Some(remaining) = min_splash.checked_sub(start.elapsed()) {
                    std::thread::sleep(remaining);
                }

                if let Some(splash) = splash {
                    let _ = splash.close();
                }
                if let Some(main) = main {
                    let _ = main.show();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::doc::take_launch_file,
            commands::doc::open_document,
            commands::doc::render_page,
            commands::doc::get_page_sizes,
            commands::pages::list_pages,
            commands::pages::reorder_pages,
            commands::pages::delete_pages,
            commands::pages::rotate_page,
            commands::pages::extract_pages,
            commands::annots::add_text_annotation,
            commands::annots::list_available_fonts,
            commands::annots::list_text_annotations,
            commands::annots::delete_text_annotation,
            commands::signatures::import_signature,
            commands::signatures::save_drawn_signature,
            commands::signatures::list_signatures,
            commands::signatures::delete_signature,
            commands::signatures::get_signature_bytes,
            commands::signatures::place_signature,
            commands::save::save_document,
            commands::save::save_document_as,
            commands::recents::list_recent_documents,
            commands::recents::list_recent_places,
            commands::recents::set_recent_document_pinned,
            commands::recents::set_recent_place_pinned,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
