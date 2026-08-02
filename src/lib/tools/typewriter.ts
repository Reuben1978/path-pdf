import { addTextAnnotation, deleteTextAnnotation, listTextAnnotations, type TextAnnotationInfo } from "../ipc";

export const DEFAULT_FONT_SIZE = 12;

/**
 * Converts a click position in canvas pixel space (top-left origin, Y down)
 * into PDF page space (bottom-left origin, Y up). This is the one boundary
 * where that conversion happens, per CLAUDE.md's coordinate-system rule --
 * everything past this point (ipc.ts, the Rust side) stays in PDF space.
 */
export function pixelToPdfPoint(
  pixelX: number,
  pixelY: number,
  canvasWidthPx: number,
  canvasHeightPx: number,
  pageWidthPoints: number,
  pageHeightPoints: number,
): { x: number; y: number } {
  const x = (pixelX / canvasWidthPx) * pageWidthPoints;
  const y = pageHeightPoints - (pixelY / canvasHeightPx) * pageHeightPoints;
  return { x, y };
}

export async function placeText(
  id: number,
  pageIndex: number,
  pixelX: number,
  pixelY: number,
  canvasWidthPx: number,
  canvasHeightPx: number,
  pageWidthPoints: number,
  pageHeightPoints: number,
  text: string,
  fontSize: number = DEFAULT_FONT_SIZE,
  fontName: string = "helvetica",
): Promise<void> {
  const { x, y } = pixelToPdfPoint(pixelX, pixelY, canvasWidthPx, canvasHeightPx, pageWidthPoints, pageHeightPoints);
  // The click point becomes the text's top-left in the UI, but PDFium places
  // text objects with (x, y) as the baseline's left edge -- shift down by
  // roughly the font's ascent so the typed text appears under the cursor
  // rather than floating above the click point.
  await addTextAnnotation(id, pageIndex, x, y - fontSize, text, fontSize, fontName);
}

export async function listAnnotations(id: number, pageIndex: number): Promise<TextAnnotationInfo[]> {
  return listTextAnnotations(id, pageIndex);
}

export async function removeAnnotation(id: number, pageIndex: number, annotationIndex: number): Promise<void> {
  return deleteTextAnnotation(id, pageIndex, annotationIndex);
}
