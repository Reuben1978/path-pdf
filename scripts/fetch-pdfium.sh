#!/usr/bin/env bash
# One-time setup: download prebuilt PDFium binaries from bblanchon/pdfium-binaries
# into vendor/pdfium/<target-triple>/. Not committed (see .gitignore).
#
# Usage:
#   ./scripts/fetch-pdfium.sh                          # fetch for the host target
#   ./scripts/fetch-pdfium.sh <rust-target-triple>      # fetch for a specific target
#   ./scripts/fetch-pdfium.sh --force [<target-triple>] # re-fetch even if present
#
# At runtime, the Rust side loads the library dynamically from this directory via
# pdfium-render's Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(...)),
# pointed at vendor/pdfium/<triple>/lib (Linux) or vendor/pdfium/<triple>/bin (Windows).
# No compile-time linker flags are required for this approach.

set -euo pipefail

# Pinned release -- bump this deliberately when a newer PDFium build is wanted,
# rather than always tracking "latest" (reproducible builds).
PDFIUM_RELEASE_TAG="chromium/7961"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR_DIR="${ROOT_DIR}/vendor/pdfium"

FORCE=0
TARGET_TRIPLE=""

for arg in "$@"; do
  case "$arg" in
    --force)
      FORCE=1
      ;;
    -h|--help)
      sed -n '2,13p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *)
      if [[ -n "$TARGET_TRIPLE" ]]; then
        echo "error: unexpected extra argument: $arg" >&2
        exit 1
      fi
      TARGET_TRIPLE="$arg"
      ;;
  esac
done

if [[ -z "$TARGET_TRIPLE" ]]; then
  if ! command -v rustc >/dev/null 2>&1; then
    echo "error: rustc not found; pass a target triple explicitly, e.g.:" >&2
    echo "  ./scripts/fetch-pdfium.sh x86_64-unknown-linux-gnu" >&2
    exit 1
  fi
  TARGET_TRIPLE="$(rustc -vV | awk '/^host:/ { print $2 }')"
fi

case "$TARGET_TRIPLE" in
  x86_64-unknown-linux-gnu)
    ASSET="pdfium-linux-x64"
    LIB_CHECK="lib/libpdfium.so"
    ;;
  x86_64-pc-windows-msvc)
    ASSET="pdfium-win-x64"
    LIB_CHECK="bin/pdfium.dll"
    ;;
  *)
    echo "error: unsupported target triple: $TARGET_TRIPLE" >&2
    echo "supported: x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc" >&2
    exit 1
    ;;
esac

DEST_DIR="${VENDOR_DIR}/${TARGET_TRIPLE}"

if [[ -f "${DEST_DIR}/${LIB_CHECK}" && "$FORCE" -eq 0 ]]; then
  echo "PDFium already present for ${TARGET_TRIPLE} at ${DEST_DIR} (use --force to re-fetch)"
  exit 0
fi

for tool in curl tar; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: required tool not found: $tool" >&2
    exit 1
  fi
done

URL="https://github.com/bblanchon/pdfium-binaries/releases/download/${PDFIUM_RELEASE_TAG}/${ASSET}.tgz"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

TMP_ARCHIVE="${TMP_DIR}/${ASSET}.tgz"

echo "Fetching PDFium (${PDFIUM_RELEASE_TAG}, ${ASSET}) for ${TARGET_TRIPLE}..."
if ! curl -fL --progress-bar -o "$TMP_ARCHIVE" "$URL"; then
  echo "error: failed to download $URL" >&2
  exit 1
fi

rm -rf "$DEST_DIR"
mkdir -p "$DEST_DIR"

if ! tar -xzf "$TMP_ARCHIVE" -C "$DEST_DIR"; then
  echo "error: failed to extract $TMP_ARCHIVE" >&2
  exit 1
fi

if [[ ! -f "${DEST_DIR}/${LIB_CHECK}" ]]; then
  echo "error: extraction succeeded but expected file missing: ${DEST_DIR}/${LIB_CHECK}" >&2
  exit 1
fi

echo "PDFium ready at ${DEST_DIR}"
