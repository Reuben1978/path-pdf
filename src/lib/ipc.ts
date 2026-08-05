import { invoke } from "@tauri-apps/api/core";

export interface DocumentInfo {
  id: number;
  pageCount: number;
  path: string;
}

export interface RenderedPage {
  width: number;
  height: number;
  /** Page dimensions in PDF points (bottom-left origin, Y up). */
  pageWidthPoints: number;
  pageHeightPoints: number;
  rgba: Uint8Array;
}

export interface PageSummary {
  logicalIndex: number;
  rotationDegrees: number;
}

export interface PageSize {
  widthPoints: number;
  heightPoints: number;
}

export interface TextAnnotationInfo {
  annotationIndex: number;
  contents: string;
}

export interface SignatureInfo {
  filename: string;
  width: number;
  height: number;
}

export async function openDocument(path: string): Promise<DocumentInfo> {
  return invoke<DocumentInfo>("open_document", { path });
}

/**
 * Wire format: u32 width, u32 height, f32 pageWidthPoints, f32
 * pageHeightPoints (all little-endian), then raw RGBA8 pixel bytes. Avoids
 * extra IPC round trips for the bitmap's own dimensions and for the
 * pixel<->PDF-point conversion factor the typewriter tool needs to place
 * text where the user clicked -- see src-tauri/src/commands/doc.rs.
 */
export async function renderPage(
  id: number,
  pageIndex: number,
  targetWidth: number,
): Promise<RenderedPage> {
  const buffer = await invoke<ArrayBuffer>("render_page", {
    id,
    pageIndex,
    targetWidth,
  });
  const view = new DataView(buffer);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  const pageWidthPoints = view.getFloat32(8, true);
  const pageHeightPoints = view.getFloat32(12, true);
  const rgba = new Uint8Array(buffer, 16);

  return { width, height, pageWidthPoints, pageHeightPoints, rgba };
}

export async function listPages(id: number): Promise<PageSummary[]> {
  return invoke<PageSummary[]>("list_pages", { id });
}

export async function reorderPages(id: number, newOrder: number[]): Promise<void> {
  return invoke("reorder_pages", { id, newOrder });
}

export async function deletePages(id: number, logicalIndices: number[]): Promise<void> {
  return invoke("delete_pages", { id, logicalIndices });
}

export async function rotatePage(id: number, logicalIndex: number, clockwise: boolean): Promise<void> {
  return invoke("rotate_page", { id, logicalIndex, clockwise });
}

export async function extractPages(
  id: number,
  logicalIndices: number[],
  destPath: string,
): Promise<void> {
  return invoke("extract_pages", { id, logicalIndices, destPath });
}

/**
 * `x`/`y` must already be in PDF page space (bottom-left origin, Y up) --
 * convert from canvas pixel space (top-left origin, Y down) before calling
 * this. See lib/tools/typewriter.ts, the one place that conversion happens.
 */
export async function addTextAnnotation(
  id: number,
  pageIndex: number,
  x: number,
  y: number,
  text: string,
  fontSize: number,
  fontName: string,
): Promise<void> {
  return invoke("add_text_annotation", { id, pageIndex, x, y, text, fontSize, fontName });
}

/** The standard-14 font names the typewriter tool can use -- see
 * pdf/annots.rs's AVAILABLE_FONTS for why arbitrary fonts aren't offered. */
export async function listAvailableFonts(): Promise<string[]> {
  return invoke<string[]>("list_available_fonts");
}

export async function listTextAnnotations(id: number, pageIndex: number): Promise<TextAnnotationInfo[]> {
  return invoke<TextAnnotationInfo[]>("list_text_annotations", { id, pageIndex });
}

export async function deleteTextAnnotation(
  id: number,
  pageIndex: number,
  annotationIndex: number,
): Promise<void> {
  return invoke("delete_text_annotation", { id, pageIndex, annotationIndex });
}

export async function importSignature(sourcePath: string): Promise<string> {
  return invoke<string>("import_signature", { sourcePath });
}

export async function saveDrawnSignature(pngBytes: Uint8Array): Promise<string> {
  return invoke<string>("save_drawn_signature", { pngBytes: Array.from(pngBytes) });
}

export async function listSignatures(): Promise<SignatureInfo[]> {
  return invoke<SignatureInfo[]>("list_signatures");
}

export async function deleteSignature(filename: string): Promise<void> {
  return invoke("delete_signature", { filename });
}

export async function getSignatureBytes(filename: string): Promise<Uint8Array> {
  const buffer = await invoke<ArrayBuffer>("get_signature_bytes", { filename });
  return new Uint8Array(buffer);
}

/**
 * `x`/`y`/`width`/`height` must already be in PDF page space -- see
 * lib/tools/signature.ts for the drag-rectangle-to-PDF-points conversion.
 * Returns the placed annotation's index, e.g. for a follow-up
 * `resizeSignatureAnnotation` call.
 */
export async function placeSignature(
  id: number,
  pageIndex: number,
  filename: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<number> {
  return invoke("place_signature", { id, pageIndex, filename, x, y, width, height });
}

/**
 * Resizes/repositions an already-placed signature. Internally this deletes
 * and recreates the annotation (see the Rust-side doc comment on
 * `resize_signature_annotation` for why), so the annotation's index can
 * change -- always use the returned index for any further resize of the
 * same signature.
 */
export async function resizeSignatureAnnotation(
  id: number,
  pageIndex: number,
  annotationIndex: number,
  filename: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<number> {
  return invoke("resize_signature_annotation", {
    id,
    pageIndex,
    annotationIndex,
    filename,
    x,
    y,
    width,
    height,
  });
}

export async function saveDocument(id: number, flatten: boolean): Promise<void> {
  return invoke("save_document", { id, flatten });
}

export async function saveDocumentAs(id: number, destPath: string, flatten: boolean): Promise<void> {
  return invoke("save_document_as", { id, destPath, flatten });
}

export async function getPageSizes(id: number): Promise<PageSize[]> {
  return invoke<PageSize[]>("get_page_sizes", { id });
}

/**
 * Checks whether the OS launched this process with a file to open (e.g. the
 * app was set as the default PDF handler and the user double-clicked a
 * file). Returns null on a normal launch, or if already consumed once.
 */
export async function takeLaunchFile(): Promise<string | null> {
  return invoke<string | null>("take_launch_file");
}

export interface RecentEntry {
  path: string;
  lastUsed: number;
  pinned: boolean;
}

export async function listRecentDocuments(): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("list_recent_documents");
}

export async function listRecentPlaces(): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("list_recent_places");
}

export async function setRecentDocumentPinned(path: string, pinned: boolean): Promise<void> {
  return invoke("set_recent_document_pinned", { path, pinned });
}

export async function setRecentPlacePinned(path: string, pinned: boolean): Promise<void> {
  return invoke("set_recent_place_pinned", { path, pinned });
}
