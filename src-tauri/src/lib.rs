use std::path::PathBuf;

use pdfium_render::prelude::Pdfium;
use tauri::{Emitter, Manager};

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

/// Pulls the launched-with file path out of a process's argv (`argv[0]` is
/// the exe itself). Shared by this process's own startup and by
/// tauri-plugin-single-instance's callback, which relays a *second*
/// process's argv here instead of it ever reaching `main`. `.exists()`
/// guards against treating some unrelated flag as a path.
fn file_arg_from(argv: &[String]) -> Option<PathBuf> {
    argv.get(1).map(PathBuf::from).filter(|p| p.exists())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Not available on mobile (see the target-specific dependency in
    // Cargo.toml). Without this, double-clicking a second PDF while Path
    // PDF is already running would launch a whole separate process/window
    // instead of opening the file in the existing one -- this must be the
    // first plugin registered, per tauri-plugin-single-instance's docs.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(path) = file_arg_from(&argv) {
                let _ = app.emit("open-file", path.to_string_lossy().into_owned());
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The event loop doesn't start pumping/painting windows until
            // this closure returns, so nothing blocking (PDFium's dlopen,
            // disk access) can happen here directly -- it would delay the
            // window's very first paint. Do it on a background thread
            // instead and let setup() return immediately.
            //
            // There is deliberately only one window. The splash used to be
            // a separate always-on-top window, which is what caused the
            // startup flashing: with two windows there is always some
            // instant where neither fully covers the screen (the desktop
            // shows through), or where the main window is up but hasn't
            // painted its content yet (it visibly pops in). A webview does
            // not paint at all while its window is hidden, so no ordering
            // of show/close calls could avoid both. Now index.html carries
            // the splash as static markup, so this window's *first* painted
            // frame is already the branded splash, and the frontend removes
            // that overlay once the app underneath is ready (app.svelte).
            // Built hidden and revealed once its document has loaded, for
            // one specific reason: `inner_size` is the *restore* size, and
            // the window manager maps the window at that size and draws its
            // frame there before applying `maximized`. Mapped-and-visible
            // during that step, it drew a thin bright outline of the
            // un-maximized window over the desktop -- an L-shaped line, for
            // a frame or two, before anything else appeared. Staying hidden
            // until after that means the window is only ever seen already
            // maximized. This is *not* the "hide until painted" approach
            // that failed before: the first frame is the splash overlay in
            // index.html (static markup, no framework needed), and the
            // background colour below matches it, so there is nothing blank
            // to see even if the reveal lands a frame early.
            let main_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                    .title("Path PDF")
                    .inner_size(1280.0, 800.0)
                    .resizable(true)
                    .fullscreen(false)
                    .maximized(true)
                    .visible(false)
                    // App UI is dark (see --color-bg below), but window
                    // decorations otherwise default to the OS theme. On
                    // Windows that's light unless the system itself is in
                    // dark mode, which produces a white/light native
                    // titlebar sitting on top of the dark webview for the
                    // whole time the window is up -- suspected cause of the
                    // startup "bar at the top" flash reported on Windows
                    // (not reproduced on Linux, where the window manager's
                    // own theme happens to already be dark on the dev box).
                    // Forcing dark here removes the mismatch regardless of
                    // OS theme. Unverified against the actual Windows repro
                    // as of this commit -- see CLAUDE.md in the VM Share
                    // folder for the collaboration/verification thread.
                    .theme(Some(tauri::Theme::Dark))
                    // --color-bg from app.css, matching both the splash
                    // overlay in index.html and the app UI underneath it.
                    // Without this the window and webview default to white
                    // and flash before the webview composites. It also
                    // covers a quirk seen on this machine: the webview lays
                    // out narrower than the window (~1440px in a 1920px
                    // window), leaving a strip the webview never paints --
                    // matching this colour keeps that strip invisible.
                    .background_color(tauri::window::Color(0x17, 0x15, 0x1d, 0xff))
                    .on_page_load(|window, payload| {
                        if payload.event() == tauri::webview::PageLoadEvent::Finished {
                            let _ = window.show();
                        }
                    });
            // Windows-only builder method (WebView2-specific) -- disabling
            // Tauri's native drag-drop handling is required for the app's
            // own HTML5 drag-and-drop (signature placement) to work on
            // Windows; WebKitGTK on Linux never needed this.
            #[cfg(windows)]
            let main_builder = main_builder.drag_and_drop(false);

            let _main = main_builder.build().ok();

            let app_handle = app.handle().clone();

            std::thread::spawn(move || {

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
                // passes the file path as the first CLI argument.
                let args: Vec<String> = std::env::args().collect();
                let initial_file = file_arg_from(&args);

                // The splash overlay's minimum display time is handled in
                // the frontend now (see SPLASH_MS in app.svelte), so this
                // thread's only job is getting PDFium and AppState ready.
                app_handle.manage(AppState::new(pdfium, initial_file));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::doc::take_launch_file,
            commands::doc::open_document,
            commands::doc::close_document,
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
            commands::signatures::resize_signature_annotation,
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
