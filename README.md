# Path PDF

A fast, lightweight PDF viewer and light editor. The goal is the 20% of Adobe
Acrobat that people actually use, in an app that opens instantly and installs
in seconds.

## Features

- View PDFs with fast, render-on-demand scrolling and zoom.
- Pages panel — thumbnails, reorder, rotate, delete, and extract pages to a
  new file.
- Typewriter tool — click anywhere on a page and type text onto it.
- Signature stamping — a saved library of signature/initial images, placed by
  drag and drop.
- Save / Save As, with an option to flatten annotations on export.

Editing is non-destructive: the typewriter tool and signature stamping add
annotations rather than modifying the page content, so they stay individually
movable and deletable until you choose to flatten them.

## Stack

Tauri v2 (Rust) + PDFium for rendering, TypeScript + Svelte 5 + Vite for the
UI. No Electron, no cloud sync, no telemetry.

## Building and running locally

Requires Node.js, Rust, and the platform prerequisites for
[Tauri](https://tauri.app/start/prerequisites/).

```bash
./scripts/fetch-pdfium.sh   # one-time: downloads the PDFium binary for your platform
npm install
npm run tauri dev           # dev build with hot reload
```

To build a release bundle (`.deb` / `.AppImage` on Linux, `.msi` on Windows):

```bash
npm run tauri build
```

## Testing

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run check
```

## CI / releases

`.github/workflows/build.yml` builds both platforms on every push to `master` and on `v*`
tags, and uploads the installers as downloadable workflow artifacts. Windows builds are
signed via [SignPath Foundation](https://signpath.org/) once `SIGNPATH_API_TOKEN` (secret)
and `SIGNPATH_ORGANIZATION_ID` / `SIGNPATH_PROJECT_SLUG` (repo variables) are configured —
until then the signing step is skipped and the plain unsigned installer is what you download.

## License

MIT — see [LICENSE](LICENSE).
