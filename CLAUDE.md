# CLAUDE.md

Guidance for Claude Code working in this repository.

## Project

A fast, lightweight, open-source PDF viewer and light editor. The goal is the 20% of
Adobe Acrobat that people actually use, in an app that opens instantly and installs
in seconds.

**v0.1 scope — build only these:**

1. Open and view PDFs, fast.
2. Pages panel — thumbnails, reorder, rotate, delete, extract.
3. Typewriter tool — click anywhere, type text onto the page.
4. Signature stamping — a saved library of signature/initial images, placed by drag.
5. Save / Save As, with the option to flatten annotations on export.

**Explicit non-goals for v0.1.** Do not build these, and do not add scaffolding
"in preparation" for them: OCR, form filling, redaction, digital/cryptographic
signatures, PDF/A conversion, cloud sync, accounts, telemetry, or a plugin system.
Scope creep is the main risk to this project.

## Platforms

Linux and Windows are both first-class from day one. macOS is untested and unsupported
for now — don't add Mac-specific code paths, but don't deliberately break portability
either.

- Development happens on Linux Mint (Ubuntu/Debian base, apt).
- Windows builds come from `.github/workflows/build.yml` (GitHub Actions), tested on a
  separate physical/VM Windows machine, not the dev box. Assume the developer cannot debug
  Windows interactively without friction, so be conservative with platform-specific code.
- Anything touching file paths, line endings, or the filesystem needs to work on both.
  Never hardcode `/` separators; use `std::path::PathBuf`.

## Stack

| Layer | Choice |
|---|---|
| Shell | Tauri v2 |
| Core | Rust |
| PDF engine | PDFium via the `pdfium-render` crate |
| UI | TypeScript + Svelte 5 + Vite |
| Styling | Plain CSS with custom properties. No Tailwind, no component library. |

PDFium binaries come from `bblanchon/pdfium-binaries`, fetched by `scripts/fetch-pdfium.sh`
into `vendor/pdfium/<target>/` (gitignored, not committed). Release builds bundle the
platform's PDFium library as a Tauri resource (`tauri.linux.conf.json` /
`tauri.windows.conf.json`) and resolve it at runtime via `resource_dir()`, falling back to
the `vendor/` dev path if the bundled resource isn't found — see the doc comment on
`pdfium_library_dir()` in `src-tauri/src/lib.rs`. CI must run `fetch-pdfium.sh` before
`tauri build`, since `vendor/` isn't in git.

Do not add a dependency without asking. Every crate and npm package is a size and
maintenance cost, and "lightweight" is a product requirement, not a preference.

## Layout

```
src-tauri/
  src/
    main.rs          # Tauri entry, command registration
    commands/        # IPC command handlers — thin, no logic
    pdf/
      doc.rs         # Document open/save lifecycle
      render.rs      # Page -> RGBA bitmap
      annots.rs      # FreeText (typewriter) + Stamp (signature) annotations
      pages.rs       # Reorder, rotate, delete, extract
    state.rs         # App state, open document registry
src/
  lib/
    viewer/          # Canvas, scroll, zoom
    panel/           # Pages panel
    tools/           # Typewriter, signature placement
    ipc.ts           # Typed wrappers over invoke()
  app.svelte
vendor/pdfium/       # gitignored
```

Keep PDF logic in `src-tauri/src/pdf/`. Command handlers in `commands/` should
validate input, call into `pdf/`, and translate errors. If a handler is longer than
about 30 lines, the logic belongs in `pdf/`.

## Commands

```bash
./scripts/fetch-pdfium.sh   # one-time: download PDFium for the host target
npm install
npm run tauri dev           # dev build with hot reload
npm run tauri build         # release bundle (.deb + .AppImage on Linux, .msi on Windows)
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run check               # svelte-check + tsc
```

Run `cargo clippy` and `npm run check` before declaring work finished. Both must be clean.

## Architecture notes

**Rendering path.** Rust rasterizes a page to an RGBA buffer with PDFium and returns
raw bytes to the frontend via `tauri::ipc::Response`, which the UI blits to a canvas.
Do **not** base64-encode bitmaps or pass them as JSON — that is the single easiest way
to make this app slow.

**Render on demand.** Never render the whole document up front. Render the visible
page plus one page either side. Thumbnails render at low resolution in a background
task and are cached to disk in the app cache dir, keyed by file hash plus page index.

**Coordinate systems.** PDF user space has its origin at the bottom-left with Y
increasing upward. Canvas has its origin at the top-left with Y increasing downward.
Convert at exactly one boundary — in `ipc.ts` — and keep everything on the Rust side
in PDF space. Mixed-up coordinates will be the most common bug in this codebase, so
when a stamp or text box lands in the wrong place, check the conversion first.
Page rotation and non-zero MediaBox origins both affect this.

**Editing is non-destructive.** The typewriter tool creates FreeText annotations and
signature stamping creates Stamp annotations. Neither modifies the page content
stream. This means edits stay individually selectable, movable, and deletable, and
the original document is recoverable. Flattening happens only on export, only when
the user asks for it.

**Saving.** Use incremental save when the file is unchanged structurally (annotations
only) — it's faster and preserves the original bytes. Full rewrite when pages have
been reordered or removed. Never write over the user's file in place without an
atomic temp-file-then-rename.

## Feature specifics

**Typewriter tool.** A click places an insertion point and typing creates a FreeText
annotation with a transparent background and no border. Font size, family, and color
are user-adjustable and persist between sessions. Text reflows within a box the user
can resize. Font embedding is the hard part here: PDFium's standard 14 fonts are safe
everywhere, so default to Helvetica and treat custom font embedding as a later,
separate task.

**Pages panel.** A left-side panel of thumbnails with multi-select, drag to reorder,
and a context menu for rotate / delete / extract to new file. Virtualize the list —
a 1000-page document must not create 1000 DOM nodes. Reordering mutates an in-memory
page order that is applied on save, not on each drag.

**Signature stamping.** Signatures live in the app data dir as PNGs with alpha. The
user imports a file or draws one on a canvas. Placing a signature creates a Stamp
annotation containing the image, scaled to the drag rectangle with the aspect ratio
locked. Strip alpha carefully — PDFium wants a soft mask, not premultiplied RGBA.

## Conventions

**Rust.** No `unwrap()` or `expect()` outside tests and `main.rs`. Errors use
`thiserror` in `pdf/`, converted to a serializable `AppError` at the command boundary.
Never let a malformed PDF panic the process — a corrupt file should surface a friendly
message, not take down the window.

**TypeScript.** `strict` mode on. No `any`. All IPC calls go through typed wrappers
in `ipc.ts`; components never call `invoke()` directly.

**Untrusted input.** Every PDF is untrusted input. Assume files are malformed,
truncated, encrypted, or hostile. Validate page indices before passing them to
PDFium — out-of-range indices are undefined behavior in some of its APIs.

**Commits.** Conventional commits (`feat:`, `fix:`, `perf:`, `refactor:`). One logical
change each.

## Testing

Fixture PDFs live in `tests/fixtures/`: a plain text document, a 500-page document, a
scanned image-only file, a rotated-pages file, a password-protected file, and a
deliberately truncated file. Every PDF-handling change gets a test against the
relevant fixtures.

Annotation tests must round-trip: create the annotation, save, reopen, assert it is
present with the same geometry.

## Performance budget

These are requirements, not aspirations. Regressions here are bugs.

- Cold start to window visible: under 500ms.
- First page painted for a 50-page document: under 200ms.
- Scrolling: 60fps, no blank frames on already-rendered pages.
- Release binary: under 20MB excluding PDFium.
- Idle memory with a 100-page document open: under 300MB.

## Known issues

**Webview lays out narrower than the window (Linux dev box).** On this dev machine the
webview's content area measures ~1440px wide inside a 1920px window — roughly a 480px
strip on the right that never paints. It's currently invisible because the window and
webview background colors are made to match (see `background_color()` in `lib.rs`), but
the app is not actually using the full window width. Also caused CSS viewport units
(`vw`, `vh`, `vmin`) and `position: fixed; inset: 0` to measure wrong during the splash
overlay work (see git history around the splash-overlay commits) — normal-flow percentage
sizing (`width: 100%` with `height: 100%` ancestors) was the only approach that filled the
window correctly. Not yet root-caused; worth investigating on its own rather than routing
around it again. Check whether it reproduces on Windows before assuming it's Linux/WebKitGTK-specific.

## License

MIT. Keep it that way — check any new dependency's license before adding it, and flag
anything copyleft rather than pulling it in. PDFium is BSD-3-Clause and compatible.
Its license file must ship in the bundle.
