import { renderPage, type PageSize } from "../ipc";

// Print quality, not screen quality -- the viewer renders at whatever the
// canvas happens to be laid out at, but print output needs a fixed DPI
// independent of window size or zoom.
const PRINT_DPI = 150;

/** Rendered pages waiting to be printed, plus whether that render is still
 * in flight. Populated by printDocument() and consumed by PrintArea.svelte,
 * which is invisible on screen and only shown by @media print rules. */
class PrintState {
  preparing = $state(false);
  pageImages = $state<string[]>([]);
}

export const printState = new PrintState();

function pageToDataUrl(rgba: Uint8Array, width: number, height: number): string {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas context unavailable");
  ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);
  return canvas.toDataURL("image/png");
}

/**
 * Renders every page at print resolution, then hands off to the webview's
 * native print handling (window.print()) -- on Linux that's WebKitGTK's
 * print dialog, on Windows WebView2's, both of which go through the OS's
 * normal printer/queue selection. There is no Tauri print API; this is the
 * standard way a webview app reaches the OS print dialog.
 */
export async function printDocument(id: number, pageSizes: PageSize[]): Promise<void> {
  printState.preparing = true;
  try {
    const images: string[] = [];
    for (let pageIndex = 0; pageIndex < pageSizes.length; pageIndex++) {
      const targetWidth = Math.round((pageSizes[pageIndex].widthPoints / 72) * PRINT_DPI);
      const page = await renderPage(id, pageIndex, targetWidth);
      images.push(pageToDataUrl(page.rgba, page.width, page.height));
    }
    printState.pageImages = images;

    // Let the images actually paint into the print-only DOM before handing
    // off to window.print(), which snapshots layout synchronously.
    await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));

    const clear = () => {
      printState.pageImages = [];
      window.removeEventListener("afterprint", clear);
    };
    window.addEventListener("afterprint", clear);
    window.print();
  } finally {
    printState.preparing = false;
  }
}
