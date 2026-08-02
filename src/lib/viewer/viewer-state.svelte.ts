const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 0.1;

/** Shared zoom level for the continuous-scroll viewer. Applied via the CSS
 * `zoom` property (not `transform: scale`), because `zoom` -- unlike
 * `transform` -- affects layout and scroll area sizing correctly, so pages
 * scaled up still scroll to their full extent instead of clipping. WebKitGTK
 * (what Tauri uses on Linux) supports it. */
class ViewerState {
  zoom = $state(1);

  zoomIn() {
    this.zoom = Math.min(MAX_ZOOM, round(this.zoom + ZOOM_STEP));
  }

  zoomOut() {
    this.zoom = Math.max(MIN_ZOOM, round(this.zoom - ZOOM_STEP));
  }

  zoomBy(delta: number) {
    this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, round(this.zoom + delta)));
  }

  reset() {
    this.zoom = 1;
  }
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

export const viewerState = new ViewerState();
