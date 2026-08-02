/** Shared UI state for the typewriter tool's on/off toggle, since the
 * toggle button (in the Signatures panel) and the click handler it enables
 * (in the per-page canvas) live in different components. */
class ToolState {
  textToolActive = $state(false);
}

export const toolState = new ToolState();
