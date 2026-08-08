use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pdfium_render::prelude::Pdfium;
use tauri::{Emitter, Listener, Manager};

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

/// Dismisses the splash screen, once *both* the minimum splash duration has
/// elapsed (background thread, for branding) and the frontend has reported
/// that it has actually painted pixels ("frontend-painted", see
/// src/app.svelte). Whichever finishes second performs the dismissal;
/// `dismissed` guards against both doing it.
///
/// Note this only *closes the splash* -- it does not show main. Main is
/// already on screen by this point, deliberately: see the comment on the
/// splash's on_page_load in `run()` for why that ordering is what actually
/// fixes the startup flash.
fn try_dismiss_splash(
    splash: &Option<tauri::WebviewWindow>,
    min_duration_elapsed: &AtomicBool,
    frontend_painted: &AtomicBool,
    dismissed: &AtomicBool,
) {
    if !min_duration_elapsed.load(Ordering::SeqCst) || !frontend_painted.load(Ordering::SeqCst) {
        return;
    }
    if dismissed.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(splash) = splash {
        let _ = splash.close();
    }
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
            // The event loop doesn't start pumping/painting windows until this
            // closure returns, so nothing blocking (PDFium's dlopen, disk
            // access) can happen here directly -- it would delay the splash
            // window's very first paint. Do it on a background thread instead
            // and let setup() return immediately.
            //
            let min_duration_elapsed = Arc::new(AtomicBool::new(false));
            let frontend_painted = Arc::new(AtomicBool::new(false));
            let dismissed = Arc::new(AtomicBool::new(false));

            // Main is built first (so the splash, created after, stacks above
            // it) and starts hidden. It's shown a moment later, underneath
            // the splash -- see the splash's on_page_load below.
            let main_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                    .title("Path PDF")
                    .inner_size(1280.0, 800.0)
                    .resizable(true)
                    .fullscreen(false)
                    .maximized(true)
                    // --color-bg from src/app.css. Without this both the
                    // window and webview default to white, so any frame
                    // rendered before the webview composites its content
                    // flashes white against this app's dark UI. Keep in
                    // sync with app.css.
                    .background_color(tauri::window::Color(0x17, 0x15, 0x1d, 0xff))
                    .visible(false);
            // Windows-only builder method (WebView2-specific) -- disabling
            // Tauri's native drag-drop handling is required for the app's
            // own HTML5 drag-and-drop (signature placement) to work on
            // Windows; WebKitGTK on Linux never needed this.
            #[cfg(windows)]
            let main_builder = main_builder.drag_and_drop(false);

            let main = main_builder.build().ok();

            // The splash is built here (rather than declared in
            // tauri.conf.json) so on_page_load can be attached before the
            // webview navigates -- only available on the builder, not on an
            // already-created window.
            //
            // When the splash has painted, it shows itself AND shows main
            // underneath it. That ordering is the fix for the startup flash,
            // and it took three wrong attempts to get right, so: a webview
            // does not render while its window is hidden. So "wait until
            // main has painted, then show main" is circular -- it can never
            // paint, and every variation of that produced either a blank
            // window flash or (when gated on requestAnimationFrame) a
            // permanent deadlock. Main must be on screen to paint at all.
            // Putting it on screen *behind* the always-on-top splash lets it
            // paint during the splash's minimum display time, so by the time
            // the splash closes there is a fully-drawn window underneath and
            // nothing to flash. It also means the swap involves no show()
            // at all -- only the splash closing -- so there is no instant
            // where neither window covers the screen.
            let splash = {
                let main_for_splash_cb = main.clone();
                tauri::WebviewWindowBuilder::new(
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
                // Matches splashscreen.html's own black background, same
                // reason as main above: the default is white.
                .background_color(tauri::window::Color(0x00, 0x00, 0x00, 0xff))
                .visible(false)
                .on_page_load(move |window, payload| {
                    if payload.event() == tauri::webview::PageLoadEvent::Finished {
                        let _ = window.show();
                        if let Some(main) = &main_for_splash_cb {
                            let _ = main.show();
                            // Keep the splash above main -- main was just
                            // mapped, which can raise it above the splash
                            // depending on the window manager.
                            let _ = window.set_focus();
                        }
                    }
                })
                .build()
                .ok()
            };

            if let Some(main) = &main {
                let splash_for_cb = splash.clone();
                let min_duration_elapsed_for_cb = min_duration_elapsed.clone();
                let frontend_painted_for_cb = frontend_painted.clone();
                let dismissed_for_cb = dismissed.clone();
                main.once("frontend-painted", move |_event| {
                    frontend_painted_for_cb.store(true, Ordering::SeqCst);
                    try_dismiss_splash(
                        &splash_for_cb,
                        &min_duration_elapsed_for_cb,
                        &frontend_painted_for_cb,
                        &dismissed_for_cb,
                    );
                });
            }

            let app_handle = app.handle().clone();
            let splash_for_thread = splash.clone();

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
                // passes the file path as the first CLI argument.
                let args: Vec<String> = std::env::args().collect();
                let initial_file = file_arg_from(&args);

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

                min_duration_elapsed.store(true, Ordering::SeqCst);
                try_dismiss_splash(
                    &splash_for_thread,
                    &min_duration_elapsed,
                    &frontend_painted,
                    &dismissed,
                );
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
