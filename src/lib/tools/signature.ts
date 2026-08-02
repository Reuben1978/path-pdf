export const SIGNATURE_DRAG_MIME = "application/x-pdfapp-signature";

/**
 * Converts a point in canvas pixel space (top-left origin, Y down) into PDF
 * page space (bottom-left origin, Y up). This is the one boundary where that
 * conversion happens, per CLAUDE.md's coordinate-system rule -- mirrors
 * tools/typewriter.ts's identical conversion for the typewriter tool.
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
